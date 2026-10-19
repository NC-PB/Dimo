//! Photos and scans as raster sources (R.19 to R.21). Every image is made in the test, so no
//! binary fixtures are needed. Only the tests that read a written PDF back use PDFium; they
//! skip without it locally and fail instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

use std::io::Cursor;

use dimo_core::{
    ImageEdit, ImageFormat, Orientation, Paper, PaperSize, Rect, Rotation, SheetKind, Size,
    UNITS_PER_MM,
};
use dimo_pdf::image_source::MAX_DPI;
use dimo_pdf::tiles::{TileConfig, TileKey, TileService};
use dimo_pdf::{ImageSource, PdfEngine, PdfError, RasterOpener, RasterSource, SourceKind};
use image::codecs::jpeg::JpegEncoder;
use image::{ImageEncoder, RgbImage};
use tiff::encoder::{Rational, TiffEncoder, colortype};
use tiff::tags::ResolutionUnit;

const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// An RGB test page: black left half, white right half.
fn halves(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    })
}

fn png(image: &RgbImage) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

fn jpeg(image: &RgbImage) -> Vec<u8> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 95)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

/// Puts an EXIF segment with `orientation` right after the start marker of a JPEG.
fn with_exif_orientation(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    let mut tiff = b"II*\0\x08\0\0\0".to_vec();
    tiff.extend_from_slice(&1u16.to_le_bytes()); // one entry
    tiff.extend_from_slice(&0x0112u16.to_le_bytes()); // Orientation
    tiff.extend_from_slice(&3u16.to_le_bytes()); // SHORT
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&orientation.to_le_bytes());
    tiff.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // value padding, no next directory
    let mut segment = b"Exif\0\0".to_vec();
    segment.extend_from_slice(&tiff);
    let len = u16::try_from(segment.len() + 2).unwrap();
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&segment);
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// A three page TIFF: RGB at 200 dpi, 16 bit grey without a resolution, 8 bit grey at 300 dpi.
fn three_page_tiff() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out).unwrap();
    let rgb = halves(400, 200);
    let mut page = encoder.new_image::<colortype::RGB8>(400, 200).unwrap();
    page.resolution(ResolutionUnit::Inch, Rational { n: 200, d: 1 });
    page.write_data(rgb.as_raw()).unwrap();
    let grey16: Vec<u16> = (0..300 * 100)
        .map(|i| if i % 300 < 150 { 0 } else { 65535 })
        .collect();
    encoder
        .write_image::<colortype::Gray16>(300, 100, &grey16)
        .unwrap();
    let grey8 = vec![128u8; 600 * 600];
    let mut page = encoder.new_image::<colortype::Gray8>(600, 600).unwrap();
    page.resolution(ResolutionUnit::Inch, Rational { n: 300, d: 1 });
    page.write_data(&grey8).unwrap();
    out.into_inner()
}

