//! The PDFium render thread and the document handle.
//!
//! See the crate docs for why all PDFium work runs on one dedicated thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use pdfium_render::prelude::{
    PdfColor, PdfDocument, PdfPageIndex, PdfRenderConfig, Pdfium, PdfiumError,
};

use crate::PdfError;
use crate::geometry::{SheetRect, SheetSize};
use crate::hash::ContentHash;
use crate::library::resolve_library_path;
use crate::raster::RgbaImage;
use crate::sheet_kind::{SheetAnalysis, analyze_sheet};
use crate::text::{TextRun, text_runs};

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

/// The render thread loop. Owns every PDFium value; nothing PDFium related leaves this thread.
fn serve(pdfium: &Pdfium, rx: &mpsc::Receiver<Request>) {
    let mut docs: HashMap<u64, PdfDocument<'_>> = HashMap::new();
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
                        docs.insert(id, doc);
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
                let result = match docs.get(&doc) {
                    Some(document) => render(document, sheet, region, zoom),
                    None => Err(PdfError::Render(format!("document {doc} is not open"))),
                };
                let _ = reply.send(result);
            }
            Request::Close { doc } => {
                docs.remove(&doc);
            }
            Request::TextRuns { doc, sheet, reply } => {
                let _ = reply.send(with_doc(&docs, doc, |d| text_runs(d, sheet)));
            }
            Request::AnalyzeSheet { doc, sheet, reply } => {
                let _ = reply.send(with_doc(&docs, doc, |d| analyze_sheet(d, sheet)));
            }
        }
    }
}

fn with_doc<'a, T>(
    docs: &HashMap<u64, PdfDocument<'a>>,
    doc: u64,
    f: impl FnOnce(&PdfDocument<'a>) -> Result<T, PdfError>,
) -> Result<T, PdfError> {
    docs.get(&doc).map_or_else(
        || Err(PdfError::Text(format!("document {doc} is not open"))),
        f,
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

fn render(
    doc: &PdfDocument<'_>,
    sheet: usize,
    region: SheetRect,
    zoom: f64,
) -> Result<RgbaImage, PdfError> {
    let (width, height) = output_size(region, zoom)?;
    let index = PdfPageIndex::try_from(sheet).map_err(|_| PdfError::SheetOutOfRange {
        index: sheet,
        count: usize::try_from(doc.pages().len()).unwrap_or(0),
    })?;
    let page = doc.pages().get(index).map_err(render_error)?;

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
