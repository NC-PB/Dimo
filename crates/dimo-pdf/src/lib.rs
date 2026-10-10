//! PDF access through PDFium: tile rendering, text runs with geometry, vector paths and writing
//! ballooned PDFs (ADR 0005).
//!
//! Current scope (T0.5): open a document from bytes, its SHA-256 (FR-DOC-07), sheet count and
//! sheet sizes, and rendering a region of a sheet at a zoom level into an RGBA buffer.
//! T0.7 adds [`tiles`]: the tile grid, worker pool and memory and disk caches behind the
//! viewport's `dimo://tile/...` protocol.
//!
//! T0.6: text runs with geometry per sheet ([`Document::text_runs`], see [`TextRun`] for how
//! characters are merged) and sheet classification from the page content
//! ([`Document::analyze_sheet`], see [`SheetAnalysis`] for the rule).
//!
//! T1.2: [`PdfEngine::write_ballooned`] writes a copy of a PDF with balloons as vector page
//! content or as annotations (FR-EXP-01, D-33), deterministic (FR-EXP-11). The primitives are
//! in [`overlay`], the bundled number font and its subsetter in `font`.
//! T1.9: [`project_overlay`] maps the balloons of a project to these primitives with the
//! balloon layout rule shared with the viewport (`dimo_core::BALLOON_METRICS`).
//!
//! T2.6: [`Document::region_text`] returns the characters, runs and basic dimension frames
//! inside a region drawn by the user, also for rotated text (see [`region`]).
//!
//! # Coordinates
//!
//! All geometry is in sheet space (AGENTS.md rule 4): PDF user units, origin at the top left of
//! the sheet, y downward. See [`geometry`].
//!
//! # Threading
//!
//! PDFium is not thread safe, and pdfium-render keeps its function bindings in a process
//! global, so a process can load PDFium only once. This crate therefore runs **all PDFium calls
//! on one dedicated render thread** (`dimo-pdfium`), started by [`PdfEngine::start`].
//! [`PdfEngine`] and [`Document`] are handles that send requests to that thread over a channel
//! and wait for the answer. They are `Send + Sync` and can be used from any thread.
//!
//! Why a thread and not a mutex around a shared instance:
//!
//! - A document borrows the PDFium instance. A thread that owns both keeps that borrow inside
//!   one stack frame: no self referential structs, no `unsafe`.
//! - A mutex would serialize the same calls, but every caller would hold it for a whole render,
//!   and pdfium-render's `thread_safe` feature would add a second global lock around every FFI
//!   call.
//! - A request queue is where tile priorities and cancellation (M1 viewer) fit naturally.
//!
//! The cost is one extra thread and that rendering uses one core; PDFium cannot render in
//! parallel within one process anyway. If the render thread panics, every later call returns
//! [`PdfError::EngineStopped`].
//!
//! T1.11: the render thread keeps the most recently used loaded pages of every open document
//! (four per document) and serves renders, text runs and sheet analysis from them, because
//! loading a dense page costs several times more than rendering a tile of it. Closing a
//! document closes its cached pages first. See `page_cache` for why this needs no `unsafe`.
//!
//! # Locating PDFium
//!
//! See [`library`]: explicit path, then `DIMO_PDFIUM_PATH`, then `vendor/pdfium/<target>/`
//! filled by `scripts/fetch-pdfium.sh`. The pinned release is in `scripts/pdfium.toml`; its API
//! level must match the `pdfium_*` feature of pdfium-render in the workspace `Cargo.toml`.

pub mod geometry;
pub mod library;
pub mod overlay;
pub mod project_overlay;
pub mod region;
pub mod sheet_kind;
pub mod text;
pub mod tiles;

mod engine;
mod error;
mod font;
mod hash;
mod page_cache;
mod page_space;
mod raster;
mod writer;

pub use engine::{Document, MAX_RENDER_SIDE, PdfEngine};
pub use error::PdfError;
pub use geometry::{SheetRect, SheetSize};
pub use hash::ContentHash;
pub use overlay::{
    Balloon, BalloonOutput, BalloonOverlay, BalloonShape, Leader, PdfDate, Rgb, SheetBalloons,
    SheetPoint, Stroke,
};
pub use project_overlay::project_overlay;
pub use raster::RgbaImage;
pub use region::{RegionRun, RegionText, TextChar};
pub use sheet_kind::{FontInfo, MIN_TEXT_CHARS, RASTER_COVERAGE, SheetAnalysis};
pub use text::TextRun;
