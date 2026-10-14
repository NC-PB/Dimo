//! The tile service: source registry, worker pool and both caches.
//!
//! Sources are registered by their render key (the content hash for PDFs). A PDF opened through [`TileService::open_document`]
//! (or [`RasterOpener::open_pdf`]) is kept as its [`Document`] too, for text and recognition;
//! any other [`RasterSource`] is registered with [`TileService::insert_source`]. Tiles are
//! rendered through the [`RasterSource`] only (MF F1).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, RwLock, mpsc};
use std::thread::JoinHandle;
use std::time::SystemTime;

use dimo_core::Sha256Hex;

use crate::engine::{Document, PdfEngine};
use crate::error::PdfError;
use crate::source::{Opened, RasterOpener, RasterSource, SourceKind};

use super::disk::DiskCache;
use super::encode::encode_png;
use super::grid::{CacheKey, TileGrid, TileKey, TileRange, zoom_scale};
use super::memory::MemoryCache;
use super::schedule::{Next, Scheduler};
use super::{Tile, TileError};

const MIB: usize = 1024 * 1024;

/// Callback that receives the result of one tile request.
type Done = Box<dyn FnOnce(Result<Tile, TileError>) + Send>;

/// Settings of a [`TileService`].
#[derive(Debug, Clone)]
pub struct TileConfig {
    /// Byte budget of the in memory cache of encoded tiles. Default 128 MiB, a small part of
    /// the 1.5 GB budget of NFR-PERF-03 (about 20 000 tiles of `test_drawing_1.pdf`).
    pub memory_budget: usize,
    /// Directory of the disk cache, or `None` for no disk cache. The app passes a folder in the
    /// user cache directory, never a project folder.
    pub disk_dir: Option<PathBuf>,
    /// Size the disk cache is pruned to when the service starts. Default 2 GiB. Between starts
    /// the cache can grow beyond it.
    pub disk_budget: u64,
    /// Number of worker threads. Default: available cores minus one, between 2 and 4. With 0,
    /// requests that miss the memory cache are queued but never served (tests only).
    pub workers: usize,
}

impl Default for TileConfig {
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, std::num::NonZero::get);
        Self {
            memory_budget: 128 * MIB,
            disk_dir: None,
            disk_budget: 2 * 1024 * 1024 * 1024,
            workers: cores.saturating_sub(1).clamp(2, 4),
        }
    }
}

/// Counters of a running service, for tests, logs and the performance harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileStats {
    /// Bytes in the memory cache.
    pub memory_bytes: usize,
    /// Highest number of bytes the memory cache ever held.
    pub memory_peak_bytes: usize,
    /// Byte budget of the memory cache.
    pub memory_budget: usize,
    /// Tiles in the memory cache.
    pub memory_tiles: usize,
    /// Jobs waiting in the queue.
    pub queued: usize,
    /// Requests answered from the memory cache.
    pub memory_hits: u64,
    /// Tiles read from the disk cache.
    pub disk_hits: u64,
    /// Tiles rendered by PDFium.
    pub renders: u64,
    /// Requests answered with [`TileError::Cancelled`].
    pub cancelled: u64,
}

/// Renders, caches and serves tiles of open documents. See the module docs.
///
/// All methods take `&self` and are safe to call from any thread. None of them blocks on
/// rendering except [`TileService::get`] and [`TileService::open_document`]. Dropping the
/// service answers queued requests with [`TileError::Stopped`] and joins the workers.
pub struct TileService {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

/// A registered source; `pdf` is the same source as a PDF document, if it is one.
#[derive(Clone)]
struct Registered {
    source: Arc<dyn RasterSource>,
    pdf: Option<Arc<Document>>,
}

struct Shared {
    /// PDFium, `None` for a service that renders other sources only.
    engine: Option<PdfEngine>,
    docs: RwLock<HashMap<Sha256Hex, Registered>>,
    memory: Mutex<MemoryCache>,
    disk: Option<DiskCache>,
    queue: Mutex<Scheduler<Done>>,
    wake: Condvar,
    stop: AtomicBool,
    memory_hits: AtomicU64,
    disk_hits: AtomicU64,
    renders: AtomicU64,
    cancelled: AtomicU64,
}

impl std::fmt::Debug for TileService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileService")
            .field("workers", &self.workers.len())
            .field("stats", &self.stats())
            .finish_non_exhaustive()
    }
}

impl TileService {
    /// Starts the worker threads. When a disk cache is configured but cannot be created, the
    /// service runs without it and logs a warning.
    pub fn new(engine: PdfEngine, config: TileConfig) -> Self {
        Self::start(Some(engine), config)
    }

