//! Photos and scans as drawings: the image [`RasterSource`] (R.19, ADR 0010).
//!
//! An [`ImageSource`] draws the sheets of a stored JPEG, PNG or TIFF with the edit settings of
//! each page ([`ImageEdit`]). Opening decodes every page once (which checks the whole file and
//! gives the page sizes); afterwards at most [`PAGE_CACHE`] pages are kept in memory and others
//! are decoded again when a tile needs them.
//!
//! # Sheet size
//!
//! - With a paper set (R01 decision 9) the sheet is the paper.
//! - Without one, a resolution the file states between [`MIN_SCAN_DPI`] and [`MAX_SCAN_DPI`]
//!   gives the size (a scan). Otherwise (a phone photo says 72 dpi or nothing) the long side is
//!   297 mm, the length of an A4 sheet, and the aspect ratio is kept.
//!
//! The page is then reduced to at most [`MAX_DPI`] at the sheet size (R01 decision 9) by
//! averaging square blocks of whole pixels, so the same file always gives the same pixels.
//!
//! # Edits
//!
//! Quarter turns and the paper apply now. Fine angle, corner correction, trim and clean up
//! come with the image editor (R.20); a page that uses them is refused until then.

mod decode;

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use dimo_core::{ImageEdit, ImageFormat, Rect, Rotation, Sha256Hex, SheetKind, Size, UNITS_PER_MM};
use image::DynamicImage;

use crate::engine::output_size;
use crate::error::PdfError;
use crate::raster::RgbaImage;
use crate::source::{RasterSource, SourceKind};

pub use decode::{MAX_DECODE_BYTES, MAX_PAGES};

/// Highest resolution a page is kept at, at its sheet size (R01 decision 9).
pub const MAX_DPI: f64 = 300.0;

/// Lowest stated resolution taken as the real size of a scan.
pub const MIN_SCAN_DPI: f64 = 100.0;

/// Highest stated resolution taken as the real size of a scan.
pub const MAX_SCAN_DPI: f64 = 1200.0;

/// Long side of a sheet whose size the file does not tell, in mm (A4).
pub const DEFAULT_LONG_SIDE_MM: f64 = 297.0;

/// Pages kept decoded in memory per source.
pub const PAGE_CACHE: usize = 2;

/// Smallest long side of a reduction level; below it no further level is made.
const MIN_LEVEL_SIDE: u32 = 64;

/// Version of the pixels an image source draws, part of every render key. Raise it whenever
/// decoding, reduction or rendering gives other pixels for the same file and settings, so the
/// tile disk cache never serves tiles of the old code.
const PIXELS_VERSION: &[u8] = b"dimo image source 2\n";

/// Sheet units per inch.
const UNITS_PER_INCH: f64 = 72.0;

/// A photo or scan with the edit settings of its pages.
pub struct ImageSource {
    bytes: Arc<[u8]>,
    format: ImageFormat,
    content_hash: Sha256Hex,
    render_key: Sha256Hex,
    pages: Vec<PageLayout>,
    /// Recently used pages, most recent last. Each entry is filled once outside the lock, so a
    /// page that is decoding does not block tiles of other pages.
    cache: Mutex<Vec<(usize, Arc<PageCell>)>>,
}

/// A page being decoded or decoded, shared by the tile workers that need it.
type PageCell = OnceLock<Result<Arc<Pyramid>, String>>;

/// How one page becomes a sheet.
#[derive(Debug, Clone)]
struct PageLayout {
    rotation: Rotation,
    /// Sheet size in sheet units, after quarter turns.
    sheet: Size,
    /// Block size of the reduction (1 keeps every pixel).
    block: u32,
    /// Effective resolution of the kept pixels at the sheet size.
    dpi: u32,
    /// Index of the TIFF directory (or 0) the page is read from; thumbnails are skipped.
    directory: usize,
}

/// Pixels of a page, upright (before quarter turns), 1 or 3 channels.
struct Level {
    width: u32,
    height: u32,
    channels: usize,
    pixels: Vec<u8>,
}

/// A page reduced to at most [`MAX_DPI`] (level 0) and its halvings, so zoomed out tiles
/// average every source pixel instead of skipping thin lines.
struct Pyramid {
    levels: Vec<Level>,
}

