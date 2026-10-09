//! Ballooned PDF writer against the real PDFium library (T1.2, FR-EXP-01, FR-EXP-11, D-33).
//!
//! Skips without PDFium like `pdfium.rs`, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine in helpers too, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::truth::TruthFile;
use dimo_pdf::{
    Balloon, BalloonOutput, BalloonOverlay, BalloonShape, ContentHash, Leader, PdfDate, PdfEngine,
    PdfError, Rgb, RgbaImage, SheetBalloons, SheetPoint, SheetRect, Stroke, TextRun,
};

const TEST_DRAWING_1: &str = "test_drawing_1.pdf";

/// D-24 outline color.
const BLUE: Rgb = Rgb::new(0x00, 0x57, 0xB8);

const DATE: PdfDate = PdfDate {
    year: 2026,
    month: 10,
    day: 9,
    hour: 12,
    minute: 0,
    second: 0,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_drawing(name: &str) -> Vec<u8> {
    let path = repo_root().join("corpus/drawings").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

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

/// One balloon per truth characteristic of `test_drawing_1`, numbered 1 to 24, up and right of
/// the callout with a leader to its top right corner. Shapes cycle circle, rectangle, flag.
fn test_drawing_1_balloons() -> Vec<Balloon> {
    let path = repo_root().join("corpus/truth/test_drawing_1.truth.json");
    let truth = TruthFile::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    truth
        .characteristics
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let r = &c.region;
            let corner = SheetPoint::new(
                r.center.x + r.size.width / 2.0,
                r.center.y - r.size.height / 2.0,
            );
            let shape = [
                BalloonShape::Circle,
                BalloonShape::Rectangle,
                BalloonShape::Flag,
            ][i % 3];
            let (width, height) = match shape {
                BalloonShape::Circle => (20.0, 20.0),
                _ => (28.0, 16.0),
            };
            Balloon {
                shape,
                center: SheetPoint::new(corner.x + 18.0, corner.y - 16.0),
                width,
                height,
                fill: Some(Rgb::WHITE),
                outline: Some(Stroke {
                    color: BLUE,
                    width: 1.0,
                }),
                text: (i + 1).to_string(),
                text_color: Rgb::BLACK,
                text_size: 10.0,
                leader: Some(Leader {
                    anchor: corner,
                    stroke: Stroke {
                        color: BLUE,
                        width: 0.75,
                    },
                }),
            }
        })
        .collect()
}

fn overlay(balloons: Vec<Balloon>, output: BalloonOutput) -> BalloonOverlay {
    BalloonOverlay {
        sheets: vec![SheetBalloons { sheet: 0, balloons }],
        output,
    }
}

fn ballooned(engine: &PdfEngine, output: BalloonOutput) -> Vec<u8> {
    engine
        .write_ballooned(
            corpus_drawing(TEST_DRAWING_1),
            overlay(test_drawing_1_balloons(), output),
        )
        .unwrap()
}