    /// A service without PDFium: it renders sources registered with
    /// [`TileService::insert_source`] and refuses to open PDFs.
    pub fn without_pdf(config: TileConfig) -> Self {
        Self::start(None, config)
    }

    fn start(engine: Option<PdfEngine>, config: TileConfig) -> Self {
        let TileConfig {
            memory_budget,
            disk_dir,
            disk_budget,
            workers,
        } = config;
        let disk = disk_dir
            .as_deref()
            .and_then(|dir| match DiskCache::open(dir) {
                Ok(disk) => Some(disk),
                Err(e) => {
                    tracing::warn!(dir = %dir.display(), "tile disk cache disabled: {e}");
                    None
                }
            });
        let shared = Arc::new(Shared {
            engine,
            docs: RwLock::new(HashMap::new()),
            memory: Mutex::new(MemoryCache::new(memory_budget)),
            disk,
            queue: Mutex::new(Scheduler::default()),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            memory_hits: AtomicU64::new(0),
            disk_hits: AtomicU64::new(0),
            renders: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        });
        if shared.disk.is_some() {
            spawn_prune(&shared, disk_budget);
        }
        let workers = (0..workers)
            .filter_map(|i| {
                let shared = Arc::clone(&shared);
                std::thread::Builder::new()
                    .name(format!("dimo-tiles-{i}"))
                    .spawn(move || shared.work())
                    .inspect_err(|e| tracing::warn!("cannot start tile worker: {e}"))
                    .ok()
            })
            .collect();
        Self { shared, workers }
    }

    /// Opens a PDF from its bytes and registers it under its content hash, or returns the
    /// already open document with the same hash. Blocks while PDFium parses the file.
    pub fn open_document(&self, bytes: Vec<u8>) -> Result<Arc<Document>, PdfError> {
        self.open_pdf_document(bytes).map(|(doc, _)| doc)
    }

    /// [`TileService::open_document`], also telling whether this call opened it.
    fn open_pdf_document(&self, bytes: Vec<u8>) -> Result<(Arc<Document>, bool), PdfError> {
        let hash = Sha256Hex::of(&bytes);
        let open = read(&self.shared.docs).get(&hash).cloned();
        let (doc, newly_opened) = match open {
            Some(Registered { pdf: Some(doc), .. }) => (doc, false),
            Some(Registered { source, .. }) => {
                return Err(other_kind(&hash, source.kind()));
            }
            None => {
                let engine = self.shared.engine.as_ref().ok_or(PdfError::NoPdfium)?;
                let doc = Arc::new(engine.open(bytes)?);
                let mut docs = write(&self.shared.docs);
                let entry = docs.entry(hash).or_insert_with(|| Registered {
                    source: Arc::clone(&doc) as Arc<dyn RasterSource>,
                    pdf: Some(Arc::clone(&doc)),
                });
                match &entry.pdf {
                    Some(pdf) if Arc::ptr_eq(pdf, &doc) => (doc, true),
                    // Another thread opened it in between.
                    Some(pdf) => (Arc::clone(pdf), false),
                    None => return Err(other_kind(&hash, entry.source.kind())),
                }
            }
        };
        self.touch(SourceKind::Pdf, &hash);
        Ok((doc, newly_opened))
    }

    /// Registers a source under its [`RasterSource::render_key`] and returns it, or the source
    /// already registered under that key. A source with other settings has another key, so it
    /// never gets the tiles of the old settings; close the old one when it is no longer shown.
    pub fn insert_source(&self, source: Arc<dyn RasterSource>) -> Arc<dyn RasterSource> {
        let hash = source.render_key();
        let registered = {
            let mut docs = write(&self.shared.docs);
            let entry = docs
                .entry(hash)
                .or_insert_with(|| Registered { source, pdf: None });
            Arc::clone(&entry.source)
        };
        self.touch(registered.kind(), &hash);
        registered
    }

    fn touch(&self, kind: SourceKind, hash: &Sha256Hex) {
        if let Some(disk) = &self.shared.disk
            && let Err(e) = disk.touch(kind, hash)
        {
            tracing::warn!("cannot mark tile cache use: {e}");
        }
    }

    /// The PDF engine the tiles render with, for other work such as writing a ballooned PDF.
    /// `None` for a service started with [`TileService::without_pdf`].
    pub fn engine(&self) -> Option<&PdfEngine> {
        self.shared.engine.as_ref()
    }

    /// The open PDF document with this content hash.
    pub fn document(&self, doc: &Sha256Hex) -> Option<Arc<Document>> {
        read(&self.shared.docs).get(doc).and_then(|r| r.pdf.clone())
    }

