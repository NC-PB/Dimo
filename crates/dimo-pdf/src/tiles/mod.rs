//! Tiled rendering for the drawing viewport (T0.7, 05 Architecture "Tile layer").
//!
//! The viewport shows a sheet as a grid of 512 px tiles requested through the custom URI
//! protocol `dimo://tile/{doc}/{sheet}/{zoom}/{x}/{y}`. This module holds everything behind
//! that route except the Tauri wiring: addressing, scheduling, rendering, encoding and caching.
//!
//! # Addressing
//!
//! - `doc` is the SHA-256 of the drawing (FR-DOC-07) as 64 hex digits. A URL therefore always
//!   names the same pixels, and the disk cache can use the same key.
//! - `zoom` is a power of two level: `2^zoom` pixels per sheet unit, level 0 is 72 dpi
//!   ([`MIN_ZOOM`]`..=`[`MAX_ZOOM`]). The viewport scales between levels.
//! - `x`, `y` are column and row in the [`TileGrid`] of that sheet and level. Edge tiles are
//!   cut at the sheet edge.
//!
//! # Pipeline
//!
//! [`TileService::request`] answers from the memory cache at once when it can. Otherwise the
//! request goes into a queue served by a small pool of worker threads. A worker checks the
//! disk cache, else renders the tile through [`crate::Document::render_region`], encodes it
//! as PNG (see `encode.rs` for the choice) and stores it in both caches. PDFium itself renders
//! one tile at a time on its own thread (see the crate docs), so the pool exists to overlap
//! rendering with encoding and disk IO, and to keep at most one render per worker committed to
//! the render thread. Everything else waits in the queue, where it can still be cancelled.
//! Requests for the same tile share one job.
//!
//! # Caches
//!
//! - Memory: [`MemoryCache`], least recently used tiles evicted first, a hard byte budget
//!   ([`TileConfig::memory_budget`]).
//! - Disk: in the directory the app passes, normally the user cache directory, never the
//!   project. Keyed by content hash and a render version. Pruned to
//!   [`TileConfig::disk_budget`] when the service starts, least recently used documents first.
//!
//! # Cancellation
//!
//! The viewport declares which tiles it still wants with [`TileService::set_interest`]: a list
//! of [`TileRange`]s per document, normally the visible tiles plus a margin. Queued requests
//! outside the interest are answered with [`TileError::Cancelled`] immediately, and requests
//! that arrive later outside it are cancelled when a worker takes them. A tile that is already
//! being rendered completes and is cached. This needs no request ids and is robust against
//! the order in which protocol requests and commands arrive. Without a declared interest
//! every request is served.

mod disk;
mod encode;
mod grid;
mod memory;
mod schedule;
mod service;

use std::sync::Arc;

pub use encode::TILE_CONTENT_TYPE;
pub use grid::{MAX_ZOOM, MIN_ZOOM, TILE_SIZE, TileGrid, TileKey, TileRange, zoom_scale};
pub use memory::MemoryCache;
pub use service::{TileConfig, TileService, TileStats};

/// An encoded tile (PNG bytes, [`TILE_CONTENT_TYPE`]). Cheap to clone.
#[derive(Clone, PartialEq, Eq)]
pub struct Tile(Arc<[u8]>);

impl Tile {
    /// Wraps encoded bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }

    /// The encoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Size in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the tile has no bytes (never true for a rendered tile).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tile").field("len", &self.len()).finish()
    }
}

/// Why a tile could not be delivered. Cloneable because one result can answer several requests.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TileError {
    /// The route is not `tile/{doc}/{sheet}/{zoom}/{x}/{y}` with valid numbers.
    #[error("invalid tile address: {0}")]
    InvalidAddress(String),
    /// No open document has this content hash.
    #[error("document {0} is not open")]
    UnknownDocument(String),
    /// Sheet, zoom level or tile position outside the document.
    #[error("tile out of range: {0}")]
    OutOfRange(String),
    /// The request left the viewport's interest before it was rendered.
    #[error("tile request cancelled")]
    Cancelled,
    /// Rendering or encoding failed.
    #[error("tile render failed: {0}")]
    Render(String),
    /// The service shut down before the request was served.
    #[error("tile service stopped")]
    Stopped,
}