fn texts(runs: &[TextRun]) -> Vec<&str> {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/// Raw stream payloads (between `stream` and `endstream`) of a PDF without object streams.
fn stream_payloads(pdf: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = find(&pdf[at..], b"stream") {
        let start = at + i + b"stream".len();
        if pdf[..at + i].ends_with(b"end") {
            at = start;
            continue;
        }
        let start = start + usize::from(pdf[start] == b'\r') + 1;
        let end = start + find(&pdf[start..], b"endstream").unwrap();
        let payload = pdf[start..end]
            .strip_suffix(b"\n")
            .unwrap_or(&pdf[start..end]);
        out.push(payload.strip_suffix(b"\r").unwrap_or(payload));
        at = end + b"endstream".len();
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

/// Acceptance 1: the ballooned drawing opens in PDFium, has the same sheet, and every original
/// text run is still extractable, in order, followed by the balloon numbers.
#[test]
fn ballooned_test_drawing_1_keeps_text() {
    let Some(engine) = engine() else { return };
    let original = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let before = original.text_runs(0).unwrap();
    assert!(before.len() > 50);

    let doc = engine
        .open(ballooned(&engine, BalloonOutput::PageContent))
        .unwrap();
    assert_eq!(doc.sheet_sizes(), original.sheet_sizes());
    let after = doc.text_runs(0).unwrap();
    assert_eq!(after[..before.len()], before[..]);
    // PDFium orders text objects on one line by x, so compare as sets, matched by number.
    let balloons = test_drawing_1_balloons();
    let mut numbers = texts(&after[before.len()..]);
    numbers.sort_by_key(|t| t.parse::<usize>().unwrap());
    let expected: Vec<String> = (1..=balloons.len()).map(|n| n.to_string()).collect();
    assert_eq!(numbers, expected);
    for run in &after[before.len()..] {
        let balloon = &balloons[run.text.parse::<usize>().unwrap() - 1];
        assert_eq!(run.font_name, "OpenSans-Bold");
        assert_eq!(run.rotation, 0.0);
        assert!((run.font_size - 10.0).abs() < 1e-3, "{run:?}");
        // The number is centered on the balloon.
        let (cx, cy) = (
            run.bbox.x + run.bbox.width / 2.0,
            run.bbox.y + run.bbox.height / 2.0,
        );
        assert!((cx - balloon.center.x).abs() < 0.01, "{run:?}");
        assert!((cy - balloon.center.y).abs() < 1.5, "{run:?}");
    }

    // Annotations are not page text: the runs are exactly the original ones.
    let annotated = engine
        .open(ballooned(
            &engine,
            BalloonOutput::Annotations { date: DATE },
        ))
        .unwrap();
    assert_eq!(annotated.text_runs(0).unwrap(), before);
}

/// FR-EXP-01: the original content streams are copied unchanged into the ballooned file.
#[test]
fn original_content_is_unchanged() {
    let Some(engine) = engine() else { return };
    let input = corpus_drawing(TEST_DRAWING_1);
    let streams = stream_payloads(&input);
    // Two content streams, the font program and two font tables.
    assert_eq!(streams.len(), 4);
    for output in [
        BalloonOutput::PageContent,
        BalloonOutput::Annotations { date: DATE },
    ] {
        let out = ballooned(&engine, output);
        for (i, stream) in streams.iter().enumerate() {
            assert!(
                find(&out, stream).is_some(),
                "stream {i} changed ({output:?})"
            );
        }
    }
}

/// Acceptance 2, FR-EXP-11: two runs produce byte identical files, in both modes. Dates and
/// IDs come only from the input.
#[test]
fn two_runs_are_byte_identical() {
    let Some(engine) = engine() else { return };
    let content = ballooned(&engine, BalloonOutput::PageContent);
    assert_eq!(content, ballooned(&engine, BalloonOutput::PageContent));
    let annotations = ballooned(&engine, BalloonOutput::Annotations { date: DATE });
    assert_eq!(
        annotations,
        ballooned(&engine, BalloonOutput::Annotations { date: DATE })
    );
    // test_drawing_1 has no /ID, so both parts are derived: the first from the input.
    let first = ContentHash::of(&corpus_drawing(TEST_DRAWING_1)).to_hex()[..32].to_uppercase();
    assert_eq!(count(&content, format!("/ID[<{first}><").as_bytes()), 1);
    // The only dates are the input's creation date and, for annotations, the given date.
    assert_eq!(count(&content, b"(D:"), 1);
    assert_eq!(count(&annotations, b"(D:"), 1 + 2 * 24);
    assert_eq!(count(&annotations, b"(D:20261009120000Z00'00')"), 2 * 24);
    assert_eq!(count(&annotations, b"/Subtype/Stamp"), 24);
}

/// The embedded font is a tagged subset with only the characters used.
#[test]
fn font_is_embedded_as_subset() {
    let Some(engine) = engine() else { return };
    let out = ballooned(&engine, BalloonOutput::PageContent);
    let input = corpus_drawing(TEST_DRAWING_1);
    assert_eq!(count(&out, b"/FontFile2"), count(&input, b"/FontFile2") + 1);
    let base_font = find(&out, b"+OpenSans-Bold").unwrap();
    let tag = &out[base_font - 6..base_font];
    assert!(tag.iter().all(u8::is_ascii_uppercase), "{tag:?}");
    // The whole file grows by far less than the 104 KB font program.
    let growth = out.len() - input.len();
    assert!(growth < 16_000, "{growth}");
}

/// Coarse fingerprint of a rendered image, same method and limits as in `pdfium.rs`
/// (see there): 4 ink levels per block, hashed. The limits were chosen for the plain drawing;
/// `fingerprint_margin` reports how far the ballooned render is from them.
fn fingerprint(img: &RgbaImage, cell: u32) -> String {
    let cols = img.width().div_ceil(cell);
    let rows = img.height().div_ceil(cell);
    let mut grid = String::new();
    for row in block_ink(img, cell).chunks(cols as usize) {
        grid.push('|');
        for &(ink, count) in row {
            // Mean ink below `limit` hundredths, compared exactly in integers.
            let below = |limit: u64| ink * 100 < limit * count;
            grid.push(if below(250) {
                ' '
            } else if below(1925) {
                '.'
            } else if below(8200) {
                '+'
            } else {
                '#'
            });
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

/// Summed ink (255 minus luminance) and pixel count per block, row by row.
fn block_ink(img: &RgbaImage, cell: u32) -> Vec<(u64, u64)> {
    let mut blocks = Vec::new();
    for gy in 0..img.height().div_ceil(cell) {
        for gx in 0..img.width().div_ceil(cell) {
            let (mut ink, mut count) = (0u64, 0u64);
            for y in gy * cell..((gy + 1) * cell).min(img.height()) {
                for x in gx * cell..((gx + 1) * cell).min(img.width()) {
                    let [r, g, b, _] = img.pixel(x, y).unwrap();
                    let lum = (299 * u64::from(r) + 587 * u64::from(g) + 114 * u64::from(b)) / 1000;
                    ink += 255 - lum;
                    count += 1;
                }
            }
            blocks.push((ink, count));
        }
    }
    blocks
}

/// Smallest distance of any block's mean ink to a level limit.
fn fingerprint_margin(img: &RgbaImage, cell: u32) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    block_ink(img, cell)
        .iter()
        .map(|&(ink, count)| {
            let mean = ink as f64 / count as f64;
            [2.5, 19.25, 82.0]
                .iter()
                .map(|limit| (mean - limit).abs())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Acceptance 3: the rendered ballooned sheet is stable. Annotation mode looks the same.
#[test]
fn ballooned_sheet_snapshot() {
    let Some(engine) = engine() else { return };
    let doc = engine
        .open(ballooned(&engine, BalloonOutput::PageContent))
        .unwrap();
    let full = SheetRect::full(doc.sheet_size(0).unwrap());
    let img = doc.render_region(0, full, 0.25).unwrap();
    assert_eq!((img.width(), img.height()), (396, 306));
    let margin = fingerprint_margin(&img, 12);
    assert!(
        margin > 0.3,
        "a block is {margin} mean ink from a level limit"
    );
    let print = fingerprint(&img, 12);
    insta::assert_snapshot!("test_drawing_1_ballooned", print);

    let annotated = engine
        .open(ballooned(
            &engine,
            BalloonOutput::Annotations { date: DATE },
        ))
        .unwrap();
    let img = annotated.render_region(0, full, 0.25).unwrap();
    assert_eq!(fingerprint(&img, 12), print);
}

/// Builds a one page PDF with a correct cross reference table and the given page entries.
fn synthetic_pdf(page_entries: &str, trailer_extra: &str) -> Vec<u8> {
    let content = "0 0 0 rg 10 10 5 5 re f";
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
            "trailer\n<< /Size {} /Root 1 0 R {trailer_extra}>>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A black rectangle balloon with the sheet box x 100..140, y 50..70 and a centered number.
fn black_box() -> Balloon {
    Balloon {
        shape: BalloonShape::Rectangle,
        center: SheetPoint::new(120.0, 60.0),
        width: 40.0,
        height: 20.0,
        fill: Some(Rgb::BLACK),
        outline: None,
        text: "7".to_owned(),
        text_color: Rgb::WHITE,
        text_size: 12.0,
        leader: None,
    }
}

fn is_black(img: &RgbaImage, x: u32, y: u32) -> bool {
    img.pixel(x, y).unwrap()[..3].iter().all(|&c| c < 30)
}

fn is_white(img: &RgbaImage, x: u32, y: u32) -> bool {
    img.pixel(x, y).unwrap()[..3].iter().all(|&c| c > 225)
}

/// Rule 4: balloons are placed in sheet space on cropped and rotated pages, and their numbers
/// read upright on the sheet.
#[test]
fn balloons_follow_crop_box_and_rotation() {
    let Some(engine) = engine() else { return };
    for page in [
        "/MediaBox [0 0 600 800]",
        "/MediaBox [0 0 600 800] /CropBox [100 50 500 750]",
        "/MediaBox [0 0 600 800] /Rotate 90",
        "/MediaBox [0 0 600 800] /CropBox [100 50 500 750] /Rotate 180",
        "/MediaBox [0 0 600 800] /Rotate 270",
    ] {
        for output in [
            BalloonOutput::PageContent,
            BalloonOutput::Annotations { date: DATE },
        ] {
            let out = engine
                .write_ballooned(synthetic_pdf(page, ""), overlay(vec![black_box()], output))
                .unwrap();
            let doc = engine.open(out).unwrap();
            let img = doc
                .render_region(0, SheetRect::new(90.0, 40.0, 60.0, 40.0), 4.0)
                .unwrap();
            // Box edges at pixels x 40..200, y 40..120; corners black, outside white.
            for (x, y) in [(41, 41), (198, 41), (41, 118), (198, 118)] {
                assert!(is_black(&img, x, y), "{page} {output:?} ({x}, {y})");
            }
            for (x, y) in [(38, 80), (202, 80), (120, 38), (120, 122)] {
                assert!(is_white(&img, x, y), "{page} {output:?} ({x}, {y})");
            }
            if output == BalloonOutput::PageContent {
                let runs = doc.text_runs(0).unwrap();
                let run = runs.iter().find(|r| r.text == "7").unwrap();
                assert_eq!(run.rotation, 0.0, "{page}");
                let cx = run.bbox.x + run.bbox.width / 2.0;
                assert!((cx - 120.0).abs() < 0.01, "{page} {run:?}");
            }
        }
    }
}

/// A file identifier from the input is kept; the changing part is derived.
#[test]
fn input_file_id_is_kept() {
    let Some(engine) = engine() else { return };
    let input = synthetic_pdf(
        "/MediaBox [0 0 600 800]",
        "/ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] ",
    );
    let request = overlay(vec![black_box()], BalloonOutput::PageContent);
    let out = engine
        .write_ballooned(input.clone(), request.clone())
        .unwrap();
    assert_eq!(out, engine.write_ballooned(input, request).unwrap());
    let at = find(&out, b"/ID[").unwrap();
    let id = String::from_utf8_lossy(&out[at..at + 4 + 34 + 34]);
    assert!(
        id.starts_with("/ID[<00112233445566778899AABBCCDDEEFF><"),
        "{id}"
    );
    assert!(!id.ends_with("<00112233445566778899AABBCCDDEEFF>"), "{id}");
}

#[test]
fn invalid_requests_are_rejected() {
    let Some(engine) = engine() else { return };
    let input = synthetic_pdf("/MediaBox [0 0 600 800]", "");
    let mut out_of_range = overlay(vec![black_box()], BalloonOutput::PageContent);
    out_of_range.sheets[0].sheet = 1;
    assert!(matches!(
        engine.write_ballooned(input.clone(), out_of_range),
        Err(PdfError::SheetOutOfRange { index: 1, count: 1 })
    ));
    let mut bad = black_box();
    bad.text = "\u{D8}1".to_owned();
    assert!(matches!(
        engine.write_ballooned(
            input.clone(),
            overlay(vec![bad], BalloonOutput::PageContent)
        ),
        Err(PdfError::InvalidOverlay(_))
    ));
    assert!(matches!(
        engine.write_ballooned(
            b"not a pdf".to_vec(),
            overlay(vec![], BalloonOutput::PageContent)
        ),
        Err(PdfError::Open(_))
    ));
    // Nothing to draw still gives a valid, deterministic copy.
    let empty = engine
        .write_ballooned(input, overlay(vec![], BalloonOutput::PageContent))
        .unwrap();
    assert_eq!(engine.open(empty).unwrap().sheet_count(), 1);
}