impl ImageSource {
    /// Opens an image from its bytes. `edits` holds one entry per page in page order; `None`
    /// uses [`ImageEdit::default`] for every page (a new drawing). Decodes every page once.
    pub fn open(
        bytes: Vec<u8>,
        format: ImageFormat,
        edits: Option<&[ImageEdit]>,
    ) -> Result<Self, PdfError> {
        // One page at a time: each full size page is dropped before the next is decoded, so
        // opening needs the memory of one page, not of all of them.
        let default = ImageEdit::default();
        let mut layouts = Vec::new();
        let mut first = None;
        decode::for_each_page(&bytes, format, |directory, page| {
            let edit = match edits {
                Some(edits) => edits
                    .get(layouts.len())
                    .ok_or_else(|| edit_count(edits.len()))?,
                None => &default,
            };
            let layout = layout(&page, edit, directory)?;
            if first.is_none() {
                first = Some(Arc::new(pyramid(&page.image, layout.block)));
            }
            layouts.push(layout);
            Ok(())
        })?;
        if let Some(edits) = edits
            && edits.len() != layouts.len()
        {
            return Err(edit_count(edits.len()));
        }
        let content_hash = Sha256Hex::of(&bytes);
        let all_default = vec![ImageEdit::default(); layouts.len()];
        let edits = edits.unwrap_or(&all_default);
        let render_key = render_key(&content_hash, edits)?;
        tracing::debug!(
            pages = layouts.len(),
            format = format.extension(),
            "image drawing opened"
        );
        Ok(Self {
            bytes: bytes.into(),
            format,
            content_hash,
            render_key,
            pages: layouts,
            cache: Mutex::new(
                first
                    .map(|p| (0, Arc::new(OnceLock::from(Ok(p)))))
                    .into_iter()
                    .collect(),
            ),
        })
    }

    /// The render key an image with these bytes and edit settings gets, without decoding it,
    /// so an opener can find a source that is already open.
    pub fn render_key_of(bytes: &[u8], edits: &[ImageEdit]) -> Result<Sha256Hex, PdfError> {
        render_key(&Sha256Hex::of(bytes), edits)
    }

    /// The file format.
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    fn page(&self, sheet: usize) -> Result<&PageLayout, PdfError> {
        self.pages.get(sheet).ok_or(PdfError::SheetOutOfRange {
            index: sheet,
            count: self.pages.len(),
        })
    }

    /// The reduced pixels of a page, from the cache or decoded again. The cache lock is held
    /// only to find or add the page's cell; the decode runs outside it, and workers that need
    /// the same page wait for that one decode.
    fn reduced(&self, sheet: usize) -> Result<Arc<Pyramid>, PdfError> {
        let layout = self.page(sheet)?;
        let cell = {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            let entry = match cache.iter().position(|(i, _)| *i == sheet) {
                Some(at) => cache.remove(at),
                None => (sheet, Arc::new(PageCell::new())),
            };
            let cell = Arc::clone(&entry.1);
            cache.push(entry);
            if cache.len() > PAGE_CACHE {
                cache.remove(0);
            }
            cell
        };
        cell.get_or_init(|| {
            decode::decode_page(&self.bytes, self.format, layout.directory)
                .map(|page| Arc::new(pyramid(&page.image, layout.block)))
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(PdfError::Image)
    }
}

impl std::fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageSource")
            .field("format", &self.format)
            .field("content_hash", &self.content_hash)
            .field("render_key", &self.render_key)
            .field("pages", &self.pages)
            .finish_non_exhaustive()
    }
}

impl RasterSource for ImageSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Image
    }

    fn content_hash(&self) -> &Sha256Hex {
        &self.content_hash
    }

    fn render_key(&self) -> Sha256Hex {
        self.render_key
    }

    fn sheet_count(&self) -> usize {
        self.pages.len()
    }

    fn sheet_size(&self, sheet: usize) -> Result<Size, PdfError> {
        Ok(self.page(sheet)?.sheet)
    }

    fn render_region(&self, sheet: usize, region: Rect, zoom: f64) -> Result<RgbaImage, PdfError> {
        let layout = self.page(sheet)?.clone();
        let (width, height) = output_size(region, zoom)?;
        let page = self.reduced(sheet)?;
        render(&page, &layout, region, zoom, width, height)
    }

    fn sheet_kind(&self, sheet: usize) -> Result<SheetKind, PdfError> {
        self.page(sheet)?;
        Ok(SheetKind::Raster)
    }

    fn raster_dpi(&self, sheet: usize) -> Option<u32> {
        self.pages.get(sheet).map(|p| p.dpi)
    }
}

