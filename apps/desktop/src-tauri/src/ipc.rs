//! IPC types and commands of the desktop shell (NFR-MNT-03).
//!
//! Every type here derives `specta::Type`, so its TypeScript twin is generated into
//! `apps/desktop/src/lib/ipc/bindings.ts`. Never write those types by hand in the frontend.

use dimo_pdf::ContentHash;
use dimo_pdf::tiles::{self, TileKey};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tauri_specta::Event;

use crate::tiles::TileState;

/// Cargo profile the running binary was built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfile {
    /// Built without optimizations, with debug assertions (`tauri dev`).
    Debug,
    /// Optimized release build.
    Release,
}

impl BuildProfile {
    /// Profile of the current binary, derived from `debug_assertions`.
    pub const fn current() -> Self {
        if cfg!(debug_assertions) {
            Self::Debug
        } else {
            Self::Release
        }
    }
}

/// Static facts about the running app, shown in the UI and in bug reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct AppInfo {
    /// App version from the workspace manifest, for example `0.1.0`.
    pub version: String,
    /// Build profile of the running binary.
    pub build_profile: BuildProfile,
    /// Whether the PDFium library could be loaded. Without it no drawing can be opened.
    pub pdfium_available: bool,
}

impl AppInfo {
    /// Collects the facts about the running app. `pdfium_available` comes from the tile
    /// service, which loads PDFium at startup.
    pub fn current(pdfium_available: bool) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            build_profile: BuildProfile::current(),
            pdfium_available,
        }
    }
}

/// Identifier of a background job within one app session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct JobId(pub u32);

/// Progress of a long running job, emitted as a Tauri event (05 Architecture, IPC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct JobProgress {
    /// Job the progress belongs to.
    pub id: JobId,
    /// Completed share of the work, from 0.0 to 1.0.
    pub fraction: f64,
    /// Short description of the current step, shown next to the progress bar.
    pub message: String,
}

/// Error of a command: a machine readable kind and a message for logs and bug reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandError {
    /// The PDFium library could not be loaded at startup; no drawing can be opened.
    #[error("PDFium unavailable: {message}")]
    PdfiumUnavailable {
        /// Why loading failed.
        message: String,
    },
    /// A file could not be read.
    #[error("cannot read file: {message}")]
    Io {
        /// The operating system error.
        message: String,
    },
    /// The file is not a PDF that PDFium can open.
    #[error("cannot open document: {message}")]
    InvalidDocument {
        /// The PDFium error.
        message: String,
    },
    /// An argument is malformed, for example a document id that is not a content hash.
    #[error("invalid argument: {message}")]
    InvalidArgument {
        /// What is wrong.
        message: String,
    },
}

/// Size of one sheet in sheet units (PDF user units, 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct SheetInfo {
    /// Width in sheet units.
    pub width: f64,
    /// Height in sheet units.
    pub height: f64,
}

/// An open document as the viewport needs it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct DocumentInfo {
    /// Content hash (SHA-256, 64 hex digits, FR-DOC-07). The `{doc}` part of tile URLs.
    pub doc: String,
    /// Sheets in order; the index is the `{sheet}` part of tile URLs.
    pub sheets: Vec<SheetInfo>,
}

/// Address of one tile, the parts of `dimo://tile/{doc}/{sheet}/{zoom}/{x}/{y}`.
/// Used by the frontend helper that builds tile URLs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct TileAddress {
    /// Content hash of the document.
    pub doc: String,
    /// Zero based sheet index.
    pub sheet: u32,
    /// Zoom level: `2^zoom` pixels per sheet unit, from `MIN_TILE_ZOOM` to `MAX_TILE_ZOOM`.
    pub zoom: i32,
    /// Tile column.
    pub x: u32,
    /// Tile row.
    pub y: u32,
}

impl TileAddress {
    /// The tile key, or an error when `doc` is not a content hash.
    pub fn key(&self) -> Result<TileKey, CommandError> {
        Ok(TileKey {
            doc: parse_doc(&self.doc)?,
            sheet: self.sheet,
            zoom: self.zoom,
            x: self.x,
            y: self.y,
        })
    }
}