/// An uncompressed 1 bit TIFF, the way scanners often store line drawings. `rows` holds the
/// packed bits, each row starting on a byte.
fn bilevel_tiff(width: u16, height: u16, rows: &[u8], white_is_zero: bool) -> Vec<u8> {
    let entries: [(u16, u16, u32); 9] = [
        (256, 3, u32::from(width)),
        (257, 3, u32::from(height)),
        (258, 3, 1),
        (259, 3, 1),
        (262, 3, u32::from(!white_is_zero)),
        (273, 4, 0), // strip offset, set below
        (277, 3, 1),
        (278, 3, u32::from(height)),
        (279, 4, u32::try_from(rows.len()).unwrap()),
    ];
    let ifd_len = 2 + entries.len() * 12 + 4;
    let data_at = u32::try_from(8 + ifd_len).unwrap();
    let mut out = b"II*\0\x08\0\0\0".to_vec();
    out.extend_from_slice(&u16::try_from(entries.len()).unwrap().to_le_bytes());
    for (tag, kind, value) in entries {
        let value = if tag == 273 { data_at } else { value };
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        if kind == 3 {
            out.extend_from_slice(&u16::try_from(value).unwrap().to_le_bytes());
            out.extend_from_slice(&[0, 0]);
        } else {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(rows);
    out
}

fn whole_sheet(source: &dyn RasterSource, sheet: usize, zoom: f64) -> dimo_pdf::RgbaImage {
    let size = source.sheet_size(sheet).unwrap();
    source
        .render_region(sheet, Rect::new(0.0, 0.0, size.width, size.height), zoom)
        .unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_png_photo_without_resolution_becomes_an_a4_long_sheet() {
    let source = ImageSource::open(png(&halves(800, 600)), ImageFormat::Png, None).unwrap();
    assert_eq!(source.kind(), SourceKind::Image);
    assert_eq!(source.sheet_count(), 1);
    assert_eq!(source.sheet_kind(0).unwrap(), SheetKind::Raster);
    let size = source.sheet_size(0).unwrap();
    assert!(close(size.width, 297.0 * UNITS_PER_MM));
    assert!(close(size.height, 297.0 * UNITS_PER_MM * 0.75));
    // 800 px over 11.69 in.
    assert_eq!(source.raster_dpi(0), Some(68));

    let image = whole_sheet(&source, 0, 1.0);
    assert_eq!(image.pixel(10, 10), Some(BLACK));
    assert_eq!(image.pixel(image.width() - 10, 10), Some(WHITE));
    // Outside the sheet is white.
    let outside = source
        .render_region(0, Rect::new(-50.0, -50.0, 40.0, 40.0), 1.0)
        .unwrap();
    assert!(outside.as_bytes().chunks(4).all(|p| p == WHITE));
}

#[test]
fn a_jpeg_follows_its_exif_orientation() {
    // 6: the camera was turned, the picture is shown turned 90 degrees clockwise.
    let plain = jpeg(&halves(400, 200));
    let turned = with_exif_orientation(&plain, 6);
    let source = ImageSource::open(turned, ImageFormat::Jpeg, None).unwrap();
    let size = source.sheet_size(0).unwrap();
    assert!(
        size.height > size.width,
        "portrait after the turn: {size:?}"
    );
    let image = whole_sheet(&source, 0, 1.0);
    // The black left half is now on top.
    let top = image.pixel(image.width() / 2, 10).unwrap();
    let bottom = image.pixel(image.width() / 2, image.height() - 10).unwrap();
    assert!(
        top[0] < 30 && bottom[0] > 225,
        "top {top:?}, bottom {bottom:?}"
    );

    let upright = ImageSource::open(plain, ImageFormat::Jpeg, None).unwrap();
    let size = upright.sheet_size(0).unwrap();
    assert!(size.width > size.height);
}

#[test]
fn a_three_page_tiff_gives_three_sheets_with_their_sizes() {
    let source = ImageSource::open(three_page_tiff(), ImageFormat::Tiff, None).unwrap();
    assert_eq!(source.sheet_count(), 3);
    // Page 1 at 200 dpi: 2 x 1 inch.
    let first = source.sheet_size(0).unwrap();
    assert!(
        close(first.width, 144.0) && close(first.height, 72.0),
        "{first:?}"
    );
    assert_eq!(source.raster_dpi(0), Some(200));
    // Page 2 states no resolution: long side 297 mm.
    let second = source.sheet_size(1).unwrap();
    assert!(close(second.width, 297.0 * UNITS_PER_MM), "{second:?}");
    // Page 3 at 300 dpi: 2 x 2 inch.
    let third = source.sheet_size(2).unwrap();
    assert!(close(third.width, 144.0) && close(third.height, 144.0));

    let image = whole_sheet(&source, 1, 2.0);
    assert_eq!(image.pixel(5, 5), Some(BLACK), "16 bit black stays black");
    assert_eq!(image.pixel(image.width() - 5, 5), Some(WHITE));
    let image = whole_sheet(&source, 2, 1.0);
    assert_eq!(image.pixel(5, 5), Some([128, 128, 128, 255]));
    // Pages are decoded again when the cache dropped them.
    let image = whole_sheet(&source, 0, 1.0);
    assert_eq!(image.pixel(5, 5), Some(BLACK));
}

#[test]
fn one_bit_scans_read_with_both_photometric_conventions() {
    // 8 x 2 pixels: first row left half set, second row empty.
    let rows = [0b1111_0000, 0b0000_0000];
    for white_is_zero in [true, false] {
        let bytes = bilevel_tiff(8, 2, &rows, white_is_zero);
        let source = ImageSource::open(bytes, ImageFormat::Tiff, None).unwrap();
        let size = source.sheet_size(0).unwrap();
        let image = source
            .render_region(
                0,
                Rect::new(0.0, 0.0, size.width, size.height),
                8.0 / size.width,
            )
            .unwrap();
        let set = if white_is_zero { BLACK } else { WHITE };
        let clear = if white_is_zero { WHITE } else { BLACK };
        assert_eq!(
            image.pixel(0, 0),
            Some(set),
            "white_is_zero {white_is_zero}"
        );
        assert_eq!(image.pixel(7, 0), Some(clear));
        assert_eq!(image.pixel(0, 1), Some(clear));
    }
}

#[test]
fn large_photos_are_kept_at_most_at_300_dpi() {
    // A 300 dpi scan of an A4 page is kept as it is; at 600 dpi it is halved.
    let a4_at = |dpi: u32| {
        let width = 210 * dpi * 10 / 254;
        let height = 297 * dpi * 10 / 254;
        let mut out = Cursor::new(Vec::new());
        let mut encoder = TiffEncoder::new(&mut out).unwrap();
        let mut page = encoder
            .new_image::<colortype::Gray8>(width, height)
            .unwrap();
        page.resolution(ResolutionUnit::Inch, Rational { n: dpi, d: 1 });
        page.write_data(&vec![255u8; (width * height) as usize])
            .unwrap();
        out.into_inner()
    };
    let at_300 = ImageSource::open(a4_at(300), ImageFormat::Tiff, None).unwrap();
    assert_eq!(at_300.raster_dpi(0), Some(300));
    let at_600 = ImageSource::open(a4_at(600), ImageFormat::Tiff, None).unwrap();
    let dpi = f64::from(at_600.raster_dpi(0).unwrap());
    assert!((MAX_DPI - 1.0..=MAX_DPI).contains(&dpi), "{dpi}");
    let size = at_600.sheet_size(0).unwrap();
    assert!((size.width / UNITS_PER_MM - 210.0).abs() < 0.2, "{size:?}");
}

#[test]
fn quarter_turns_and_paper_set_the_sheet() {
    let bytes = png(&halves(400, 200));
    let turned = ImageEdit {
        rotation: Rotation::Deg90,
        ..ImageEdit::default()
    };
    let plain = ImageSource::open(bytes.clone(), ImageFormat::Png, None).unwrap();
    let source = ImageSource::open(
        bytes.clone(),
        ImageFormat::Png,
        Some(std::slice::from_ref(&turned)),
    )
    .unwrap();
    let (a, b) = (plain.sheet_size(0).unwrap(), source.sheet_size(0).unwrap());
    assert!(close(a.width, b.height) && close(a.height, b.width));
    let image = whole_sheet(&source, 0, 1.0);
    let top = image.pixel(image.width() / 2, 5).unwrap();
    assert_eq!(
        top, BLACK,
        "turned clockwise, the black left half is on top"
    );

    let on_a4 = ImageEdit {
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: Orientation::Landscape,
        }),
        ..ImageEdit::default()
    };
    let source = ImageSource::open(bytes, ImageFormat::Png, Some(&[on_a4])).unwrap();
    let size = source.sheet_size(0).unwrap();
    assert!(close(size.width, 297.0 * UNITS_PER_MM) && close(size.height, 210.0 * UNITS_PER_MM));
}

#[test]
fn thin_lines_stay_visible_when_zoomed_out() {
    // A 300 dpi A4 scan, white with one black line one pixel wide.
    let (width, height) = (2480u32, 3508u32);
    let mut grey = vec![255u8; (width * height) as usize];
    for y in 0..height {
        grey[(y * width + 1001) as usize] = 0;
    }
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out).unwrap();
    let mut page = encoder
        .new_image::<colortype::Gray8>(width, height)
        .unwrap();
    page.resolution(ResolutionUnit::Inch, Rational { n: 300, d: 1 });
    page.write_data(&grey).unwrap();
    let source = ImageSource::open(out.into_inner(), ImageFormat::Tiff, None).unwrap();
    // 1/8 zoom: one output pixel covers about 33 scan pixels.
    let image = whole_sheet(&source, 0, 0.125);
    let darkest = (0..image.width())
        .map(|x| image.pixel(x, image.height() / 2).unwrap()[0])
        .min()
        .unwrap();
    // The line covers about 1/33 of an output pixel, so its area average is a light grey;
    // what matters is that it is averaged in and never skipped (sampling only four pixels
    // per output pixel misses it and gives pure white).
    assert!(darkest < 255, "the line is averaged in, darkest {darkest}");
}