/// The render key: the content hash and the edit settings of every page (ADR 0010).
fn render_key(content_hash: &Sha256Hex, edits: &[ImageEdit]) -> Result<Sha256Hex, PdfError> {
    let settings = serde_json::to_vec(edits)
        .map_err(|e| PdfError::Image(format!("cannot write the edit settings: {e}")))?;
    let mut input = PIXELS_VERSION.to_vec();
    input.extend_from_slice(content_hash.to_hex().as_bytes());
    input.push(b'\n');
    input.extend_from_slice(&settings);
    Ok(Sha256Hex::of(&input))
}

fn edit_count(edits: usize) -> PdfError {
    PdfError::ImageEdit(format!(
        "{edits} edit setting(s) do not match the pages of the image"
    ))
}

/// Sheet size, reduction and resolution of a page with its edit.
fn layout(page: &decode::Page, edit: &ImageEdit, directory: usize) -> Result<PageLayout, PdfError> {
    if !edit.fine_angle.is_zero() || edit.corners.is_some() || edit.trim.is_some() || edit.clean_up
    {
        return Err(PdfError::ImageEdit(
            "fine rotation, corner correction, trim and clean up are not supported yet".to_owned(),
        ));
    }
    let (width, height) = page.image.dimensions_f64();
    let turned = matches!(edit.rotation, Rotation::Deg90 | Rotation::Deg270);
    let upright = match page.dpi {
        Some((x, y)) if is_scan_dpi(x) && is_scan_dpi(y) => Size {
            width: width / x * UNITS_PER_INCH,
            height: height / y * UNITS_PER_INCH,
        },
        _ => {
            let long = DEFAULT_LONG_SIDE_MM * UNITS_PER_MM;
            let scale = long / width.max(height);
            Size {
                width: width * scale,
                height: height * scale,
            }
        }
    };
    let sheet = match &edit.paper {
        Some(paper) => paper.sheet_size(),
        None if turned => Size {
            width: upright.height,
            height: upright.width,
        },
        None => upright,
    };
    // Pixels across the sheet as it reads, after the quarter turns.
    let (across, down) = if turned {
        (height, width)
    } else {
        (width, height)
    };
    let max_across = sheet.width / UNITS_PER_INCH * MAX_DPI;
    let max_down = sheet.height / UNITS_PER_INCH * MAX_DPI;
    let block = to_u32((across / max_across).max(down / max_down).ceil()).max(1);
    let kept = (across / f64::from(block)).ceil();
    Ok(PageLayout {
        rotation: edit.rotation,
        sheet,
        block,
        dpi: to_u32((kept / (sheet.width / UNITS_PER_INCH)).round()),
        directory,
    })
}

/// A whole number of 0 or more as `u32`, saturating; 0 for NaN.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_u32(value: f64) -> u32 {
    // Float to int `as` casts saturate and turn NaN into 0.
    value as u32
}

fn is_scan_dpi(dpi: f64) -> bool {
    (MIN_SCAN_DPI..=MAX_SCAN_DPI).contains(&dpi)
}

trait DimensionsF64 {
    fn dimensions_f64(&self) -> (f64, f64);
}

impl DimensionsF64 for DynamicImage {
    fn dimensions_f64(&self) -> (f64, f64) {
        (f64::from(self.width()), f64::from(self.height()))
    }
}

