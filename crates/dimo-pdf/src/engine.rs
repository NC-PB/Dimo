//! The PDFium render thread and the document handle.
//!
//! See the crate docs for why all PDFium work runs on one dedicated thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use pdfium_render::prelude::{
    PdfColor, PdfDocument, PdfPage, PdfRenderConfig, Pdfium, PdfiumError,
};

use crate::PdfError;
use crate::geometry::{SheetRect, SheetSize};
use crate::hash::ContentHash;
use crate::library::resolve_library_path;
use crate::overlay::BalloonOverlay;
use crate::page_cache::{PAGES_PER_DOCUMENT, PageCache};
use crate::raster::RgbaImage;
use crate::sheet_kind::{SheetAnalysis, analyze_sheet};
use crate::text::{TextRun, load_page, text_error, text_runs};
use crate::writer::write_ballooned;

/// Largest width or height of one rendered image in pixels. A0 at 300 dpi is about
/// 9933 x 14043 pixels, so this leaves headroom while bounding memory (1 GiB RGBA at most).
pub const MAX_RENDER_SIDE: u32 = 16_384;

/// Extra sheet units rendered left of and above every region, then cut off. 32 units (11 mm)
/// cover glyphs of text up to about 10 mm high (ISO 3098 sizes up to 10). See
/// [`Document::render_region`].
const GLYPH_MARGIN_UNITS: f64 = 32.0;

/// Largest render margin in pixels, so deep zoom levels render at most four times the pixels
/// of a 512 pixel tile. At zoom 4 and 5 only glyphs wider than 512 pixels can still be affected.
const MAX_MARGIN_PX: u32 = 512;

/// Margin in whole pixels for a render at `zoom`.
fn glyph_margin_px(zoom: f64) -> u32 {
    let px = (GLYPH_MARGIN_UNITS * zoom)
        .ceil()
        .min(f64::from(MAX_MARGIN_PX));
    // Between 0 and MAX_MARGIN_PX, so the cast cannot truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = px as u32;
    px
}

/// Copies the `width` x `height` pixels at `(left, top)`. The caller keeps the crop inside.
fn crop(image: &RgbaImage, left: u32, top: u32, width: u32, height: u32) -> RgbaImage {
    let stride = image.width() as usize * 4;
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for row in image
        .as_bytes()
        .chunks_exact(stride)
        .skip(top as usize)
        .take(height as usize)
    {
        out.extend_from_slice(&row[left as usize * 4..(left + width) as usize * 4]);
    }
    RgbaImage::from_raw(width, height, out).unwrap_or_else(|| image.clone())
}

/// The process wide engine. PDFium and the pdfium-render bindings are process global, so there
/// is exactly one render thread per process.
static ENGINE: Mutex<Option<PdfEngine>> = Mutex::new(None);

type Reply<T> = mpsc::Sender<Result<T, PdfError>>;

enum Request {
    Open {
        bytes: Vec<u8>,
        reply: Reply<(u64, Vec<SheetSize>)>,
    },
    Render {
        doc: u64,
        sheet: usize,
        region: SheetRect,
        zoom: f64,
        reply: Reply<RgbaImage>,
    },
    Close {
        doc: u64,
    },
    TextRuns {
        doc: u64,
        sheet: usize,
        reply: Reply<Vec<TextRun>>,
    },
    AnalyzeSheet {
        doc: u64,
        sheet: usize,
        reply: Reply<SheetAnalysis>,
    },
    WriteBallooned {
        original: Vec<u8>,
        overlay: BalloonOverlay,
        reply: Reply<Vec<u8>>,
    },
    /// Page cache counters of a document, `None` if it is not open. Tests only.
    #[cfg(test)]
    PageCacheStats {
        doc: u64,
        reply: mpsc::Sender<Option<crate::page_cache::PageCacheStats>>,
    },
}

/// Handle to the PDFium render thread. Cheap to clone, usable from any thread.
///
/// Every call blocks the calling thread until the render thread has answered. From async code,
/// call it through a blocking task.
#[derive(Clone)]
pub struct PdfEngine {
    tx: mpsc::Sender<Request>,
    library: Arc<PathBuf>,
}

