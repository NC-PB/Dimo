//! Background jobs of the shell (05 Architecture, IPC): IDs, cancel flags, progress events and
//! the thread a job runs on. Export jobs use it ([`crate::export`]); recognition runs only take
//! an ID.
//!
//! A job checks its cancel flag between steps; [`cancel_job`] sets it. Progress goes to the
//! frontend as [`JobProgress`] events; how a job ends is the job's own event.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Manager, State};

use crate::ipc::{CommandError, JobId, JobProgress};
use crate::project::emit;

/// Cancel flags of the running jobs.
#[derive(Debug, Default)]
pub struct JobRegistry {
    running: Mutex<HashMap<JobId, Arc<AtomicBool>>>,
}

impl JobRegistry {
    /// Registers job `id` and returns its cancel flag.
    pub fn start(&self, id: JobId) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.lock().insert(id, Arc::clone(&flag));
        flag
    }

    /// Asks job `id` to stop before its next step. False if no such job runs.
    pub fn cancel(&self, id: JobId) -> bool {
        self.lock().get(&id).is_some_and(|flag| {
            flag.store(true, Ordering::SeqCst);
            true
        })
    }

    /// Forgets job `id` when it ended.
    pub fn finish(&self, id: JobId) {
        self.lock().remove(&id);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<JobId, Arc<AtomicBool>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// IDs of background jobs within one app session.
static NEXT_JOB: AtomicU32 = AtomicU32::new(1);

/// A new job ID, also for recognition runs that answer directly (T2.6).
pub(crate) fn next_job_id() -> JobId {
    JobId(NEXT_JOB.fetch_add(1, Ordering::SeqCst))
}

/// Why a job stopped before it wrote its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStop {
    /// [`cancel_job`] was called.
    Cancelled,
    /// A step failed.
    Failed(CommandError),
}

impl From<CommandError> for JobStop {
    fn from(error: CommandError) -> Self {
        Self::Failed(error)
    }
}

/// Sends the progress of job `id` to the frontend.
pub(crate) fn progress(app: &AppHandle, id: JobId, fraction: f64, message: &str) {
    emit(
        app,
        &JobProgress {
            id,
            fraction,
            message: message.to_owned(),
        },
    );
}

/// Forgets the job in the registry when its thread ends, also by a panic.
struct Finish {
    app: AppHandle,
    id: JobId,
}

impl Drop for Finish {
    fn drop(&mut self) {
        self.app.state::<JobRegistry>().finish(self.id);
    }
}

/// Starts `work` as a new job on its own thread named `name` and returns its ID at once.
/// `work` gets the ID and the cancel flag; the job leaves the registry when `work` returns.
pub(crate) fn spawn(
    app: &AppHandle,
    name: &str,
    work: impl FnOnce(JobId, &AtomicBool) + Send + 'static,
) -> Result<JobId, CommandError> {
    let id = next_job_id();
    let cancel = app.state::<JobRegistry>().start(id);
    let finish = Finish {
        app: app.clone(),
        id,
    };
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || {
            let _finish = finish;
            work(id, &cancel);
        })
        // A thread that does not start drops `work` and with it `finish`.
        .map_err(CommandError::task_failed)?;
    Ok(id)
}

/// Asks the job `id` to stop before its next step. Returns false if no such job runs (it may
/// just have ended). A job that is already writing its file finishes it and ends with its
/// normal outcome.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub fn cancel_job(jobs: State<'_, JobRegistry>, id: JobId) -> bool {
    jobs.cancel(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_flags_follow_the_jobs() {
        let jobs = JobRegistry::default();
        let flag = jobs.start(JobId(4));
        assert!(!jobs.cancel(JobId(5)), "unknown job");
        assert!(jobs.cancel(JobId(4)));
        assert!(flag.load(Ordering::SeqCst));
        jobs.finish(JobId(4));
        assert!(!jobs.cancel(JobId(4)), "ended job");
    }

    #[test]
    fn job_ids_increase() {
        let a = next_job_id();
        let b = next_job_id();
        assert!(b.0 > a.0);
    }
}