#[test]
fn tiff_thumbnails_are_not_pages() {
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out).unwrap();
    encoder
        .write_image::<colortype::Gray8>(40, 20, &[0u8; 800])
        .unwrap();
    let mut thumb = encoder.new_image::<colortype::Gray8>(4, 2).unwrap();
    thumb
        .encoder()
        .write_tag(tiff::tags::Tag::NewSubfileType, 1u32)
        .unwrap();
    thumb.write_data(&[0u8; 8]).unwrap();
    encoder
        .write_image::<colortype::Gray8>(30, 30, &[255u8; 900])
        .unwrap();
    let source = ImageSource::open(out.into_inner(), ImageFormat::Tiff, None).unwrap();
    assert_eq!(source.sheet_count(), 2);
    // The second sheet is the third directory: square and white.
    let size = source.sheet_size(1).unwrap();
    assert!(close(size.width, size.height));
    assert_eq!(whole_sheet(&source, 1, 1.0).pixel(3, 3), Some(WHITE));
}

#[test]
fn the_render_key_follows_the_settings() {
    let bytes = png(&halves(40, 20));
    let plain = ImageSource::open(bytes.clone(), ImageFormat::Png, None).unwrap();
    let again = ImageSource::open(bytes.clone(), ImageFormat::Png, None).unwrap();
    let turned = ImageSource::open(
        bytes,
        ImageFormat::Png,
        Some(&[ImageEdit {
            rotation: Rotation::Deg180,
            ..ImageEdit::default()
        }]),
    )
    .unwrap();
    assert_eq!(plain.render_key(), again.render_key());
    assert_eq!(plain.content_hash(), turned.content_hash());
    assert_ne!(plain.render_key(), turned.render_key());
    assert_ne!(plain.render_key(), *plain.content_hash());
}

