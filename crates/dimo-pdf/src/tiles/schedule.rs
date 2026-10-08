//! The tile request queue: coalescing, ordering and cancellation. No IO, no threads.

use std::collections::{HashMap, VecDeque};

use crate::hash::ContentHash;

use super::{TileKey, TileRange};

/// Queue of tile jobs. Every job has one or more waiters `W` (callbacks in the service) that
/// receive the result.
///
/// - Requests for a tile that is already queued or being produced join that job.
/// - Jobs are taken in request order (FIFO).
/// - The interest of a document (see [`super::TileService::set_interest`]) cancels queued
///   jobs outside it, both when it is set and when a job is taken from the queue. Jobs being
///   produced are not cancelled; their result still goes into the cache.
#[derive(Debug)]
pub(crate) struct Scheduler<W> {
    order: VecDeque<TileKey>,
    jobs: HashMap<TileKey, Job<W>>,
    interest: HashMap<ContentHash, Vec<TileRange>>,
}

#[derive(Debug)]
struct Job<W> {
    waiters: Vec<W>,
    running: bool,
}

/// What [`Scheduler::next`] hands to a worker.
#[derive(Debug)]
pub(crate) enum Next<W> {
    /// Produce this tile, then call [`Scheduler::finish`].
    Run(TileKey),
    /// The job left the interest; answer these waiters with "cancelled".
    Cancelled(Vec<W>),
    /// Nothing queued.
    Idle,
}

impl<W> Default for Scheduler<W> {
    fn default() -> Self {
        Self {
            order: VecDeque::new(),
            jobs: HashMap::new(),
            interest: HashMap::new(),
        }
    }
}

impl<W> Scheduler<W> {
    /// Adds a waiter for `key`. Returns `true` when this created a new queued job.
    pub(crate) fn push(&mut self, key: TileKey, waiter: W) -> bool {
        if let Some(job) = self.jobs.get_mut(&key) {
            job.waiters.push(waiter);
            return false;
        }
        self.jobs.insert(
            key,
            Job {
                waiters: vec![waiter],
                running: false,
            },
        );
        self.order.push_back(key);
        true
    }

    /// Takes the oldest queued job. Jobs outside their document's interest come back as
    /// [`Next::Cancelled`].
    pub(crate) fn next(&mut self) -> Next<W> {
        let Some(key) = self.order.pop_front() else {
            return Next::Idle;
        };
        if !self.wanted(&key) {
            let waiters = self
                .jobs
                .remove(&key)
                .map(|j| j.waiters)
                .unwrap_or_default();
            return Next::Cancelled(waiters);
        }
        if let Some(job) = self.jobs.get_mut(&key) {
            job.running = true;
        }
        Next::Run(key)
    }

    /// Ends a running job and returns its waiters, including those that joined while it ran.
    pub(crate) fn finish(&mut self, key: &TileKey) -> Vec<W> {
        self.jobs.remove(key).map(|j| j.waiters).unwrap_or_default()
    }

    /// Sets the tiles a document still wants and returns the waiters of queued jobs outside
    /// them, which are removed. `None` lifts the restriction; an empty list cancels every
    /// queued job of the document.
    pub(crate) fn set_interest(
        &mut self,
        doc: ContentHash,
        ranges: Option<Vec<TileRange>>,
    ) -> Vec<W> {
        let Some(ranges) = ranges else {
            self.interest.remove(&doc);
            return Vec::new();
        };
        self.interest.insert(doc, ranges);
        let mut cancelled = Vec::new();
        let jobs = &mut self.jobs;
        let interest = &self.interest;
        self.order.retain(|key| {
            if key.doc != doc || wanted(interest, key) {
                return true;
            }
            if let Some(job) = jobs.remove(key) {
                cancelled.extend(job.waiters);
            }
            false
        });
        cancelled
    }

    /// Removes every queued job of a document (closed document) and returns their waiters.
    /// Running jobs finish normally.
    pub(crate) fn remove_document(&mut self, doc: &ContentHash) -> Vec<W> {
        self.interest.remove(doc);
        let mut removed = Vec::new();
        let jobs = &mut self.jobs;
        self.order.retain(|key| {
            if key.doc != *doc {
                return true;
            }
            if let Some(job) = jobs.remove(key) {
                removed.extend(job.waiters);
            }
            false
        });
        removed
    }