    /// The registered source with this render key, of any kind.
    pub fn source(&self, doc: &Sha256Hex) -> Option<Arc<dyn RasterSource>> {
        read(&self.shared.docs)
            .get(doc)
            .map(|r| Arc::clone(&r.source))
    }

    /// Closes a document: queued requests for it are cancelled and its tiles leave the memory
    /// cache (the disk cache keeps them). Returns whether it was open.
    pub fn close_document(&self, doc: &Sha256Hex) -> bool {
        let was_open = write(&self.shared.docs).remove(doc).is_some();
        let waiters = lock(&self.shared.queue).remove_document(doc);
        lock(&self.shared.memory).retain(|key| key.tile.doc != *doc);
        self.shared.cancel(waiters);
        was_open
    }

    /// Requests a tile. `done` is called exactly once with the result: right away on the
    /// calling thread for invalid addresses and memory cache hits, otherwise later on a worker
    /// thread. Never blocks on rendering.
    pub fn request(
        &self,
        key: TileKey,
        done: impl FnOnce(Result<Tile, TileError>) + Send + 'static,
    ) {
        let kind = match self.validate(&key) {
            Ok(kind) => kind,
            Err(e) => {
                done(Err(e));
                return;
            }
        };
        if let Some(tile) = lock(&self.shared.memory).get(&CacheKey { kind, tile: key }) {
            self.shared.memory_hits.fetch_add(1, Ordering::Relaxed);
            done(Ok(tile));
            return;
        }
        let mut queue = lock(&self.shared.queue);
        // Checked under the queue lock, so a request cannot slip in after the final drain.
        if self.shared.stop.load(Ordering::Acquire) {
            drop(queue);
            done(Err(TileError::Stopped));
            return;
        }
        if queue.push(key, Box::new(done)) {
            self.shared.wake.notify_one();
        }
    }

    /// Requests a tile and waits for it. For tests, tools and the command line.
    pub fn get(&self, key: TileKey) -> Result<Tile, TileError> {
        let (tx, rx) = mpsc::channel();
        self.request(key, move |result| {
            let _ = tx.send(result);
        });
        rx.recv().unwrap_or(Err(TileError::Stopped))
    }

    /// Declares which tiles of `doc` the viewport still wants (see "Cancellation" in the module
    /// docs). Queued requests outside `ranges` are answered with [`TileError::Cancelled`] now,
    /// later ones when a worker takes them. `None` serves every request again.
    pub fn set_interest(&self, doc: Sha256Hex, ranges: Option<Vec<TileRange>>) {
        let waiters = lock(&self.shared.queue).set_interest(doc, ranges);
        self.shared.cancel(waiters);
    }

    /// Current counters.
    pub fn stats(&self) -> TileStats {
        let (memory_bytes, memory_peak_bytes, memory_budget, memory_tiles) = {
            let memory = lock(&self.shared.memory);
            (
                memory.bytes(),
                memory.peak_bytes(),
                memory.budget(),
                memory.len(),
            )
        };
        TileStats {
            memory_bytes,
            memory_peak_bytes,
            memory_budget,
            memory_tiles,
            queued: lock(&self.shared.queue).queued(),
            memory_hits: self.shared.memory_hits.load(Ordering::Relaxed),
            disk_hits: self.shared.disk_hits.load(Ordering::Relaxed),
            renders: self.shared.renders.load(Ordering::Relaxed),
            cancelled: self.shared.cancelled.load(Ordering::Relaxed),
        }
    }

    /// Checks that the tile exists: source registered, sheet, zoom level and position in
    /// range. Returns the kind of the source.
    fn validate(&self, key: &TileKey) -> Result<SourceKind, TileError> {
        let source = self
            .source(&key.doc)
            .ok_or_else(|| TileError::UnknownDocument(key.doc.to_hex()))?;
        tile_grid(source.as_ref(), key).map(|_| source.kind())
    }
}

