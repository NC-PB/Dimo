//! Integration tests against the real PDFium library.
//!
//! Tests that need PDFium skip with a message when the library is missing (docs/dev/testing.md),
//! unless `CI=true` or `DIMO_REQUIRE_PDFIUM=1` is set: then a missing library fails the test, so
//! CI can never pass by skipping. Fetch the library with `./scripts/fetch-pdfium.sh`.
//!
//! All tests run in one process and share the one render thread, which also exercises
//! concurrent use of the engine.

// Test code (rust.md): unwrap is fine in helpers too, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_pdf::{ContentHash, PdfEngine, PdfError, RgbaImage, SheetRect};

const TEST_DRAWING_1: &str = "test_drawing_1.pdf";
const TEST_DRAWING_1_SHA256: &str =
    "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
}

fn corpus_drawing(name: &str) -> Vec<u8> {
    let path = repo_root().join("corpus/drawings").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// The shared engine, or `None` (with a message) when PDFium is missing and not required.
fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// The SHA-256 recorded in corpus/PROVENANCE.md for `name`, possibly abbreviated as
/// `<prefix>...<suffix>`. Returns (prefix, suffix).
fn provenance_hash(name: &str) -> (String, String) {
    let text = std::fs::read_to_string(repo_root().join("corpus/PROVENANCE.md")).unwrap();
    let row = text
        .lines()
        .find(|l| l.contains(&format!("drawings/{name}")))
        .unwrap_or_else(|| panic!("{name} has no PROVENANCE.md entry"));
    let hash = row.split('|').nth(2).unwrap().trim();
    match hash.split_once("...") {
        Some((prefix, suffix)) => (prefix.to_owned(), suffix.to_owned()),
        None => (hash.to_owned(), String::new()),
    }
}

// FR-DOC-07, docs/dev/testing.md: corpus files are verified against PROVENANCE.md.
// Needs no PDFium, so it always runs.
#[test]
fn test_drawing_1_hash_matches_provenance() {
    let hash = ContentHash::of(&corpus_drawing(TEST_DRAWING_1)).to_hex();
    assert_eq!(hash, TEST_DRAWING_1_SHA256);
    let (prefix, suffix) = provenance_hash(TEST_DRAWING_1);
    assert!(prefix.len() >= 8, "PROVENANCE.md hash too short");
    assert!(
        hash.starts_with(&prefix),
        "{hash} vs PROVENANCE.md {prefix}..."
    );
    assert!(
        hash.ends_with(&suffix),
        "{hash} vs PROVENANCE.md ...{suffix}"
    );
}

#[test]
fn test_drawing_1_sheets() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    assert_eq!(doc.content_hash().to_hex(), TEST_DRAWING_1_SHA256);
    assert_eq!(doc.sheet_count(), 1);
    let size = doc.sheet_size(0).unwrap();
    assert!((size.width - 1584.0).abs() < 1e-9, "{size:?}");
    assert!((size.height - 1224.0).abs() < 1e-9, "{size:?}");
    assert!(matches!(
        doc.sheet_size(1),
        Err(PdfError::SheetOutOfRange { index: 1, count: 1 })
    ));
}

/// Coarse, quantized fingerprint of a rendered image for snapshots.
///
/// Exact pixel hashes are fragile: anti-aliasing may differ in the last bit between CPU
/// architectures (fused multiply add on arm64) or after a PDFium bump. The image is therefore
/// averaged over `cell` x `cell` pixel blocks, and each block's mean ink (255 minus luminance)
/// is quantized to four levels. The level limits (2.5, 19.25, 82 mean ink) sit in gaps of the
/// distribution for `test_drawing_1.pdf` at zoom 0.25 and 12 px cells: every block is at least 0.68
/// mean ink away from a limit, about 100 luminance steps summed over the block (more than a
/// third of a pixel turning from white to black). Last bit differences cannot flip a block.
/// The grid itself is part of the snapshot, so a real change shows where it happened.
fn fingerprint(img: &RgbaImage, cell: u32) -> String {
    let cols = img.width().div_ceil(cell);
    let rows = img.height().div_ceil(cell);
    let mut grid = String::new();
    for gy in 0..rows {
        grid.push('|');
        for gx in 0..cols {
            let (mut ink, mut count) = (0u64, 0u64);
            for y in gy * cell..((gy + 1) * cell).min(img.height()) {
                for x in gx * cell..((gx + 1) * cell).min(img.width()) {
                    let [r, g, b, _] = img.pixel(x, y).unwrap();
                    let lum = (299 * u64::from(r) + 587 * u64::from(g) + 114 * u64::from(b)) / 1000;
                    ink += 255 - lum;
                    count += 1;
                }
            }
            // Mean ink below `limit` hundredths, compared exactly in integers.
            let below = |limit: u64| ink * 100 < limit * count;
            let level = if below(250) {
                ' '
            } else if below(1925) {
                '.'
            } else if below(8200) {
                '+'
            } else {
                '#'
            };
            grid.push(level);
        }
        grid.push_str("|\n");
    }
    format!(
        "image: {} x {} px\ncells: {cell} px, grid {cols} x {rows}\ngrid sha256: {}\n\n{grid}",
        img.width(),
        img.height(),
        ContentHash::of(grid.as_bytes()),
    )
}