    /// Removes every queued job and returns all waiters (shutdown). Running jobs keep theirs.
    pub(crate) fn drain(&mut self) -> Vec<W> {
        let mut waiters = Vec::new();
        for key in self.order.drain(..) {
            if let Some(job) = self.jobs.remove(&key) {
                waiters.extend(job.waiters);
            }
        }
        waiters
    }

    /// Number of queued (not running) jobs.
    pub(crate) fn queued(&self) -> usize {
        self.order.len()
    }

    fn wanted(&self, key: &TileKey) -> bool {
        wanted(&self.interest, key)
    }
}

fn wanted(interest: &HashMap<ContentHash, Vec<TileRange>>, key: &TileKey) -> bool {
    interest
        .get(&key.doc)
        .is_none_or(|ranges| ranges.iter().any(|r| r.contains(key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &[u8]) -> ContentHash {
        ContentHash::of(name)
    }

    fn key(d: &[u8], x: u32) -> TileKey {
        TileKey {
            doc: doc(d),
            sheet: 0,
            zoom: 0,
            x,
            y: 0,
        }
    }

    fn range(x: std::ops::Range<u32>) -> TileRange {
        TileRange {
            sheet: 0,
            zoom: 0,
            columns: x,
            rows: 0..1,
        }
    }

    #[test]
    fn fifo_and_coalescing() {
        let mut s = Scheduler::default();
        assert!(s.push(key(b"a", 0), 1));
        assert!(s.push(key(b"a", 1), 2));
        assert!(!s.push(key(b"a", 0), 3));
        assert_eq!(s.queued(), 2);
        let Next::Run(k) = s.next() else { panic!() };
        assert_eq!(k, key(b"a", 0));
        // Joins the running job.
        assert!(!s.push(key(b"a", 0), 4));
        assert_eq!(s.finish(&k), vec![1, 3, 4]);
        let Next::Run(k) = s.next() else { panic!() };
        assert_eq!(k, key(b"a", 1));
        assert_eq!(s.finish(&k), vec![2]);
        assert!(matches!(s.next(), Next::Idle));
    }

    #[test]
    fn interest_cancels_queued_jobs_of_that_document_only() {
        let mut s = Scheduler::default();
        for x in 0..4 {
            s.push(key(b"a", x), x);
            s.push(key(b"b", x), 10 + x);
        }
        let mut cancelled = s.set_interest(doc(b"a"), Some(vec![range(1..3)]));
        cancelled.sort_unstable();
        assert_eq!(cancelled, vec![0, 3]);
        assert_eq!(s.queued(), 6);
        // Everything of "b" is cancelled by an empty interest.
        let mut cancelled = s.set_interest(doc(b"b"), Some(Vec::new()));
        cancelled.sort_unstable();
        assert_eq!(cancelled, vec![10, 11, 12, 13]);
        assert_eq!(s.queued(), 2);
    }

    #[test]
    fn interest_is_checked_when_a_job_is_taken() {
        let mut s = Scheduler::default();
        s.set_interest(doc(b"a"), Some(vec![range(0..1)]));
        // Pushed after the interest was set, outside of it.
        s.push(key(b"a", 5), 1);
        s.push(key(b"a", 0), 2);
        assert!(matches!(s.next(), Next::Cancelled(w) if w == vec![1]));
        assert!(matches!(s.next(), Next::Run(k) if k == key(b"a", 0)));
        // Lifting the interest lets everything through again.
        s.set_interest(doc(b"a"), None);
        s.push(key(b"a", 5), 3);
        assert!(matches!(s.next(), Next::Run(k) if k == key(b"a", 5)));
    }

    #[test]
    fn running_jobs_are_not_cancelled() {
        let mut s = Scheduler::default();
        s.push(key(b"a", 0), 1);
        let Next::Run(k) = s.next() else { panic!() };
        assert_eq!(
            s.set_interest(doc(b"a"), Some(Vec::new())),
            Vec::<i32>::new()
        );
        assert_eq!(s.drain(), Vec::<i32>::new());
        assert_eq!(s.finish(&k), vec![1]);
    }

    #[test]
    fn remove_document_and_drain() {
        let mut s = Scheduler::default();
        s.push(key(b"a", 0), 1);
        s.push(key(b"b", 0), 2);
        s.push(key(b"a", 1), 3);
        assert_eq!(s.remove_document(&doc(b"a")), vec![1, 3]);
        assert_eq!(s.drain(), vec![2]);
        assert_eq!(s.queued(), 0);
    }
}