#[test]
fn the_same_file_always_gives_the_same_pixels() {
    let bytes = jpeg(&halves(1000, 700));
    let a = ImageSource::open(bytes.clone(), ImageFormat::Jpeg, None).unwrap();
    let b = ImageSource::open(bytes, ImageFormat::Jpeg, None).unwrap();
    let region = Rect::new(13.5, 20.25, 300.0, 200.0);
    assert_eq!(
        a.render_region(0, region, 1.7).unwrap(),
        b.render_region(0, region, 1.7).unwrap()
    );
}

#[test]
fn damaged_and_unsupported_files_are_refused_with_a_reason() {
    let refused = |bytes: Vec<u8>, format, edits: Option<&[ImageEdit]>| match ImageSource::open(
        bytes, format, edits,
    ) {
        Err(PdfError::Image(reason)) => reason,
        other => panic!("expected an image error, got {other:?}"),
    };
    let mut cut = png(&halves(100, 100));
    cut.truncate(cut.len() / 2);
    refused(cut, ImageFormat::Png, None);
    refused(b"\xFF\xD8\xFF garbage".to_vec(), ImageFormat::Jpeg, None);

    let bytes = png(&halves(10, 10));
    let edit_refused = |bytes: Vec<u8>, edits: &[ImageEdit]| match ImageSource::open(
        bytes,
        ImageFormat::Png,
        Some(edits),
    ) {
        Err(PdfError::ImageEdit(reason)) => reason,
        other => panic!("expected an edit error, got {other:?}"),
    };
    edit_refused(bytes.clone(), &[]);
    edit_refused(bytes.clone(), &[ImageEdit::default(), ImageEdit::default()]);
    // R.20: corners straighten the sheet, so a fine angle with them is refused.
    let twice = ImageEdit {
        fine_angle: "2".parse().unwrap(),
        corners: Some(
            ImageSource::open(bytes.clone(), ImageFormat::Png, None)
                .unwrap()
                .page_corners(0, dimo_core::Rotation::Deg0)
                .unwrap(),
        ),
        ..ImageEdit::default()
    };
    let reason = edit_refused(bytes, &[twice]);
    assert!(reason.contains("fine angle"), "{reason}");

    // CMYK TIFF.
    let mut out = Cursor::new(Vec::new());
    TiffEncoder::new(&mut out)
        .unwrap()
        .write_image::<colortype::CMYK8>(2, 2, &[0u8; 16])
        .unwrap();
    let reason = refused(out.into_inner(), ImageFormat::Tiff, None);
    assert!(reason.contains("not supported"), "{reason}");
}

