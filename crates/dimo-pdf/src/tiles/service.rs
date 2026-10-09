//! The tile service: document registry, worker pool and both caches.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, RwLock, mpsc};
use std::thread::JoinHandle;
use std::time::SystemTime;

use crate::engine::{Document, PdfEngine};
use crate::error::PdfError;
use crate::hash::ContentHash;

use super::disk::DiskCache;
use super::encode::encode_png;
use super::grid::{TileGrid, TileKey, TileRange, zoom_scale};
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

struct Shared {
    engine: PdfEngine,
    docs: RwLock<HashMap<ContentHash, Arc<Document>>>,
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

    /// Opens a document from its bytes and registers it under its content hash, or returns the
    /// already open document with the same hash. Blocks while PDFium parses the file.
    pub fn open_document(&self, bytes: Vec<u8>) -> Result<Arc<Document>, PdfError> {
        let hash = ContentHash::of(&bytes);
        let open = read(&self.shared.docs).get(&hash).cloned();
        let doc = if let Some(doc) = open {
            doc
        } else {
            let doc = Arc::new(self.shared.engine.open(bytes)?);
            let mut docs = write(&self.shared.docs);
            Arc::clone(docs.entry(hash).or_insert(doc))
        };
        if let Some(disk) = &self.shared.disk
            && let Err(e) = disk.touch(&hash)
        {
            tracing::warn!("cannot mark tile cache use: {e}");
        }
        Ok(doc)
    }

    /// The PDF engine the tiles render with, for other work such as writing a ballooned PDF.
    pub fn engine(&self) -> &PdfEngine {
        &self.shared.engine
    }

    /// The open document with this content hash.
    pub fn document(&self, doc: &ContentHash) -> Option<Arc<Document>> {
        read(&self.shared.docs).get(doc).cloned()
    }

    /// Closes a document: queued requests for it are cancelled and its tiles leave the memory
    /// cache (the disk cache keeps them). Returns whether it was open.
    pub fn close_document(&self, doc: &ContentHash) -> bool {
        let was_open = write(&self.shared.docs).remove(doc).is_some();
        let waiters = lock(&self.shared.queue).remove_document(doc);
        lock(&self.shared.memory).retain(|key| key.doc != *doc);
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
        if let Err(e) = self.validate(&key) {
            done(Err(e));
            return;
        }
        if let Some(tile) = lock(&self.shared.memory).get(&key) {
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
    pub fn set_interest(&self, doc: ContentHash, ranges: Option<Vec<TileRange>>) {
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

    /// Checks that the tile exists: document open, sheet, zoom level and position in range.
    fn validate(&self, key: &TileKey) -> Result<(), TileError> {
        let doc = self
            .document(&key.doc)
            .ok_or_else(|| TileError::UnknownDocument(key.doc.to_hex()))?;
        tile_grid(&doc, key).map(|_| ())
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
        if let Some(tile) = lock(&self.memory).get(key) {
            self.memory_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(tile);
        }
        if let Some(bytes) = self.disk.as_ref().and_then(|disk| disk.read(key)) {
            self.disk_hits.fetch_add(1, Ordering::Relaxed);
            let tile = Tile::new(bytes);
            lock(&self.memory).insert(*key, tile.clone());
            return Ok(tile);
        }
        let doc = read(&self.docs)
            .get(&key.doc)
            .cloned()
            .ok_or_else(|| TileError::UnknownDocument(key.doc.to_hex()))?;
        let grid = tile_grid(&doc, key)?;
        let (region, (width, height)) = grid
            .tile_region(key.x, key.y)
            .zip(grid.tile_pixels(key.x, key.y))
            .ok_or_else(|| out_of_range(key))?;
        // `render_region` applies the glyph edge margin itself.
        let image = doc
            .render_region(key.sheet as usize, region, zoom_scale(key.zoom))
            .map_err(|e| TileError::Render(e.to_string()))?;
        self.renders.fetch_add(1, Ordering::Relaxed);
        let tile = Tile::new(encode_png(&image, 0, 0, width, height)?);
        lock(&self.memory).insert(*key, tile.clone());
        if let Some(disk) = &self.disk
            && let Err(e) = disk.write(key, tile.as_bytes())
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

fn tile_grid(doc: &Document, key: &TileKey) -> Result<TileGrid, TileError> {
    let size = doc
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