#[test]
fn test_drawing_1_overview_snapshot() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let full = SheetRect::full(doc.sheet_size(0).unwrap());
    let img = doc.render_region(0, full, 0.25).unwrap();
    assert_eq!((img.width(), img.height()), (396, 306));
    // Rendering is deterministic within one build.
    assert_eq!(img, doc.render_region(0, full, 0.25).unwrap());
    insta::assert_snapshot!("test_drawing_1_overview", fingerprint(&img, 12));
}

/// A region render must equal the same pixels cut out of a full sheet render (rule 4: one
/// coordinate system, tiles fit together).
#[test]
fn region_matches_crop_of_full_render() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let zoom = 1.5;
    let full = doc
        .render_region(0, SheetRect::full(doc.sheet_size(0).unwrap()), zoom)
        .unwrap();
    assert_eq!((full.width(), full.height()), (2376, 1836));
    // Tile at whole pixel offset (600, 450) px = (400, 300) sheet units.
    let (px, py, tw, th) = (600u32, 450u32, 256u32, 256u32);
    let region = SheetRect::new(
        f64::from(px) / zoom,
        f64::from(py) / zoom,
        f64::from(tw) / zoom,
        f64::from(th) / zoom,
    );
    let tile = doc.render_region(0, region, zoom).unwrap();
    assert_eq!((tile.width(), tile.height()), (tw, th));
    let mut max_diff = 0u8;
    let mut ink = 0usize;
    for y in 0..th {
        for x in 0..tw {
            let a = tile.pixel(x, y).unwrap();
            let b = full.pixel(px + x, py + y).unwrap();
            for c in 0..4 {
                max_diff = max_diff.max(a[c].abs_diff(b[c]));
            }
            if a[0] < 128 {
                ink += 1;
            }
        }
    }
    assert!(
        ink > 100,
        "tile should contain drawing content, found {ink} dark pixels"
    );
    assert!(max_diff <= 2, "tile differs from full render by {max_diff}");
}

/// Crops at many tile edges equal the full render. PDFium misplaces glyphs that cross the left
/// or top bitmap edge, so a grid of crops over the whole sheet catches a missing margin.
#[test]
fn region_crops_match_full_render_everywhere() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let zoom = 2.0;
    let size = doc.sheet_size(0).unwrap();
    let full = doc.render_region(0, SheetRect::full(size), zoom).unwrap();
    let step = 256u32;
    let mut worst = 0u8;
    for py in (0..full.height()).step_by(step as usize) {
        for px in (0..full.width()).step_by(step as usize) {
            let (w, h) = (step.min(full.width() - px), step.min(full.height() - py));
            let region = SheetRect::new(
                f64::from(px) / zoom,
                f64::from(py) / zoom,
                f64::from(w) / zoom,
                f64::from(h) / zoom,
            );
            let crop = doc.render_region(0, region, zoom).unwrap();
            assert_eq!((crop.width(), crop.height()), (w, h));
            for y in 0..h {
                for x in 0..w {
                    let a = crop.pixel(x, y).unwrap();
                    let b = full.pixel(px + x, py + y).unwrap();
                    for c in 0..4 {
                        worst = worst.max(a[c].abs_diff(b[c]));
                    }
                }
            }
        }
    }
    assert!(worst <= 2, "a crop differs from the full render by {worst}");
}

#[test]
fn engine_is_usable_from_many_threads() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let region = SheetRect::new(100.0, 100.0, 200.0, 150.0);
    let expected = doc.render_region(0, region, 2.0).unwrap();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..4)
            .map(|_| s.spawn(|| doc.render_region(0, region, 2.0).unwrap()))
            .collect();
        for h in handles {
            assert_eq!(h.join().unwrap(), expected);
        }
    });
}

#[test]
fn errors_are_reported() {
    let Some(engine) = engine() else { return };
    assert!(matches!(
        engine.open(b"not a pdf".to_vec()),
        Err(PdfError::Open(_))
    ));
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let r = SheetRect::new(0.0, 0.0, 10.0, 10.0);
    assert!(matches!(
        doc.render_region(1, r, 1.0),
        Err(PdfError::SheetOutOfRange { .. })
    ));
    assert!(matches!(
        doc.render_region(0, r, 0.0),
        Err(PdfError::InvalidRender(_))
    ));
    assert!(matches!(
        doc.render_region(0, SheetRect::new(0.0, 0.0, 2000.0, 10.0), 10.0),
        Err(PdfError::InvalidRender(_))
    ));
}

