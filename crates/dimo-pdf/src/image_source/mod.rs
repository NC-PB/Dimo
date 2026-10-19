//! Photos and scans as drawings: the image [`RasterSource`] (R.19, R.20, ADR 0010).
//!
//! An [`ImageSource`] draws the sheets of a stored JPEG, PNG or TIFF with the edit settings of
//! each page ([`ImageEdit`]). Opening decodes every page once (which checks the whole file and
//! gives the page sizes); afterwards at most [`PAGE_CACHE`] pages are kept in memory and others
//! are decoded again when a tile needs them. [`ImageSource::with_edits`] gives the same image
//! with other settings without decoding it again (the image editor, R.20).
//!
//! # Sheet size
//!
//! - With a paper set (R01 decision 9) the sheet is the paper.
//! - Without one, a resolution the file states between [`MIN_SCAN_DPI`] and [`MAX_SCAN_DPI`]
//!   gives the size (a scan). Otherwise (a phone photo says 72 dpi or nothing) the long side is
//!   297 mm, the length of an A4 sheet, and the aspect ratio is kept. A trim makes the sheet
//!   smaller at the same scale.
//!
//! The page is then reduced to at most [`MAX_DPI`] at the sheet size (R01 decision 9) by
//! averaging square blocks of whole pixels, so the same file always gives the same pixels.
//!
//! # Edits
//!
//! Quarter turns, fine angle, corners, trim and paper are one projective map per page
//! (`geometry.rs`); a tile pixel is sampled from the page point the inverse map gives. Clean up
//! (R01 decision 11) is applied to the kept pixels before the reductions (`clean.rs`).

mod clean;
mod corners;
mod decode;
mod geometry;
mod pdf;

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use dimo_core::{
    Homography, ImageEdit, ImageFormat, Point, Rect, Rotation, Sha256Hex, SheetCorners, SheetKind,
    SheetMap, Size,
};
use image::DynamicImage;

use crate::engine::output_size;
use crate::error::PdfError;
use crate::raster::RgbaImage;
use crate::source::{RasterSource, SourceKind};

pub use decode::{MAX_DECODE_BYTES, MAX_PAGES};
use geometry::PageInfo;
pub use geometry::{EditStage, PageGeometry};

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
const PIXELS_VERSION: &[u8] = b"dimo image source 3\n";

/// A photo or scan with the edit settings of its pages.
pub struct ImageSource {
    bytes: Arc<[u8]>,
    format: ImageFormat,
    content_hash: Sha256Hex,
    render_key: Sha256Hex,
    edits: Vec<ImageEdit>,
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
    info: PageInfo,
    geometry: PageGeometry,
    /// From sheet units back to upright page pixels.
    from_sheet: Homography,
    clean_up: bool,
    /// Index of the TIFF directory (or 0) the page is read from; thumbnails are skipped.
    directory: usize,
}

impl PageLayout {
    fn new(info: PageInfo, edit: &ImageEdit, directory: usize) -> Result<Self, PdfError> {
        edit.validate()
            .map_err(|e| PdfError::ImageEdit(e.to_owned()))?;
        let geometry = geometry::page_geometry(&info, edit)?;
        let from_sheet = geometry
            .to_sheet
            .inverse()
            .ok_or_else(|| PdfError::ImageEdit("the edit flattens the page".to_owned()))?;
        Ok(Self {
            info,
            geometry,
            from_sheet,
            clean_up: edit.clean_up,
            directory,
        })
    }

    /// True if both layouts keep the same pixels, so a decoded page can be shared.
    fn same_pixels(&self, other: &Self) -> bool {
        self.geometry.block == other.geometry.block
            && self.clean_up == other.clean_up
            && self.directory == other.directory
    }
}