impl std::fmt::Debug for PdfEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfEngine")
            .field("library", &self.library)
            .finish_non_exhaustive()
    }
}

impl PdfEngine {
    /// Starts the render thread with the library found by the default search
    /// (`DIMO_PDFIUM_PATH`, then `vendor/pdfium/`), or returns the running engine.
    pub fn start() -> Result<Self, PdfError> {
        Self::start_impl(None)
    }

    /// Starts the render thread with the given library file or directory, or returns the
    /// running engine. Once an engine runs, later calls return it unchanged, whatever path they
    /// pass: PDFium can be loaded only once per process.
    pub fn start_with_library(path: &Path) -> Result<Self, PdfError> {
        Self::start_impl(Some(path))
    }

    fn start_impl(explicit: Option<&Path>) -> Result<Self, PdfError> {
        // The lock is held while starting so concurrent callers cannot load PDFium twice.
        let mut guard = ENGINE.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(engine) = guard.as_ref() {
            return Ok(engine.clone());
        }
        let path = resolve_library_path(explicit)?;
        let engine = spawn_render_thread(path)?;
        *guard = Some(engine.clone());
        Ok(engine)
    }

    /// The library file this engine loaded.
    pub fn library_path(&self) -> &Path {
        &self.library
    }

    /// Opens a PDF from its bytes. The SHA-256 of the bytes is computed on the calling thread
    /// (FR-DOC-07). The document stays open until the returned handle is dropped.
    pub fn open(&self, bytes: Vec<u8>) -> Result<Document, PdfError> {
        let hash = ContentHash::of(&bytes);
        let (id, sheets) = self.call(|reply| Request::Open { bytes, reply })?;
        Ok(Document {
            id,
            hash,
            sheets,
            engine: self.clone(),
        })
    }

    /// Writes a copy of the PDF `original` with the balloons of `overlay` added as vector
    /// graphics (FR-EXP-01, D-33) and returns the new file. The original bytes are not changed;
    /// documents open in this engine are not affected. Same input, same output bytes
    /// (FR-EXP-11). See [`crate::overlay`] for the primitives.
    pub fn write_ballooned(
        &self,
        original: Vec<u8>,
        overlay: BalloonOverlay,
    ) -> Result<Vec<u8>, PdfError> {
        overlay.validate()?;
        self.call(|reply| Request::WriteBallooned {
            original,
            overlay,
            reply,
        })
    }

    fn call<T>(&self, make: impl FnOnce(Reply<T>) -> Request) -> Result<T, PdfError> {
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(make(reply))
            .map_err(|_| PdfError::EngineStopped)?;
        answer.recv().map_err(|_| PdfError::EngineStopped)?
    }
}

/// An open PDF. Sheets are the pages of the PDF in order, zero based.
///
/// Dropping the handle closes the document on the render thread.
pub struct Document {
    id: u64,
    hash: ContentHash,
    sheets: Vec<SheetSize>,
    engine: PdfEngine,
}

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("hash", &self.hash)
            .field("sheets", &self.sheets)
            .finish_non_exhaustive()
    }
}

impl Document {
    /// SHA-256 of the bytes the document was opened from (FR-DOC-07).
    pub const fn content_hash(&self) -> &ContentHash {
        &self.hash
    }

    /// Number of sheets (pages).
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// Sizes of all sheets in sheet units.
    pub fn sheet_sizes(&self) -> &[SheetSize] {
        &self.sheets
    }

    /// Size of one sheet in sheet units.
    pub fn sheet_size(&self, sheet: usize) -> Result<SheetSize, PdfError> {
        self.sheets
            .get(sheet)
            .copied()
            .ok_or(PdfError::SheetOutOfRange {
                index: sheet,
                count: self.sheets.len(),
            })
    }

