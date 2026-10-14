//! Raster sources: anything the tile service can draw a sheet from (MF F1, ADR 0010).
//!
//! The viewport, the tile caches and project creation see a drawing only through
//! [`RasterSource`]: how many sheets, how big each sheet is in sheet space (rule 4: units of
//! 1/72 inch, origin top left), and the pixels of a region at a zoom. A PDF [`Document`]
//! implements it today; image drawings (release 0.1, R.19) add a second implementation
//! without changes to the tile service.
//!
//! [`RasterOpener`] opens a drawing file as a source. The tile service implements it, so a
//! source it opened is ready for tiles, and so does the [`PdfEngine`] for tools without
//! tiles. `dimo-io` creates projects through it and never calls PDFium itself.
//!
//! Errors of every source use [`PdfError`] (`SheetOutOfRange`, `InvalidRender`, `Render`).

use std::fmt;
use std::sync::Arc;

use dimo_core::{Rect, Sha256Hex, SheetKind, Size};

use crate::engine::{Document, PdfEngine};
use crate::error::PdfError;
use crate::raster::RgbaImage;

/// The kind of a source. Part of every tile cache key, so tiles of a PDF and of an image with
/// the same bytes never mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceKind {
    /// A PDF document rendered by PDFium.
    Pdf,
    /// A photo or scan.
    Image,
}

impl SourceKind {
    /// Short lowercase name, used in cache paths.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Image => "image",
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A drawing the tile service can render: sheets with a size in sheet space and their pixels.
///
/// Implementations must be cheap to share across threads; the tile workers call
/// [`RasterSource::render_region`] in parallel.
pub trait RasterSource: Send + Sync + fmt::Debug {
    /// What kind of source this is.
    fn kind(&self) -> SourceKind;

    /// SHA-256 of the file the source was opened from (FR-DOC-07). Names the source in tile
    /// URLs and caches.
    fn content_hash(&self) -> &Sha256Hex;

    /// Identity of the pixels this source draws. It names the source in tile URLs, in the
    /// tile service and in both tile caches. The default is the content hash, right for a PDF.
    /// A source whose pixels also depend on settings (an image with its edit settings, R.19)
    /// must derive it from the hash and those settings, so changed settings never get old
    /// tiles.
    fn render_key(&self) -> Sha256Hex {
        *self.content_hash()
    }

    /// Number of sheets.
    fn sheet_count(&self) -> usize;

    /// Size of a sheet in sheet units, unrotated (rule 4).
    fn sheet_size(&self, sheet: usize) -> Result<Size, PdfError>;

    /// Renders `region` of a sheet (sheet space) at `zoom` pixels per sheet unit. The output is
    /// `round(region.width * zoom)` by `round(region.height * zoom)` pixels; pixel `(0, 0)` has
    /// its top left corner at `(region.x, region.y)`. Parts outside the sheet are white.
    fn render_region(&self, sheet: usize, region: Rect, zoom: f64) -> Result<RgbaImage, PdfError>;

    /// How the text of a sheet is stored, which decides the recognition path (FR-DOC-03).
    /// Images are always [`SheetKind::Raster`].
    fn sheet_kind(&self, sheet: usize) -> Result<SheetKind, PdfError>;

    /// Sizes of all sheets in order.
    fn sheet_sizes(&self) -> Result<Vec<Size>, PdfError> {
        (0..self.sheet_count())
            .map(|sheet| self.sheet_size(sheet))
            .collect()
    }
}

impl RasterSource for Document {
    fn kind(&self) -> SourceKind {
        SourceKind::Pdf
    }

    fn content_hash(&self) -> &Sha256Hex {
        Document::content_hash(self)
    }

    fn sheet_count(&self) -> usize {
        Document::sheet_count(self)
    }

    fn sheet_size(&self, sheet: usize) -> Result<Size, PdfError> {
        Document::sheet_size(self, sheet)
    }

    fn render_region(&self, sheet: usize, region: Rect, zoom: f64) -> Result<RgbaImage, PdfError> {
        Document::render_region(self, sheet, region, zoom)
    }

    fn sheet_kind(&self, sheet: usize) -> Result<SheetKind, PdfError> {
        Ok(self.analyze_sheet(sheet)?.kind)
    }
}

/// A source handed out by a [`RasterOpener`].
#[derive(Debug, Clone)]
pub struct Opened {
    /// The source.
    pub source: Arc<dyn RasterSource>,
    /// True if this call opened it; false if the opener already had it open (for example the
    /// drawing of the open project). Only a newly opened source may be released again.
    pub newly_opened: bool,
}

/// Opens drawing files as raster sources (MF F1). Image formats follow in R.19.
pub trait RasterOpener {
    /// Opens a PDF from its bytes.
    fn open_pdf(&self, bytes: Vec<u8>) -> Result<Opened, PdfError>;

    /// Gives up a source this opener opened and that is not used after all, for example when
    /// creating a project from it failed. `key` is its [`RasterSource::render_key`]. Does
    /// nothing for sources it does not hold.
    fn release(&self, key: &Sha256Hex);
}

/// Opens PDFs without registering them anywhere, for tools and tests without tiles.
impl RasterOpener for PdfEngine {
    fn open_pdf(&self, bytes: Vec<u8>) -> Result<Opened, PdfError> {
        Ok(Opened {
            source: Arc::new(self.open(bytes)?),
            newly_opened: true,
        })
    }

    fn release(&self, _key: &Sha256Hex) {}
}

#[cfg(test)]
mod tests {
    use super::SourceKind;

    #[test]
    fn kinds_have_distinct_names() {
        assert_eq!(SourceKind::Pdf.to_string(), "pdf");
        assert_eq!(SourceKind::Image.as_str(), "image");
    }
}