/// Level 0 reduced by `block`, then halvings down to [`MIN_LEVEL_SIDE`].
fn pyramid(image: &DynamicImage, block: u32) -> Pyramid {
    let channels = if matches!(image, DynamicImage::ImageLuma8(_)) {
        1
    } else {
        3
    };
    let base = Level {
        width: image.width(),
        height: image.height(),
        channels,
        pixels: Vec::new(),
    };
    let first = if block <= 1 {
        Level {
            pixels: image.as_bytes().to_vec(),
            ..base
        }
    } else {
        reduce(&base, image.as_bytes(), block)
    };
    let mut levels = vec![first];
    while let Some(last) = levels.last()
        && last.width.max(last.height) > MIN_LEVEL_SIDE
    {
        let next = reduce(last, &last.pixels, 2);
        levels.push(next);
    }
    Pyramid { levels }
}

/// Averages square blocks of `block` pixels of `src` (laid out as `shape`; partial blocks at
/// the edges over the pixels they have), in integers.
fn reduce(shape: &Level, src: &[u8], block: u32) -> Level {
    let (width, height, channels) = (shape.width, shape.height, shape.channels);
    let out_w = width.div_ceil(block);
    let out_h = height.div_ceil(block);
    let mut pixels = Vec::with_capacity(out_w as usize * out_h as usize * channels);
    let mut sums = vec![0u64; out_w as usize * channels];
    let mut counts = vec![0u64; out_w as usize];
    for by in 0..out_h {
        sums.fill(0);
        counts.fill(0);
        let rows = by * block..((by + 1) * block).min(height);
        for y in rows {
            let row = &src[(y * width) as usize * channels..((y + 1) * width) as usize * channels];
            for x in 0..width as usize {
                let bx = x / block as usize;
                counts[bx] += 1;
                for c in 0..channels {
                    sums[bx * channels + c] += u64::from(row[x * channels + c]);
                }
            }
        }
        for bx in 0..out_w as usize {
            let n = counts[bx].max(1);
            for c in 0..channels {
                pixels.push(u8::try_from((sums[bx * channels + c] + n / 2) / n).unwrap_or(u8::MAX));
            }
        }
    }
    Level {
        width: out_w,
        height: out_h,
        channels,
        pixels,
    }
}