    /// Renders `region` of a sheet at `zoom` pixels per sheet unit (zoom 1 is 72 dpi).
    ///
    /// The output is `round(region.width * zoom)` by `round(region.height * zoom)` pixels.
    /// Pixel `(0, 0)` has its top left corner at sheet point `(region.x, region.y)`, so tiles
    /// with whole pixel offsets at the same zoom fit together seamlessly. Parts of the region
    /// outside the sheet are white.
    ///
    /// PDFium (chromium/7881) misplaces anti-aliased glyphs that cross the left or top edge of
    /// the target bitmap: such a glyph differed from a full sheet render by up to 131 of 255
    /// levels at a tile edge of `test_drawing_1.pdf`. Right and bottom edges are not affected,
    /// and the clip rectangle does not help. So the region is rendered with a margin left and
    /// above, which moves the edge away from every glyph that starts at most
    /// [`GLYPH_MARGIN_UNITS`] outside the region, and the margin is cut off again. The margin is
    /// a whole number of pixels, so the pixel grid of the result is unchanged. It is skipped if
    /// the padded image would exceed [`MAX_RENDER_SIDE`].
    pub fn render_region(
        &self,
        sheet: usize,
        region: SheetRect,
        zoom: f64,
    ) -> Result<RgbaImage, PdfError> {
        self.sheet_size(sheet)?;
        let (width, height) = output_size(region, zoom)?;
        let margin = glyph_margin_px(zoom);
        if width + margin > MAX_RENDER_SIDE || height + margin > MAX_RENDER_SIDE {
            return self.render_raw(sheet, region, zoom);
        }
        let pad = f64::from(margin) / zoom;
        let padded = SheetRect::new(
            region.x - pad,
            region.y - pad,
            region.width + pad,
            region.height + pad,
        );
        let image = self.render_raw(sheet, padded, zoom)?;
        Ok(crop(&image, margin, margin, width, height))
    }

    fn render_raw(
        &self,
        sheet: usize,
        region: SheetRect,
        zoom: f64,
    ) -> Result<RgbaImage, PdfError> {
        output_size(region, zoom)?;
        self.engine.call(|reply| Request::Render {
            doc: self.id,
            sheet,
            region,
            zoom,
            reply,
        })
    }
}

impl Document {
    /// The text runs of a sheet in content order, in sheet space (stage 2). See [`TextRun`].
    pub fn text_runs(&self, sheet: usize) -> Result<Vec<TextRun>, PdfError> {
        self.sheet_size(sheet)?;
        self.engine.call(|reply| Request::TextRuns {
            doc: self.id,
            sheet,
            reply,
        })
    }

    /// Classifies a sheet from its content (FR-DOC-03, stage 1). See [`SheetAnalysis`].
    pub fn analyze_sheet(&self, sheet: usize) -> Result<SheetAnalysis, PdfError> {
        self.sheet_size(sheet)?;
        self.engine.call(|reply| Request::AnalyzeSheet {
            doc: self.id,
            sheet,
            reply,
        })
    }
}

#[cfg(test)]
impl Document {
    /// Page cache counters of this document on the render thread.
    fn page_cache_stats(&self) -> Option<crate::page_cache::PageCacheStats> {
        self.engine.page_cache_stats(self.id)
    }
}

#[cfg(test)]
impl PdfEngine {
    fn page_cache_stats(&self, doc: u64) -> Option<crate::page_cache::PageCacheStats> {
        let (reply, answer) = mpsc::channel();
        self.tx.send(Request::PageCacheStats { doc, reply }).ok()?;
        answer.recv().ok()?
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // If the render thread is gone there is nothing left to close.
        let _ = self.engine.tx.send(Request::Close { doc: self.id });
    }
}

/// Validates a render request and returns the output size in pixels.
fn output_size(region: SheetRect, zoom: f64) -> Result<(u32, u32), PdfError> {
    if !(zoom.is_finite() && zoom > 0.0) {
        return Err(PdfError::InvalidRender(format!(
            "zoom must be positive, got {zoom}"
        )));
    }
    let values = [region.x, region.y, region.width, region.height];
    if values.iter().any(|v| !v.is_finite()) || region.width <= 0.0 || region.height <= 0.0 {
        return Err(PdfError::InvalidRender(format!(
            "region must be finite with positive size, got {region:?}"
        )));
    }
    let width = (region.width * zoom).round().max(1.0);
    let height = (region.height * zoom).round().max(1.0);
    let max = f64::from(MAX_RENDER_SIDE);
    if width > max || height > max {
        return Err(PdfError::InvalidRender(format!(
            "output {width} x {height} pixels exceeds {MAX_RENDER_SIDE} per side"
        )));
    }
    // Both values are whole numbers in 1..=MAX_RENDER_SIDE, checked above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok((width as u32, height as u32))
}

