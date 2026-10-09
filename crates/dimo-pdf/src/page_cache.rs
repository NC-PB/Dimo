//! Most recently used loaded pages of one document, kept on the render thread (T1.11).
//!
//! Loading a page makes PDFium parse its whole content stream. On a dense A0 sheet that took
//! 52 ms, while rendering one tile of the loaded page took 7.5 ms (`docs/perf/M0.md`). The
//! render thread therefore keeps the last [`PAGES_PER_DOCUMENT`] pages of every open document
//! loaded and serves tiles, text runs and sheet analysis from them.
//!
//! # Ownership, no `unsafe`
//!
//! A `PdfPage<'a>` of pdfium-render does not borrow its `PdfDocument<'a>`: both only carry the
//! lifetime of the PDFium bindings, which the render thread owns for its whole life. So a page
//! can live in the same struct as its document without a self referential type and without
//! crates such as ouroboros. What the type system does not check is the order of the FFI
//! calls: `FPDF_ClosePage` must run before `FPDF_CloseDocument`. The render thread keeps both
//! in `OpenDoc` (engine.rs), whose `Drop` clears this cache before the document field is
//! dropped, so every page is closed first. Nothing else holds a page: the cache hands out
//! shared references only, for the duration of one request.
//!
//! The cache is generic over the page type so its behavior is tested without PDFium.

/// Loaded pages kept per open document.
///
/// Tile requests of one view come from one sheet, plus the coarse backdrop level of the same
/// sheet, so one page covers panning and zooming. Four leave room for switching back and forth
/// between a few sheets (drawing and its detail sheets) without reloading. Measured on the
/// 50 sheet A0 stress PDF (about 23 000 page objects per sheet): one loaded page costs about
/// 10.5 MiB, so four pages per document cost about 42 MiB, far below the 1.5 GB of NFR-PERF-03
/// (`docs/perf/M0.md`, "Page cache (T1.11)").
pub(crate) const PAGES_PER_DOCUMENT: usize = 4;

