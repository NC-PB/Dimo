//! Error type of the crate.

use std::path::PathBuf;

/// Everything that can go wrong while loading PDFium, opening a document or rendering.
///
/// PDFium errors are carried as text: the render thread owns all PDFium values, and only plain
/// data crosses back to the caller.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PdfError {
    /// No PDFium library was found at any of the searched locations.
    #[error(
        "PDFium library not found (searched: {}). Run ./scripts/fetch-pdfium.sh or set {}",
        display_paths(.searched),
        crate::library::PDFIUM_PATH_ENV
    )]
    LibraryNotFound {
        /// Every path that was checked, in search order.
        searched: Vec<PathBuf>,
    },

    /// The library file exists but could not be loaded or bound.
    #[error("cannot load PDFium from {path}: {message}")]
    LibraryLoad {
        /// The library file that failed to load.
        path: PathBuf,
        /// The loader or binding error.
        message: String,
    },

    /// PDFium could not open the document (damaged file, not a PDF, password protected).
    #[error("cannot open PDF: {0}")]
    Open(String),

    /// The requested sheet does not exist.
    #[error("sheet index {index} out of range, the document has {count} sheet(s)")]
    SheetOutOfRange {
        /// The requested zero based index.
        index: usize,
        /// The number of sheets in the document.
        count: usize,
    },

    /// Region or zoom are not positive finite numbers, or the output would be too large.
    #[error("invalid render request: {0}")]
    InvalidRender(String),

    /// PDFium reported an error while rendering.
    #[error("render failed: {0}")]
    Render(String),

    /// The render thread is gone (it panicked or could not start). Nothing more can be rendered
    /// in this process.
    #[error("PDFium render thread is not running")]
    EngineStopped,
}

fn display_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "nothing".to_owned();
    }
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