fn spawn_render_thread(library: PathBuf) -> Result<PdfEngine, PdfError> {
    let (tx, rx) = mpsc::channel::<Request>();
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), PdfError>>(1);
    let thread_library = library.clone();
    std::thread::Builder::new()
        .name("dimo-pdfium".to_owned())
        .spawn(move || {
            let bindings = match Pdfium::bind_to_library(&thread_library) {
                Ok(bindings) => bindings,
                Err(e) => {
                    let _ = ready_tx.send(Err(PdfError::LibraryLoad {
                        path: thread_library,
                        message: e.to_string(),
                    }));
                    return;
                }
            };
            let pdfium = Pdfium::new(bindings);
            let _ = ready_tx.send(Ok(()));
            serve(&pdfium, &rx);
        })
        .map_err(|e| PdfError::LibraryLoad {
            path: library.clone(),
            message: format!("cannot start render thread: {e}"),
        })?;
    ready_rx.recv().map_err(|_| PdfError::EngineStopped)??;
    tracing::debug!(library = %library.display(), "PDFium render thread started");
    Ok(PdfEngine {
        tx,
        library: Arc::new(library),
    })
}

/// An open document and its most recently used loaded pages (T1.11, see `page_cache`).
struct OpenDoc<'a> {
    pages: PageCache<PdfPage<'a>>,
    doc: PdfDocument<'a>,
}

impl<'a> OpenDoc<'a> {
    fn new(doc: PdfDocument<'a>) -> Self {
        Self {
            pages: PageCache::new(PAGES_PER_DOCUMENT),
            doc,
        }
    }

    /// The loaded page of `sheet`, from the cache or loaded now. A load failure is mapped with
    /// `error`, so render and text requests keep their error kinds.
    fn page(
        &mut self,
        sheet: usize,
        error: fn(PdfiumError) -> PdfError,
    ) -> Result<&PdfPage<'a>, PdfError> {
        let doc = &self.doc;
        self.pages
            .get_or_load(sheet, || load_page(doc, sheet, error))
    }
}

impl Drop for OpenDoc<'_> {
    fn drop(&mut self) {
        // Every page must be closed before its document (FPDF_ClosePage before
        // FPDF_CloseDocument). Fields drop after this, `doc` last.
        self.pages.clear();
    }
}

/// The render thread loop. Owns every PDFium value; nothing PDFium related leaves this thread.
fn serve(pdfium: &Pdfium, rx: &mpsc::Receiver<Request>) {
    let mut docs: HashMap<u64, OpenDoc<'_>> = HashMap::new();
    let mut next_id: u64 = 0;
    while let Ok(request) = rx.recv() {
        match request {
            Request::Open { bytes, reply } => {
                let result = pdfium
                    .load_pdf_from_byte_vec(bytes, None)
                    .map_err(|e| PdfError::Open(e.to_string()))
                    .map(|doc| {
                        let sheets = sheet_sizes(&doc);
                        let id = next_id;
                        next_id += 1;
                        docs.insert(id, OpenDoc::new(doc));
                        tracing::debug!(id, sheets = sheets.len(), "PDF opened");
                        (id, sheets)
                    });
                let _ = reply.send(result);
            }
            Request::Render {
                doc,
                sheet,
                region,
                zoom,
                reply,
            } => {
                let result = match docs.get_mut(&doc) {
                    Some(open) => open
                        .page(sheet, render_error)
                        .and_then(|page| render(page, region, zoom)),
                    None => Err(PdfError::Render(format!("document {doc} is not open"))),
                };
                let _ = reply.send(result);
            }
            Request::Close { doc } => {
                if let Some(open) = docs.remove(&doc) {
                    let stats = open.pages.stats();
                    tracing::debug!(doc, hits = stats.hits, misses = stats.misses, "PDF closed");
                }
            }
            Request::TextRuns { doc, sheet, reply } => {
                let _ = reply.send(with_page(&mut docs, doc, sheet, text_runs));
            }
            Request::AnalyzeSheet { doc, sheet, reply } => {
                let _ = reply.send(with_page(&mut docs, doc, sheet, |page| {
                    analyze_sheet(page, sheet)
                }));
            }
            Request::WriteBallooned {
                original,
                overlay,
                reply,
            } => {
                let _ = reply.send(write_ballooned(pdfium, &original, &overlay));
            }
            #[cfg(test)]
            Request::PageCacheStats { doc, reply } => {
                let _ = reply.send(docs.get(&doc).map(|open| open.pages.stats()));
            }
        }
    }
}