/// A small least recently used cache of loaded pages keyed by sheet index.
pub(crate) struct PageCache<P> {
    /// Most recently used first. Short (at most `capacity`), so a vector beats a map.
    pages: Vec<(usize, P)>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

/// Counters of one cache, for tests and logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageCacheStats {
    /// Cached sheets, most recently used first.
    pub(crate) sheets: Vec<usize>,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl<P> PageCache<P> {
    /// An empty cache that keeps at most `capacity` pages (at least one).
    pub(crate) fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            pages: Vec::with_capacity(capacity),
            capacity,
            hits: 0,
            misses: 0,
        }
    }

    /// The page of `sheet`, loaded with `load` on a miss. A new page evicts the least recently
    /// used one when the cache is full; the evicted page is dropped (closed) before `load` runs,
    /// so at most `capacity` pages are loaded at any time. A failed load caches nothing.
    pub(crate) fn get_or_load<E>(
        &mut self,
        sheet: usize,
        load: impl FnOnce() -> Result<P, E>,
    ) -> Result<&P, E> {
        if let Some(pos) = self.pages.iter().position(|(s, _)| *s == sheet) {
            self.hits += 1;
            let entry = self.pages.remove(pos);
            self.pages.insert(0, entry);
        } else {
            self.misses += 1;
            if self.pages.len() >= self.capacity {
                self.pages.pop();
            }
            let page = load()?;
            self.pages.insert(0, (sheet, page));
        }
        // The page of `sheet` is at the front now.
        Ok(&self.pages[0].1)
    }

    /// Drops every page.
    pub(crate) fn clear(&mut self) {
        self.pages.clear();
    }

    pub(crate) fn stats(&self) -> PageCacheStats {
        PageCacheStats {
            sheets: self.pages.iter().map(|(s, _)| *s).collect(),
            hits: self.hits,
            misses: self.misses,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    /// A page that records its sheet in a shared log when dropped.
    struct Page {
        sheet: usize,
        closed: Rc<RefCell<Vec<usize>>>,
    }

    impl Drop for Page {
        fn drop(&mut self) {
            self.closed.borrow_mut().push(self.sheet);
        }
    }

    /// Returns the sheet of the page the cache hands out and logs every load in `loads`.
    fn get(
        cache: &mut PageCache<Page>,
        sheet: usize,
        closed: &Rc<RefCell<Vec<usize>>>,
        loads: &mut Vec<usize>,
    ) -> usize {
        let page = cache
            .get_or_load(sheet, || {
                loads.push(sheet);
                Ok::<_, ()>(Page {
                    sheet,
                    closed: Rc::clone(closed),
                })
            })
            .unwrap();
        page.sheet
    }

    #[test]
    fn hit_returns_the_cached_page_without_loading() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let mut loads = Vec::new();
        let mut cache = PageCache::new(2);
        assert_eq!(get(&mut cache, 3, &closed, &mut loads), 3);
        assert_eq!(get(&mut cache, 3, &closed, &mut loads), 3);
        assert_eq!(get(&mut cache, 3, &closed, &mut loads), 3);
        assert_eq!(loads, vec![3]);
        let stats = cache.stats();
        assert_eq!((stats.hits, stats.misses), (2, 1));
        assert_eq!(stats.sheets, vec![3]);
        assert!(closed.borrow().is_empty());
    }

    #[test]
    fn evicts_the_least_recently_used_page() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let mut loads = Vec::new();
        let mut cache = PageCache::new(3);
        for sheet in [0, 1, 2] {
            get(&mut cache, sheet, &closed, &mut loads);
        }
        // Touch 0, so 1 is the least recently used.
        get(&mut cache, 0, &closed, &mut loads);
        get(&mut cache, 7, &closed, &mut loads);
        assert_eq!(*closed.borrow(), vec![1]);
        assert_eq!(cache.stats().sheets, vec![7, 0, 2]);
        // 1 is gone, so it loads again and evicts 2.
        get(&mut cache, 1, &closed, &mut loads);
        assert_eq!(*closed.borrow(), vec![1, 2]);
        assert_eq!(cache.stats().sheets, vec![1, 7, 0]);
        assert_eq!(loads, vec![0, 1, 2, 7, 1]);
        let stats = cache.stats();
        assert_eq!((stats.hits, stats.misses), (1, 5));
    }

    #[test]
    fn evicted_page_is_closed_before_the_new_one_loads() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let mut cache = PageCache::new(1);
        let c = Rc::clone(&closed);
        cache
            .get_or_load(0, || {
                Ok::<_, ()>(Page {
                    sheet: 0,
                    closed: c,
                })
            })
            .unwrap();
        let c = Rc::clone(&closed);
        let seen = Rc::clone(&closed);
        cache
            .get_or_load(1, move || {
                // Sheet 0 must already be closed when sheet 1 loads.
                assert_eq!(*seen.borrow(), vec![0]);
                Ok::<_, ()>(Page {
                    sheet: 1,
                    closed: c,
                })
            })
            .unwrap();
        assert_eq!(cache.stats().sheets, vec![1]);
    }

    #[test]
    fn failed_load_caches_nothing() {
        let mut cache: PageCache<u32> = PageCache::new(2);
        assert_eq!(cache.get_or_load(5, || Err("broken")), Err("broken"));
        assert_eq!(cache.stats().sheets, Vec::<usize>::new());
        assert_eq!(cache.get_or_load(5, || Ok::<_, &str>(9)), Ok(&9));
        assert_eq!(cache.stats().misses, 2);
    }

    #[test]
    fn clear_and_drop_close_every_page() {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let mut loads = Vec::new();
        let mut cache = PageCache::new(4);
        for sheet in [0, 1, 2] {
            get(&mut cache, sheet, &closed, &mut loads);
        }
        cache.clear();
        assert_eq!(closed.borrow().len(), 3);
        assert_eq!(cache.stats().sheets, Vec::<usize>::new());
        for sheet in [4, 5] {
            get(&mut cache, sheet, &closed, &mut loads);
        }
        drop(cache);
        let mut all = closed.borrow().clone();
        all.sort_unstable();
        assert_eq!(all, vec![0, 1, 2, 4, 5]);
    }

    #[test]
    fn capacity_is_at_least_one() {
        let mut cache = PageCache::new(0);
        cache.get_or_load(0, || Ok::<_, ()>(1)).unwrap();
        cache.get_or_load(0, || Ok::<_, ()>(2)).unwrap();
        assert_eq!(cache.stats().hits, 1);
    }
}