impl Drop for TileService {
    fn drop(&mut self) {
        let waiters = {
            let mut queue = lock(&self.shared.queue);
            self.shared.stop.store(true, Ordering::Release);
            queue.drain()
        };
        self.shared.wake.notify_all();
        for waiter in waiters {
            waiter(Err(TileError::Stopped));
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl Shared {
    /// Worker loop: take a job, produce the tile, answer every waiter. Callbacks always run
    /// outside the locks.
    fn work(&self) {
        loop {
            let next = {
                let mut queue = lock(&self.queue);
                loop {
                    if self.stop.load(Ordering::Acquire) {
                        return;
                    }
                    match queue.next() {
                        Next::Idle => {
                            queue = self
                                .wake
                                .wait(queue)
                                .unwrap_or_else(PoisonError::into_inner);
                        }
                        next => break next,
                    }
                }
            };
            match next {
                Next::Run(key) => {
                    let result = self.produce(&key);
                    let waiters = lock(&self.queue).finish(&key);
                    for waiter in waiters {
                        waiter(result.clone());
                    }
                }
                Next::Cancelled(waiters) => self.cancel(waiters),
                Next::Idle => {}
            }
        }
    }

    /// Memory cache, then disk cache, then render and encode.
    fn produce(&self, key: &TileKey) -> Result<Tile, TileError> {
        let source = read(&self.docs)
            .get(&key.doc)
            .map(|r| Arc::clone(&r.source))
            .ok_or_else(|| TileError::UnknownDocument(key.doc.to_hex()))?;
        let cache_key = CacheKey {
            kind: source.kind(),
            tile: *key,
        };
        if let Some(tile) = lock(&self.memory).get(&cache_key) {
            self.memory_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(tile);
        }
        if let Some(bytes) = self.disk.as_ref().and_then(|disk| disk.read(&cache_key)) {
            self.disk_hits.fetch_add(1, Ordering::Relaxed);
            let tile = Tile::new(bytes);
            lock(&self.memory).insert(cache_key, tile.clone());
            return Ok(tile);
        }
        let grid = tile_grid(source.as_ref(), key)?;
        let (region, (width, height)) = grid
            .tile_region(key.x, key.y)
            .zip(grid.tile_pixels(key.x, key.y))
            .ok_or_else(|| out_of_range(key))?;
        // A PDF document applies the glyph edge margin itself.
        let image = source
            .render_region(key.sheet as usize, region, zoom_scale(key.zoom))
            .map_err(|e| TileError::Render(e.to_string()))?;
        self.renders.fetch_add(1, Ordering::Relaxed);
        let tile = Tile::new(encode_png(&image, 0, 0, width, height)?);
        lock(&self.memory).insert(cache_key, tile.clone());
        if let Some(disk) = &self.disk
            && let Err(e) = disk.write(&cache_key, tile.as_bytes())
        {
            tracing::warn!(tile = %key.route(), "cannot write tile to disk cache: {e}");
        }
        Ok(tile)
    }

    fn cancel(&self, waiters: Vec<Done>) {
        self.cancelled
            .fetch_add(waiters.len() as u64, Ordering::Relaxed);
        for waiter in waiters {
            waiter(Err(TileError::Cancelled));
        }
    }
}

/// PDFs open as registered documents, so they are ready for tiles. A source this call did not
/// open (the drawing of the open project, opened again) is never released.
impl RasterOpener for TileService {
    fn open_pdf(&self, bytes: Vec<u8>) -> Result<Opened, PdfError> {
        let (doc, newly_opened) = self.open_pdf_document(bytes)?;
        Ok(Opened {
            source: doc,
            newly_opened,
        })
    }

    fn release(&self, key: &Sha256Hex) {
        self.close_document(key);
    }
}

fn other_kind(hash: &Sha256Hex, kind: SourceKind) -> PdfError {
    PdfError::Open(format!("{hash} is already open as a {kind} source"))
}

/// Prunes the disk cache on a background thread, so startup does not wait for it.
fn spawn_prune(shared: &Arc<Shared>, budget: u64) {
    let started = SystemTime::now();
    let shared = Arc::clone(shared);
    let spawned = std::thread::Builder::new()
        .name("dimo-tiles-prune".to_owned())
        .spawn(move || {
            if let Some(disk) = &shared.disk {
                match disk.prune(budget, started) {
                    Ok(left) => tracing::debug!(left, budget, "tile disk cache pruned"),
                    Err(e) => tracing::warn!("cannot prune tile disk cache: {e}"),
                }
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("cannot start tile cache pruning: {e}");
    }
}

fn tile_grid(source: &dyn RasterSource, key: &TileKey) -> Result<TileGrid, TileError> {
    let size = source
        .sheet_size(key.sheet as usize)
        .map_err(|e| TileError::OutOfRange(e.to_string()))?;
    let grid = TileGrid::new(size, key.zoom)?;
    grid.tile_pixels(key.x, key.y)
        .map(|_| grid)
        .ok_or_else(|| out_of_range(key))
}

fn out_of_range(key: &TileKey) -> TileError {
    TileError::OutOfRange(format!("tile {}/{} not in the sheet grid", key.x, key.y))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic while holding the lock leaves plain data behind, which stays usable.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}