/// Runs `f` on the cached page of a sheet for the text and analysis requests.
fn with_page<T>(
    docs: &mut HashMap<u64, OpenDoc<'_>>,
    doc: u64,
    sheet: usize,
    f: impl FnOnce(&PdfPage<'_>) -> Result<T, PdfError>,
) -> Result<T, PdfError> {
    docs.get_mut(&doc).map_or_else(
        || Err(PdfError::Text(format!("document {doc} is not open"))),
        |open| open.page(sheet, text_error).and_then(f),
    )
}

/// Sizes of all pages. `FPDF_GetPageSizeByIndexF` reads the page boxes and /Rotate without
/// loading the page, so opening a project does not parse the content of every sheet
/// (T0.9: 2.0 s instead of 0.2 s for 50 dense A0 sheets). The size is the same one
/// `PdfPage::width` reports. If PDFium refuses, the page is loaded as a fallback.
fn sheet_sizes(doc: &PdfDocument<'_>) -> Vec<SheetSize> {
    let pages = doc.pages();
    pages
        .as_range()
        .map(|index| match pages.page_size(index) {
            Ok(rect) => SheetSize {
                width: f64::from(rect.width().value),
                height: f64::from(rect.height().value),
            },
            Err(_) => pages.get(index).map_or(
                SheetSize {
                    width: 0.0,
                    height: 0.0,
                },
                |page| SheetSize {
                    width: f64::from(page.width().value),
                    height: f64::from(page.height().value),
                },
            ),
        })
        .collect()
}

fn render(page: &PdfPage<'_>, region: SheetRect, zoom: f64) -> Result<RgbaImage, PdfError> {
    let (width, height) = output_size(region, zoom)?;

    // FPDF_RenderPageBitmapWithMatrix maps the displayed page to device space with one unit per
    // pixel, origin top left, y downward (crop box and /Rotate applied): that is sheet space.
    // Our matrix scales by `zoom` and moves the region origin to pixel (0, 0). Checked by the
    // sheet_space_* tests, including a sheet with a fractional size.
    // Matrix values are f32 in PDFium; this is screen geometry only.
    #[allow(clippy::cast_possible_truncation)]
    let matrix = (
        zoom as f32,
        zoom as f32,
        (-region.x * zoom) as f32,
        (-region.y * zoom) as f32,
    );
    // Both sides are at most MAX_RENDER_SIDE, which fits an i32.
    #[allow(clippy::cast_possible_wrap)]
    let config = PdfRenderConfig::new()
        .set_fixed_size(width as i32, height as i32)
        .set_clear_color(PdfColor::WHITE)
        .render_annotations(true)
        .use_lcd_text_rendering(false)
        .transform(matrix.0, 0.0, 0.0, matrix.1, matrix.2, matrix.3)
        .map_err(render_error)?;
    let bitmap = page.render_with_config(&config).map_err(render_error)?;
    let bytes = bitmap.as_rgba_bytes();
    to_packed_rgba(width, height, bytes)
}

/// PDFium may pad rows; returns a tightly packed buffer.
fn to_packed_rgba(width: u32, height: u32, bytes: Vec<u8>) -> Result<RgbaImage, PdfError> {
    let row = width as usize * 4;
    let rows = height as usize;
    if bytes.len() == row * rows {
        return RgbaImage::from_raw(width, height, bytes)
            .ok_or_else(|| PdfError::Render("bitmap size mismatch".to_owned()));
    }
    let stride = bytes.len() / rows.max(1);
    if stride < row {
        return Err(PdfError::Render(format!(
            "bitmap stride {stride} smaller than row {row}"
        )));
    }
    let mut packed = Vec::with_capacity(row * rows);
    for chunk in bytes.chunks(stride).take(rows) {
        packed.extend_from_slice(&chunk[..row]);
    }
    RgbaImage::from_raw(width, height, packed)
        .ok_or_else(|| PdfError::Render("bitmap size mismatch".to_owned()))
}

#[allow(clippy::needless_pass_by_value)] // used as a map_err callback
fn render_error(e: PdfiumError) -> PdfError {
    PdfError::Render(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_size_rounds() {
        let r = SheetRect::new(10.0, 20.0, 256.0, 100.4);
        assert_eq!(output_size(r, 1.0).unwrap(), (256, 100));
        assert_eq!(output_size(r, 0.5).unwrap(), (128, 50));
        // Tiny regions still produce one pixel.
        assert_eq!(
            output_size(SheetRect::new(0.0, 0.0, 0.1, 0.1), 1.0).unwrap(),
            (1, 1)
        );
    }

    #[test]
    fn output_size_rejects_bad_input() {
        let r = SheetRect::new(0.0, 0.0, 100.0, 100.0);
        assert!(output_size(r, 0.0).is_err());
        assert!(output_size(r, f64::NAN).is_err());
        assert!(output_size(r, -1.0).is_err());
        assert!(output_size(SheetRect::new(0.0, 0.0, -1.0, 10.0), 1.0).is_err());
        assert!(output_size(SheetRect::new(f64::INFINITY, 0.0, 1.0, 10.0), 1.0).is_err());
        assert!(output_size(r, 200.0).is_err());
    }

    #[test]
    fn packs_padded_rows() {
        // 1 x 2 image with 8 byte stride.
        let bytes = vec![1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8, 0, 0, 0, 0];
        let img = to_packed_rgba(1, 2, bytes).unwrap();
        assert_eq!(img.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }
}

#[cfg(test)]
mod margin_tests {
    use super::glyph_margin_px;

    #[test]
    fn glyph_margin_grows_with_zoom_up_to_a_tile() {
        let margins: Vec<u32> = (-4..=5).map(|z| glyph_margin_px(2f64.powi(z))).collect();
        assert_eq!(margins, vec![2, 4, 8, 16, 32, 64, 128, 256, 512, 512]);
    }
}
/// Page cache on the real render thread (T1.11). Skips like `tests/pdfium.rs` when PDFium is
/// missing, unless `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.
#[cfg(test)]
mod page_cache_tests {
    #![allow(clippy::print_stderr, clippy::unwrap_used)]

    use super::*;
    use crate::page_cache::PAGES_PER_DOCUMENT;

    fn engine() -> Option<PdfEngine> {
        let required = std::env::var("CI").is_ok_and(|v| v == "true")
            || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0");
        match PdfEngine::start() {
            Ok(engine) => Some(engine),
            Err(e @ PdfError::LibraryNotFound { .. }) if !required => {
                eprintln!("SKIPPED (PDFium missing): {e}");
                None
            }
            Err(e) => panic!("{e}"),
        }
    }

    /// A PDF with `pages` pages of 100 x 100 units. Page `i` has a black square of `10 + i`
    /// units at the top left and the text `P<i>`.
    fn pdf(pages: usize) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            String::new(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        ];
        let mut kids = Vec::new();
        for i in 0..pages {
            let side = 10 + i;
            let content = format!(
                "0 g 0 {} {side} {side} re f BT /F1 12 Tf 50 50 Td (P{i}) Tj ET",
                100 - side
            );
            let page_id = objects.len() + 1;
            kids.push(format!("{page_id} 0 R"));
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents {} 0 R \
                 /Resources << /Font << /F1 3 0 R >> >> >>",
                page_id + 1
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            kids.join(" ")
        );
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
        );
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    fn render_sheet(doc: &Document, sheet: usize) -> RgbaImage {
        doc.render_region(sheet, SheetRect::new(0.0, 0.0, 100.0, 100.0), 1.0)
            .unwrap()
    }

    /// Width of the black square in the top row, which tells the sheets apart.
    fn square_side(img: &RgbaImage) -> usize {
        (0..img.width())
            .take_while(|&x| img.pixel(x, 0).is_some_and(|p| p[0] < 128))
            .count()
    }

    #[test]
    fn repeated_renders_of_a_sheet_hit_the_cache() {
        let Some(engine) = engine() else { return };
        let doc = engine.open(pdf(3)).unwrap();
        assert_eq!(doc.page_cache_stats().unwrap().sheets, Vec::<usize>::new());
        let first = render_sheet(&doc, 1);
        let again = render_sheet(&doc, 1);
        assert_eq!(first, again);
        assert_eq!(square_side(&first), 11);
        let stats = doc.page_cache_stats().unwrap();
        assert_eq!(stats.sheets, vec![1]);
        assert_eq!((stats.hits, stats.misses), (1, 1));
        // Text runs and analysis use the same cached page.
        let runs = doc.text_runs(1).unwrap();
        assert_eq!(runs.first().map(|r| r.text.as_str()), Some("P1"));
        doc.analyze_sheet(1).unwrap();
        let stats = doc.page_cache_stats().unwrap();
        assert_eq!((stats.hits, stats.misses), (3, 1));
    }

    #[test]
    fn least_recently_used_sheet_is_evicted() {
        let Some(engine) = engine() else { return };
        let sheets = PAGES_PER_DOCUMENT + 2;
        let doc = engine.open(pdf(sheets)).unwrap();
        for sheet in 0..sheets {
            assert_eq!(square_side(&render_sheet(&doc, sheet)), 10 + sheet);
        }
        let stats = doc.page_cache_stats().unwrap();
        let expected: Vec<usize> = (0..sheets).rev().take(PAGES_PER_DOCUMENT).collect();
        assert_eq!(stats.sheets, expected);
        assert_eq!((stats.hits, stats.misses), (0, sheets as u64));
        // Sheet 0 was evicted: it loads again and still renders correctly.
        assert_eq!(square_side(&render_sheet(&doc, 0)), 10);
        let stats = doc.page_cache_stats().unwrap();
        assert_eq!(stats.sheets.first(), Some(&0));
        assert_eq!(stats.sheets.len(), PAGES_PER_DOCUMENT);
        assert_eq!(stats.misses, sheets as u64 + 1);
    }

    #[test]
    fn documents_have_separate_caches() {
        let Some(engine) = engine() else { return };
        let a = engine.open(pdf(2)).unwrap();
        let b = engine.open(pdf(2)).unwrap();
        render_sheet(&a, 0);
        render_sheet(&b, 1);
        assert_eq!(a.page_cache_stats().unwrap().sheets, vec![0]);
        assert_eq!(b.page_cache_stats().unwrap().sheets, vec![1]);
    }

    #[test]
    fn close_drops_the_cached_pages() {
        let Some(engine) = engine() else { return };
        let doc = engine.open(pdf(2)).unwrap();
        render_sheet(&doc, 0);
        render_sheet(&doc, 1);
        let id = doc.id;
        assert_eq!(engine.page_cache_stats(id).unwrap().sheets.len(), 2);
        drop(doc);
        // The close is handled before the stats request: one queue, in order.
        assert_eq!(engine.page_cache_stats(id), None);
        // The engine still works after closing a document with cached pages.
        let other = engine.open(pdf(1)).unwrap();
        assert_eq!(square_side(&render_sheet(&other, 0)), 10);
    }

    #[test]
    fn failed_page_load_is_reported_and_not_cached() {
        let Some(engine) = engine() else { return };
        let doc = engine.open(pdf(1)).unwrap();
        // Bypasses the range check of `Document` to reach the render thread.
        let result = engine.call(|reply| Request::Render {
            doc: doc.id,
            sheet: 5,
            region: SheetRect::new(0.0, 0.0, 10.0, 10.0),
            zoom: 1.0,
            reply,
        });
        assert!(matches!(result, Err(PdfError::SheetOutOfRange { .. })));
        assert_eq!(doc.page_cache_stats().unwrap().sheets, Vec::<usize>::new());
    }
}