#[test]
fn the_tile_service_opens_and_serves_images() {
    let service = TileService::without_pdf(TileConfig::default());
    let bytes = png(&halves(800, 600));
    let opened = service
        .open_image(bytes.clone(), ImageFormat::Png, None)
        .unwrap();
    assert!(opened.newly_opened);
    let key = opened.source.render_key();
    assert!(service.source(&key).is_some());
    let tile = service
        .get(TileKey {
            doc: key,
            sheet: 0,
            zoom: 0,
            x: 0,
            y: 0,
        })
        .unwrap();
    assert_ne!(tile.as_bytes(), b"");

    let again = service
        .open_image(bytes.clone(), ImageFormat::Png, None)
        .unwrap();
    assert!(
        !again.newly_opened,
        "the open drawing is shared, not opened twice"
    );
    let stored = service
        .open_image(bytes, ImageFormat::Png, Some(&[ImageEdit::default()]))
        .unwrap();
    assert!(
        !stored.newly_opened,
        "found by its render key before decoding"
    );
    assert_eq!(stored.source.render_key(), key);
    service.release(&key);
    assert!(service.source(&key).is_none());
}

#[test]
fn the_pdf_engine_opens_images_without_registering_them() {
    let Ok(engine) = PdfEngine::start() else {
        // Without PDFium the default trait method is covered by the tile service test above.
        return;
    };
    let opened = engine
        .open_image(png(&halves(20, 10)), ImageFormat::Png, None)
        .unwrap();
    assert!(opened.newly_opened);
    assert_eq!(
        opened.source.sheet_size(0).unwrap(),
        Size {
            width: 297.0 * UNITS_PER_MM,
            height: 297.0 * UNITS_PER_MM / 2.0,
        }
    );
}

/// A pixel position `(x, y)`.
type Pixel = (f64, f64);