/// Draws `region` of the sheet at `zoom` with bilinear sampling of the reduction level that
/// has about one to two pixels per output pixel; white outside the sheet.
fn render(
    pages: &Pyramid,
    layout: &PageLayout,
    region: Rect,
    zoom: f64,
    width: u32,
    height: u32,
) -> Result<RgbaImage, PdfError> {
    let base = &pages.levels[0];
    let turned = matches!(layout.rotation, Rotation::Deg90 | Rotation::Deg270);
    // Level 0 pixels per output pixel, across the sheet.
    let across = if turned { base.height } else { base.width };
    let footprint = f64::from(across) / layout.sheet.width / zoom;
    let wanted = if footprint >= 2.0 {
        to_u32(footprint.log2().floor())
    } else {
        0
    };
    let page = &pages.levels[(wanted as usize).min(pages.levels.len() - 1)];
    let (pw, ph) = (f64::from(page.width), f64::from(page.height));
    // Page size as the sheet reads (after the quarter turns).
    let (rw, rh) = if turned { (ph, pw) } else { (pw, ph) };
    let sx = rw / layout.sheet.width;
    let sy = rh / layout.sheet.height;
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for j in 0..height {
        let y = region.origin.y + (f64::from(j) + 0.5) / zoom;
        for i in 0..width {
            let x = region.origin.x + (f64::from(i) + 0.5) / zoom;
            if x < 0.0 || y < 0.0 || x >= layout.sheet.width || y >= layout.sheet.height {
                out.extend_from_slice(&[255, 255, 255, 255]);
                continue;
            }
            let (u, v) = (x * sx, y * sy);
            // Back from the turned page to the upright page.
            let (px, py) = match layout.rotation {
                Rotation::Deg0 => (u, v),
                Rotation::Deg90 => (v, ph - u),
                Rotation::Deg180 => (pw - u, ph - v),
                Rotation::Deg270 => (pw - v, u),
            };
            let rgb = sample(page, px, py);
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    RgbaImage::from_raw(width, height, out)
        .ok_or_else(|| PdfError::Render("image tile has the wrong size".to_owned()))
}

/// Bilinear sample at a point in page pixels (pixel centres at half integers), edges clamped.
// The coordinates are clamped into the page and the result is a mix of 8 bit values, so the
// casts neither truncate nor lose a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn sample(page: &Level, x: f64, y: f64) -> [u8; 3] {
    let max_x = f64::from(page.width - 1);
    let max_y = f64::from(page.height - 1);
    let fx = (x - 0.5).clamp(0.0, max_x);
    let fy = (y - 0.5).clamp(0.0, max_y);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as usize, y0 as usize);
    let x1 = (x0 + 1).min(page.width as usize - 1);
    let y1 = (y0 + 1).min(page.height as usize - 1);
    let w = page.width as usize;
    let at = |x: usize, y: usize, c: usize| {
        let channel = if page.channels == 1 { 0 } else { c };
        f64::from(page.pixels[(y * w + x) * page.channels + channel])
    };
    let mut rgb = [0u8; 3];
    for (c, value) in rgb.iter_mut().enumerate() {
        let top = at(x0, y0, c) * (1.0 - tx) + at(x1, y0, c) * tx;
        let bottom = at(x0, y1, c) * (1.0 - tx) + at(x1, y1, c) * tx;
        *value = (top * (1.0 - ty) + bottom * ty).round() as u8;
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(width: u32, height: u32, pixels: Vec<u8>) -> DynamicImage {
        DynamicImage::ImageLuma8(image::GrayImage::from_raw(width, height, pixels).unwrap())
    }

    #[test]
    fn reduce_averages_blocks_and_partial_edges() {
        let image = grey(3, 1, vec![0, 100, 255]);
        let r = pyramid(&image, 2).levels.remove(0);
        assert_eq!((r.width, r.height), (2, 1));
        assert_eq!(r.pixels, vec![50, 255]);
    }

    #[test]
    fn pyramid_halves_down_to_the_smallest_level() {
        let image = grey(1000, 300, vec![200; 300_000]);
        let levels = pyramid(&image, 1).levels;
        let sizes: Vec<_> = levels.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(
            sizes,
            [(1000, 300), (500, 150), (250, 75), (125, 38), (63, 19)]
        );
    }

    #[test]
    fn photo_without_resolution_gets_an_a4_long_side() {
        let page = decode::Page {
            image: grey(4000, 3000, vec![0; 12_000_000]),
            dpi: Some((72.0, 72.0)),
        };
        let layout = layout(&page, &ImageEdit::default(), 0).unwrap();
        let long = 297.0 * UNITS_PER_MM;
        assert!((layout.sheet.width - long).abs() < 1e-9);
        assert!((layout.sheet.height - long * 0.75).abs() < 1e-9);
        // 4000 px over 11.7 in is 342 dpi, so blocks of 2 keep 171 dpi.
        assert_eq!(layout.block, 2);
        assert_eq!(layout.dpi, 171);
    }

    #[test]
    fn scan_resolution_gives_the_real_size_and_turns_swap_it() {
        let page = decode::Page {
            image: grey(600, 300, vec![0; 180_000]),
            dpi: Some((200.0, 200.0)),
        };
        let edit = ImageEdit {
            rotation: Rotation::Deg90,
            ..ImageEdit::default()
        };
        let layout = layout(&page, &edit, 0).unwrap();
        assert!((layout.sheet.width - 108.0).abs() < 1e-9);
        assert!((layout.sheet.height - 216.0).abs() < 1e-9);
        assert_eq!((layout.block, layout.dpi), (1, 200));
    }

    #[test]
    fn quarter_turn_maps_the_top_left_pixel_to_the_top_right() {
        // 2 x 1 page: black left, white right.
        let page = Level {
            width: 2,
            height: 1,
            channels: 1,
            pixels: vec![0, 255],
        };
        let layout = PageLayout {
            rotation: Rotation::Deg90,
            sheet: Size {
                width: 1.0,
                height: 2.0,
            },
            block: 1,
            dpi: 72,
            directory: 0,
        };
        let pages = Pyramid { levels: vec![page] };
        let out = render(&pages, &layout, Rect::new(0.0, 0.0, 1.0, 2.0), 1.0, 1, 2).unwrap();
        // Turned clockwise, the left (black) pixel is on top.
        assert_eq!(out.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(out.pixel(0, 1), Some([255, 255, 255, 255]));
    }
}