/// Pixels of a page, upright (before any edit), 1 or 3 channels.
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
            let layout = PageLayout::new(page_info(&page), edit, directory)?;
            if first.is_none() {
                first = Some(Arc::new(pyramid(&page.image, &layout)));
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
        let edits = edits.map_or_else(|| vec![ImageEdit::default(); layouts.len()], <[_]>::to_vec);
        let render_key = render_key(&content_hash, &edits)?;
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
            edits,
            pages: layouts,
            cache: Mutex::new(
                first
                    .map(|p| (0, Arc::new(OnceLock::from(Ok(p)))))
                    .into_iter()
                    .collect(),
            ),
        })
    }

    /// The same image with other edit settings (one per page), without decoding it again.
    /// Decoded pages whose kept pixels do not change are shared, so the editor preview stays
    /// fast (R.20).
    pub fn with_edits(&self, edits: &[ImageEdit]) -> Result<Self, PdfError> {
        if edits.len() != self.pages.len() {
            return Err(edit_count(edits.len()));
        }
        let pages = self
            .pages
            .iter()
            .zip(edits)
            .map(|(page, edit)| PageLayout::new(page.info, edit, page.directory))
            .collect::<Result<Vec<_>, _>>()?;
        let cache = lock(&self.cache)
            .iter()
            .filter(|(i, _)| pages[*i].same_pixels(&self.pages[*i]))
            .map(|(i, cell)| (*i, Arc::clone(cell)))
            .collect();
        Ok(Self {
            bytes: Arc::clone(&self.bytes),
            format: self.format,
            content_hash: self.content_hash,
            render_key: render_key(&self.content_hash, edits)?,
            edits: edits.to_vec(),
            pages,
            cache: Mutex::new(cache),
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

    /// The edit settings of every page, in page order.
    pub fn edits(&self) -> &[ImageEdit] {
        &self.edits
    }

    /// Width and height of an upright page in pixels of the stored image, the frame of the
    /// corner points.
    pub fn page_pixels(&self, sheet: usize) -> Result<(u32, u32), PdfError> {
        let info = self.page(sheet)?.info;
        Ok((info.width, info.height))
    }

    /// The geometry of a page with its current settings.
    pub fn page_geometry(&self, sheet: usize) -> Result<PageGeometry, PdfError> {
        Ok(self.page(sheet)?.geometry)
    }

    /// The geometry a page would get with `edit` at an editor `stage`.
    pub fn stage_geometry(
        &self,
        sheet: usize,
        edit: &ImageEdit,
        stage: EditStage,
    ) -> Result<PageGeometry, PdfError> {
        edit.validate()
            .map_err(|e| PdfError::ImageEdit(e.to_owned()))?;
        geometry::stage_geometry(&self.page(sheet)?.info, edit, stage)
    }

    /// The sheet a page gets with `edit` and the map from its current sheet to it, so balloons
    /// stay on their features (R.20, R01 decision 10).
    pub fn sheet_map(&self, sheet: usize, edit: &ImageEdit) -> Result<SheetMap, PdfError> {
        let page = self.page(sheet)?;
        let new = PageLayout::new(page.info, edit, page.directory)?;
        Ok(SheetMap {
            size: new.geometry.sheet,
            raster_dpi: Some(new.geometry.raster_dpi),
            map: new.geometry.to_sheet.after(&page.from_sheet),
        })
    }

    /// The upright page point under a sheet point, in stored pixels.
    pub fn page_point(&self, sheet: usize, at: Point) -> Result<Option<Point>, PdfError> {
        Ok(self.page(sheet)?.from_sheet.apply(at))
    }

    /// The corners of the whole page, named as the sheet reads after `rotation`: where the
    /// editor starts when no sheet edge is found.
    pub fn page_corners(&self, sheet: usize, rotation: Rotation) -> Result<SheetCorners, PdfError> {
        Ok(geometry::page_corners(&self.page(sheet)?.info, rotation))
    }

    /// The corners of a bright sheet on a darker background, if one is easy to find (R01
    /// decision 8), named as the sheet reads after `rotation`.
    pub fn propose_corners(
        &self,
        sheet: usize,
        rotation: Rotation,
    ) -> Result<Option<SheetCorners>, PdfError> {
        let layout = self.page(sheet)?;
        if layout.clean_up {
            // The proposal needs the photo as taken; a cleaned page has a white background.
            let plain = self.with_edits(
                &self
                    .edits
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        if i == sheet {
                            ImageEdit {
                                clean_up: false,
                                ..e.clone()
                            }
                        } else {
                            e.clone()
                        }
                    })
                    .collect::<Vec<_>>(),
            )?;
            return plain.propose_corners(sheet, rotation);
        }
        let pyramid = self.reduced(sheet)?;
        let scale = f64::from(layout.geometry.block);
        Ok(corners::propose(&pyramid.levels, scale, &layout.info)
            .map(|page| geometry::corners_named(page, rotation)))
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
            let mut cache = lock(&self.cache);
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
                .map(|page| Arc::new(pyramid(&page.image, layout)))
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(PdfError::Image)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn page_info(page: &decode::Page) -> PageInfo {
    PageInfo {
        width: page.image.width(),
        height: page.image.height(),
        dpi: page.dpi,
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
        Ok(self.page(sheet)?.geometry.sheet)
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
        self.pages.get(sheet).map(|p| p.geometry.raster_dpi)
    }

    fn as_image(&self) -> Option<&ImageSource> {
        Some(self)
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

/// Level 0 reduced by the layout's block (and cleaned up if it says so), then halvings down
/// to [`MIN_LEVEL_SIDE`].
fn pyramid(image: &DynamicImage, layout: &PageLayout) -> Pyramid {
    let block = layout.geometry.block;
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
    let mut first = if block <= 1 {
        Level {
            pixels: image.as_bytes().to_vec(),
            ..base
        }
    } else {
        reduce(&base, image.as_bytes(), block)
    };
    if layout.clean_up {
        first = clean::clean_up(&first);
    }
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

/// Draws `region` of the sheet at `zoom`: every output pixel is sampled bilinearly at the page
/// point the inverse edit map gives, from the reduction level with about one to two pixels per
/// output pixel; white outside the sheet and outside the page.
fn render(
    pages: &Pyramid,
    layout: &PageLayout,
    region: Rect,
    zoom: f64,
    width: u32,
    height: u32,
) -> Result<RgbaImage, PdfError> {
    let sheet = layout.geometry.sheet;
    let back = &layout.from_sheet;
    let block = f64::from(layout.geometry.block);
    // Level 0 pixels per output pixel at the middle of the region.
    let center = region.center();
    let step = 1.0 / zoom;
    let footprint = match (
        back.apply(center),
        back.apply(Point {
            x: center.x + step,
            y: center.y,
        }),
        back.apply(Point {
            x: center.x,
            y: center.y + step,
        }),
    ) {
        (Some(c), Some(x), Some(y)) => {
            let d = |a: Point, b: Point| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            d(c, x).max(d(c, y)) / block
        }
        _ => 1.0,
    };
    let mut wanted = 0;
    let mut f = footprint;
    while f >= 2.0 && wanted + 1 < pages.levels.len() {
        f /= 2.0;
        wanted += 1;
    }
    let page = &pages.levels[wanted];
    // Full size page pixels per pixel of this level.
    let per_level = block * f64::from(1_u32 << wanted.min(31));
    let (full_w, full_h) = (f64::from(layout.info.width), f64::from(layout.info.height));
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for j in 0..height {
        let y = region.origin.y + (f64::from(j) + 0.5) / zoom;
        for i in 0..width {
            let x = region.origin.x + (f64::from(i) + 0.5) / zoom;
            let inside_sheet = x >= 0.0 && y >= 0.0 && x < sheet.width && y < sheet.height;
            let at = if inside_sheet {
                back.apply(Point { x, y })
            } else {
                None
            };
            match at {
                Some(p) if p.x >= 0.0 && p.y >= 0.0 && p.x < full_w && p.y < full_h => {
                    let rgb = sample(page, p.x / per_level, p.y / per_level);
                    out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
                _ => out.extend_from_slice(&[255, 255, 255, 255]),
            }
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

    fn layout(width: u32, height: u32, edit: &ImageEdit) -> PageLayout {
        let info = PageInfo {
            width,
            height,
            dpi: Some((72.0, 72.0)),
        };
        PageLayout::new(info, edit, 0).unwrap()
    }

    #[test]
    fn reduce_averages_blocks_and_partial_edges() {
        let image = grey(3, 1, vec![0, 100, 255]);
        let base = Level {
            width: 3,
            height: 1,
            channels: 1,
            pixels: Vec::new(),
        };
        let r = reduce(&base, image.as_bytes(), 2);
        assert_eq!((r.width, r.height), (2, 1));
        assert_eq!(r.pixels, vec![50, 255]);
    }

    #[test]
    fn pyramid_halves_down_to_the_smallest_level() {
        let image = grey(1000, 300, vec![200; 300_000]);
        let edit = ImageEdit::default();
        let mut layout = layout(1000, 300, &edit);
        layout.geometry.block = 1;
        let levels = pyramid(&image, &layout).levels;
        let sizes: Vec<_> = levels.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(
            sizes,
            [(1000, 300), (500, 150), (250, 75), (125, 38), (63, 19)]
        );
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
        let edit = ImageEdit {
            rotation: Rotation::Deg90,
            ..ImageEdit::default()
        };
        let layout = layout(2, 1, &edit);
        let sheet = layout.geometry.sheet;
        let pages = Pyramid { levels: vec![page] };
        let zoom = 1.0 / sheet.width;
        let out = render(
            &pages,
            &layout,
            Rect::new(0.0, 0.0, sheet.width, sheet.height),
            zoom,
            1,
            2,
        )
        .unwrap();
        // Turned clockwise, the left (black) pixel is on top.
        assert_eq!(out.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(out.pixel(0, 1), Some([255, 255, 255, 255]));
    }

    #[test]
    fn outside_the_page_is_white_after_a_fine_angle() {
        let page = Level {
            width: 4,
            height: 4,
            channels: 1,
            pixels: vec![0; 16],
        };
        let edit = ImageEdit {
            fine_angle: "10".parse().unwrap(),
            ..ImageEdit::default()
        };
        let layout = layout(4, 4, &edit);
        let sheet = layout.geometry.sheet;
        let pages = Pyramid { levels: vec![page] };
        let zoom = 8.0 / sheet.width;
        let out = render(
            &pages,
            &layout,
            Rect::new(0.0, 0.0, sheet.width, sheet.height),
            zoom,
            8,
            8,
        )
        .unwrap();
        assert_eq!(
            out.pixel(0, 0),
            Some([255, 255, 255, 255]),
            "corner of the box"
        );
        assert_eq!(out.pixel(4, 4), Some([0, 0, 0, 255]), "middle of the page");
    }
}