/// A photo of a sheet on a table (R.20): grey table, a skewed white sheet with a black dot
/// near its top left corner. Returns the PNG, the sheet corners and the dot center in pixels.
fn skewed_photo() -> (Vec<u8>, [Pixel; 4], Pixel) {
    let corners = [(80.0, 60.0), (560.0, 90.0), (540.0, 420.0), (60.0, 400.0)];
    let dot = (150.0, 130.0);
    let inside = |x: f64, y: f64| {
        (0..4).all(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0) >= 0.0
        })
    };
    let image = RgbImage::from_fn(640, 480, |x, y| {
        let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        if (px - dot.0).powi(2) + (py - dot.1).powi(2) < 64.0 {
            image::Rgb([0, 0, 0])
        } else if inside(px, py) {
            image::Rgb([240, 240, 235])
        } else {
            image::Rgb([90, 80, 70])
        }
    });
    (png(&image), corners, dot)
}

#[test]
fn corners_of_a_photographed_sheet_are_proposed() {
    let (bytes, corners, _) = skewed_photo();
    let source = ImageSource::open(bytes, ImageFormat::Png, None).unwrap();
    let found = source.propose_corners(0, Rotation::Deg0).unwrap().unwrap();
    for (p, want) in found.points().iter().zip(corners) {
        let (x, y): (f64, f64) = (p.x.try_into().unwrap(), p.y.try_into().unwrap());
        assert!(
            (x - want.0).abs() < 6.0 && (y - want.1).abs() < 6.0,
            "{p:?} {want:?}"
        );
    }
    // Turned once, the sheet's top left is the page's bottom left.
    let turned = source.propose_corners(0, Rotation::Deg90).unwrap().unwrap();
    assert_eq!(turned.top_left, found.bottom_left);
    let plain = ImageSource::open(png(&halves(40, 40)), ImageFormat::Png, None).unwrap();
    assert_eq!(plain.propose_corners(0, Rotation::Deg0).unwrap(), None);
}

#[test]
fn an_edited_photo_keeps_its_features_where_the_map_says() {
    // R.20 acceptance: the sheet map moves a point on the old sheet onto the same feature of
    // the new sheet, through corners, trim and paper.
    let (bytes, corners, dot) = skewed_photo();
    let source = ImageSource::open(bytes.clone(), ImageFormat::Png, None).unwrap();
    let found = source.propose_corners(0, Rotation::Deg0).unwrap().unwrap();
    let edit = ImageEdit {
        corners: Some(found),
        trim: Some(dimo_core::Trim {
            left: "0.02".parse().unwrap(),
            top: "0.02".parse().unwrap(),
            right: "0.98".parse().unwrap(),
            bottom: "0.98".parse().unwrap(),
        }),
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: Orientation::Landscape,
        }),
        clean_up: true,
        ..ImageEdit::default()
    };
    let old = source.page_geometry(0).unwrap();
    let on_old = old
        .to_sheet
        .apply(dimo_core::Point { x: dot.0, y: dot.1 })
        .unwrap();
    let map = source.sheet_map(0, &edit).unwrap();
    assert!(close(map.size.width, 297.0 * UNITS_PER_MM));
    let on_new = map.map.apply(on_old).unwrap();

    let edited = source.with_edits(std::slice::from_ref(&edit)).unwrap();
    let reopened = ImageSource::open(bytes, ImageFormat::Png, Some(&[edit])).unwrap();
    assert_eq!(edited.render_key(), reopened.render_key());
    let back = edited.page_point(0, on_new).unwrap().unwrap();
    assert!((back.x - dot.0).abs() < 1e-6 && (back.y - dot.1).abs() < 1e-6);
    // The dot is black on the new sheet, the paper around it white (clean up), the table gone.
    let zoom = 2.0;
    let around = Rect::new(on_new.x - 20.0, on_new.y - 20.0, 40.0, 40.0);
    let tile = edited.render_region(0, around, zoom).unwrap();
    let (w, h) = (tile.width(), tile.height());
    assert_eq!(tile.pixel(w / 2, h / 2), Some(BLACK));
    assert_eq!(tile.pixel(1, 1), Some(WHITE));
    let corner = edited
        .render_region(0, Rect::new(0.0, 0.0, 4.0, 4.0), 1.0)
        .unwrap();
    assert_eq!(corner.pixel(2, 2), Some(WHITE), "{:?}", corners[0]);
    // Same edit, same pixels, opened either way.
    assert_eq!(
        tile.as_bytes(),
        reopened.render_region(0, around, zoom).unwrap().as_bytes()
    );
}