/// A rectangle of tiles the viewport still wants: columns `x0..x1` and rows `y0..y1`
/// (end exclusive) of one sheet at one zoom level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct TileRange {
    /// Zero based sheet index.
    pub sheet: u32,
    /// Zoom level.
    pub zoom: i32,
    /// First column.
    pub x0: u32,
    /// First row.
    pub y0: u32,
    /// Column after the last one.
    pub x1: u32,
    /// Row after the last one.
    pub y1: u32,
}

impl From<TileRange> for tiles::TileRange {
    fn from(r: TileRange) -> Self {
        Self {
            sheet: r.sheet,
            zoom: r.zoom,
            columns: r.x0..r.x1,
            rows: r.y0..r.y1,
        }
    }
}

fn parse_doc(doc: &str) -> Result<ContentHash, CommandError> {
    ContentHash::from_hex(doc).ok_or_else(|| CommandError::InvalidArgument {
        message: format!("document id {doc:?} is not a 64 digit hex content hash"),
    })
}

/// Returns version, build profile and PDFium availability of the running app.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub fn app_info(tiles: State<'_, TileState>) -> AppInfo {
    AppInfo::current(tiles.service().is_ok())
}

/// Opens a PDF file and registers it for tile rendering. Opening the same file again returns
/// the same document.
///
/// Minimal entry point for T0.7. The file dialog command of T0.8 should replace it, so the
/// webview never names arbitrary paths (NFR-SEC-01).
#[tauri::command]
#[specta::specta]
pub async fn open_document(
    tiles: State<'_, TileState>,
    path: String,
) -> Result<DocumentInfo, CommandError> {
    let service = tiles
        .service()
        .map_err(|message| CommandError::PdfiumUnavailable {
            message: message.to_owned(),
        })?
        .clone();
    // Reading, hashing and parsing block, so they run on the blocking pool.
    let doc = tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| CommandError::Io {
            message: format!("{path}: {e}"),
        })?;
        service
            .open_document(bytes)
            .map_err(|e| CommandError::InvalidDocument {
                message: e.to_string(),
            })
    })
    .await
    .map_err(|e| CommandError::Io {
        message: e.to_string(),
    })??;
    Ok(DocumentInfo {
        doc: doc.content_hash().to_hex(),
        sheets: doc
            .sheet_sizes()
            .iter()
            .map(|s| SheetInfo {
                width: s.width,
                height: s.height,
            })
            .collect(),
    })
}

/// Declares the tiles of a document the viewport still wants. Queued tile requests outside
/// `ranges` are answered with `204 No Content` and not rendered; an empty list cancels every
/// queued request of the document. See "Cancellation" in `dimo_pdf::tiles`.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub fn set_tile_interest(
    tiles: State<'_, TileState>,
    doc: String,
    ranges: Vec<TileRange>,
) -> Result<(), CommandError> {
    let doc = parse_doc(&doc)?;
    if let Ok(service) = tiles.service() {
        service.set_interest(doc, Some(ranges.into_iter().map(Into::into).collect()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_workspace_version_and_profile() {
        let info = AppInfo::current(true);
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        // Tests run with debug assertions unless built with --release.
        assert_eq!(
            info.build_profile == BuildProfile::Debug,
            cfg!(debug_assertions)
        );
        assert!(info.pdfium_available);
        assert!(!AppInfo::current(false).pdfium_available);
    }

    #[test]
    fn tile_address_parses_the_content_hash() {
        let hash = ContentHash::of(b"x");
        let address = TileAddress {
            doc: hash.to_hex(),
            sheet: 1,
            zoom: -2,
            x: 3,
            y: 4,
        };
        let key = address.key().unwrap();
        assert_eq!(key.doc, hash);
        assert_eq!(key.route(), format!("tile/{hash}/1/-2/3/4"));
        let bad = TileAddress {
            doc: "nope".to_owned(),
            ..address
        };
        assert!(matches!(
            bad.key(),
            Err(CommandError::InvalidArgument { .. })
        ));
    }

    #[test]
    fn tile_range_is_end_exclusive() {
        let range: tiles::TileRange = TileRange {
            sheet: 0,
            zoom: 1,
            x0: 2,
            y0: 3,
            x1: 4,
            y1: 5,
        }
        .into();
        assert_eq!(range.columns, 2..4);
        assert_eq!(range.rows, 3..5);
    }
}