/// Builds a one page PDF with a correct cross reference table.
fn synthetic_pdf(page_entries: &str, content: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R {page_entries} /Contents 4 0 R /Resources << >> >>"),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ];
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

fn is_black(img: &RgbaImage, x: u32, y: u32) -> bool {
    img.pixel(x, y).unwrap()[..3].iter().all(|&c| c < 30)
}

fn is_white(img: &RgbaImage, x: u32, y: u32) -> bool {
    img.pixel(x, y).unwrap()[..3].iter().all(|&c| c > 225)
}

/// Asserts that the black area of `img` is exactly the pixel rectangle `[x0, x1) x [y0, y1)`,
/// checked at its corners and one pixel outside each edge.
fn assert_black_rect(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) {
    let (mx, my) = (u32::midpoint(x0, x1), u32::midpoint(y0, y1));
    for (x, y) in [
        (x0, y0),
        (x1 - 1, y0),
        (x0, y1 - 1),
        (x1 - 1, y1 - 1),
        (mx, my),
    ] {
        assert!(
            is_black(img, x, y),
            "({x}, {y}) should be black: {:?}",
            img.pixel(x, y)
        );
    }
    let mut outside = vec![(mx, y1), (x1, my)];
    if x0 > 0 {
        outside.push((x0 - 1, my));
    }
    if y0 > 0 {
        outside.push((mx, y0 - 1));
    }
    for (x, y) in outside {
        assert!(
            is_white(img, x, y),
            "({x}, {y}) should be white: {:?}",
            img.pixel(x, y)
        );
    }
}

/// Rule 4 on a sheet with a fractional size: one sheet unit is exactly `zoom` pixels and y
/// runs downward from the top edge.
#[test]
fn sheet_space_on_fractional_page_size() {
    let Some(engine) = engine() else { return };
    // Black rectangle at PDF x 400..450, y 100..140 (PDF y runs upward from the bottom).
    let pdf = synthetic_pdf("/MediaBox [0 0 595.9 842.9]", "0 0 0 rg 400 100 50 40 re f");
    let doc = engine.open(pdf).unwrap();
    let size = doc.sheet_size(0).unwrap();
    assert!((size.width - 595.9).abs() < 1e-3 && (size.height - 842.9).abs() < 1e-3);
    // In sheet space the rectangle is x 400..450, y 702.9..742.9.
    // Region origin (390, 690.4) at zoom 4 puts its edges on whole pixels: x 40..240, y 50..210.
    let img = doc
        .render_region(0, SheetRect::new(390.0, 690.4, 70.0, 70.0), 4.0)
        .unwrap();
    assert_eq!((img.width(), img.height()), (280, 280));
    assert_black_rect(&img, 40, 50, 240, 210);
}

/// The crop box defines the sheet: its top left corner is the sheet origin.
#[test]
fn sheet_space_follows_crop_box() {
    let Some(engine) = engine() else { return };
    let pdf = synthetic_pdf(
        "/MediaBox [0 0 600 800] /CropBox [100 50 500 750]",
        "0 0 0 rg 150 650 50 50 re f",
    );
    let doc = engine.open(pdf).unwrap();
    let size = doc.sheet_size(0).unwrap();
    assert_eq!((size.width, size.height), (400.0, 700.0));
    // PDF x 150..200, y 650..700 is sheet x 50..100, y 50..100.
    let img = doc.render_region(0, SheetRect::full(size), 1.0).unwrap();
    assert_black_rect(&img, 50, 50, 100, 100);
}

/// A rotated page is a sheet in its displayed orientation.
#[test]
fn sheet_space_follows_page_rotation() {
    let Some(engine) = engine() else { return };
    // Portrait page shown rotated 90 degrees clockwise: a landscape sheet. The square at the
    // bottom left of the unrotated page appears at the top left of the sheet.
    let pdf = synthetic_pdf(
        "/MediaBox [0 0 600 800] /Rotate 90",
        "0 0 0 rg 0 0 50 30 re f",
    );
    let doc = engine.open(pdf).unwrap();
    let size = doc.sheet_size(0).unwrap();
    assert_eq!((size.width, size.height), (800.0, 600.0));
    let img = doc.render_region(0, SheetRect::full(size), 1.0).unwrap();
    // 50 wide x 30 high on the page becomes 30 wide x 50 high on the sheet.
    assert_black_rect(&img, 0, 0, 30, 50);
}