#[test]
fn a_fine_angle_straightens_a_turned_line() {
    // A horizontal black bar drawn 4 degrees clockwise; -4 degrees makes it level again.
    let tilt = 4.0_f64.to_radians();
    let image = RgbImage::from_fn(400, 300, |x, y| {
        let (dx, dy) = (f64::from(x) - 200.0, f64::from(y) - 150.0);
        // Undo the tilt to see where the point was on the level bar.
        let level_y = -dx * tilt.sin() + dy * tilt.cos();
        if level_y.abs() < 3.0 && dx.abs() < 150.0 {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    });
    let edit = ImageEdit {
        fine_angle: "-4".parse().unwrap(),
        ..ImageEdit::default()
    };
    let source = ImageSource::open(png(&image), ImageFormat::Png, Some(&[edit])).unwrap();
    let size = source.sheet_size(0).unwrap();
    let to_sheet = source.page_geometry(0).unwrap().to_sheet;
    for side in [-1.0, 1.0] {
        let end = dimo_core::Point {
            x: 200.0 + side * 120.0 * tilt.cos(),
            y: 150.0 + side * 120.0 * tilt.sin(),
        };
        let on_sheet = to_sheet.apply(end).unwrap();
        assert!(
            (on_sheet.y - size.height / 2.0).abs() < 0.01,
            "level: {on_sheet:?}"
        );
        let dot = source
            .render_region(
                0,
                Rect::new(on_sheet.x - 0.5, on_sheet.y - 0.5, 1.0, 1.0),
                1.0,
            )
            .unwrap();
        assert_eq!(dot.pixel(0, 0), Some(BLACK));
    }
}

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// PDFium for the tests that read the written PDF back; `None` (skip) without it locally.
fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            #[allow(clippy::print_stderr)]
            {
                eprintln!("SKIPPED (PDFium missing): {e}");
            }
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// R.21: the edited sheet as a PDF page, the same bytes every time.
#[test]
fn an_image_drawing_becomes_a_pdf_with_one_page_per_sheet() {
    let a4 = ImageEdit {
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: Orientation::Landscape,
        }),
        ..ImageEdit::default()
    };
    // Blue band on top, then red left and white right: a flip either way would show.
    let colour = RgbImage::from_fn(400, 280, |x, y| {
        if y < 40 {
            image::Rgb([30, 30, 200])
        } else if x < 200 {
            image::Rgb([200, 30, 30])
        } else {
            image::Rgb([255, 255, 255])
        }
    });
    let source = ImageSource::open(
        png(&colour),
        ImageFormat::Png,
        Some(std::slice::from_ref(&a4)),
    )
    .unwrap();
    let pdf = source.to_pdf().unwrap();
    assert_eq!(pdf, source.to_pdf().unwrap(), "same bytes twice");
    assert!(pdf.starts_with(b"%PDF-1.7\n"));
    assert!(contains(&pdf, b"/MediaBox [0 0 841.8898 595.2756]"));
    assert!(contains(&pdf, b"/DCTDecode") && !contains(&pdf, b"/FlateDecode"));
    assert!(!contains(&pdf, b"CreationDate") && !contains(&pdf, b"Producer"));

    // A grey three page scan: Flate grey pages.
    let scan = ImageSource::open(three_page_tiff(), ImageFormat::Tiff, None).unwrap();
    let scan_pdf = scan.to_pdf().unwrap();
    assert!(contains(&scan_pdf, b"/Count 3"));
    assert!(
        contains(&scan_pdf, b"/DeviceGray"),
        "the 16 and 8 bit grey pages"
    );
    assert!(contains(&scan_pdf, b"/DeviceRGB"), "the RGB page");

    let Some(engine) = engine() else { return };
    let doc = engine.open(pdf.clone()).unwrap();
    assert_eq!(doc.sheet_count(), 1);
    let size = doc.sheet_size(0).unwrap();
    // The media box has 4 decimals and PDFium reads it as 32 bit floats.
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    assert!(near(size.width, 297.0 * UNITS_PER_MM) && near(size.height, 210.0 * UNITS_PER_MM));
    let page = doc
        .render_region(0, Rect::new(0.0, 0.0, size.width, size.height), 0.5)
        .unwrap();
    let blue = page.pixel(page.width() / 2, 5).unwrap();
    assert!(blue[2] > 150 && blue[0] < 80, "blue band on top: {blue:?}");
    let red = page.pixel(20, page.height() - 20).unwrap();
    let white = page.pixel(page.width() - 20, page.height() - 20).unwrap();
    assert!(red[0] > 150 && red[1] < 80, "red half: {red:?}");
    assert!(white.iter().all(|&c| c > 240), "white half: {white:?}");
    let scan_doc = engine.open(scan_pdf).unwrap();
    for (pdf_size, sheet) in scan_doc
        .sheet_sizes()
        .iter()
        .zip(scan.sheet_sizes().unwrap())
    {
        assert!(near(pdf_size.width, sheet.width) && near(pdf_size.height, sheet.height));
    }
    assert_eq!(scan_doc.sheet_count(), 3);

    // The balloon writer takes it like any PDF, byte identical twice.
    let overlay = || dimo_pdf::BalloonOverlay {
        sheets: vec![dimo_pdf::SheetBalloons {
            sheet: 0,
            balloons: vec![dimo_pdf::Balloon {
                shape: dimo_pdf::BalloonShape::Circle,
                center: dimo_pdf::SheetPoint::new(600.0, 300.0),
                width: 20.0,
                height: 20.0,
                fill: Some(dimo_pdf::Rgb::WHITE),
                outline: Some(dimo_pdf::Stroke {
                    color: dimo_pdf::Rgb::new(0x00, 0x57, 0xB8),
                    width: 1.0,
                }),
                text: "17".to_owned(),
                text_color: dimo_pdf::Rgb::BLACK,
                text_size: 10.0,
                leader: None,
            }],
        }],
        output: dimo_pdf::BalloonOutput::PageContent,
    };
    let ballooned = engine.write_ballooned(pdf.clone(), overlay()).unwrap();
    assert_eq!(ballooned, engine.write_ballooned(pdf, overlay()).unwrap());
    let doc = engine.open(ballooned).unwrap();
    let runs = doc.text_runs(0).unwrap();
    let number = runs.iter().find(|r| r.text.trim() == "17").unwrap();
    let b = number.bbox;
    let (x, y) = (
        b.origin.x + b.size.width / 2.0,
        b.origin.y + b.size.height / 2.0,
    );
    assert!(
        (x - 600.0).abs() < 6.0 && (y - 300.0).abs() < 6.0,
        "balloon number at {x}, {y}"
    );
}

/// A sheet wider than one render call allows (an A0 length at 300 dpi is about 14000 pixels)
/// is drawn in blocks and still becomes one page image.
#[test]
fn a_very_wide_scan_becomes_one_pdf_page() {
    let (width, height) = (17_000u32, 60u32);
    let grey: Vec<u8> = (0..width * height)
        .map(|i| if (i % width) < width / 2 { 0 } else { 255 })
        .collect();
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out).unwrap();
    let mut page = encoder
        .new_image::<colortype::Gray8>(width, height)
        .unwrap();
    page.resolution(ResolutionUnit::Inch, Rational { n: 300, d: 1 });
    page.write_data(&grey).unwrap();
    let source = ImageSource::open(out.into_inner(), ImageFormat::Tiff, None).unwrap();
    assert_eq!(source.raster_dpi(0), Some(300));
    let pdf = source.to_pdf().unwrap();
    assert!(contains(&pdf, b"/Width 17000 /Height 60"));
    assert!(contains(&pdf, b"/DeviceGray"));
}
