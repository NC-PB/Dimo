//! Text runs and sheet classification against the real PDFium library (T0.6).
//!
//! Skips without PDFium like `pdfium.rs`, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine in helpers too, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::fmt::Write as _;
use std::path::PathBuf;

use dimo_core::sheet::SheetKind;
use dimo_core::truth::TruthFile;
use dimo_pdf::{FontInfo, PdfEngine, PdfError, SheetRect, TextRun};

const TEST_DRAWING_1: &str = "test_drawing_1.pdf";

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
}

fn corpus_drawing(name: &str) -> Vec<u8> {
    let path = repo_root().join("corpus/drawings").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn truth(name: &str) -> TruthFile {
    let path = repo_root().join("corpus/truth").join(name);
    TruthFile::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap()
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

fn center(r: &SheetRect) -> (f64, f64) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

fn contains_point(r: &SheetRect, (x, y): (f64, f64)) -> bool {
    x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height
}

/// Whether `inner` lies within `outer` grown by `margin` on every side.
fn within(inner: &SheetRect, outer: &SheetRect, margin: f64) -> bool {
    inner.x >= outer.x - margin
        && inner.y >= outer.y - margin
        && inner.x + inner.width <= outer.x + outer.width + margin
        && inner.y + inner.height <= outer.y + outer.height + margin
}

// ---------------------------------------------------------------------------------------------
// test_drawing_1 (corpus regression, docs/dev/testing.md)

/// FR-DOC-03: the sheet has a real text layer that some libraries miss (corpus notes, Pitfall).
#[test]
fn test_drawing_1_is_vector_text() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    let expected = truth("test_drawing_1.truth.json").sheets[0].kind;
    assert_eq!(expected, SheetKind::VectorText);
    assert_eq!(analysis.kind, expected, "{analysis:#?}");
    assert_eq!(analysis.text_objects, 74);
    assert_eq!(analysis.invisible_text_objects, 0);
    assert_eq!(
        analysis.fonts,
        [FontInfo {
            name: "NotoSans".to_owned(),
            embedded: true,
            text_objects: 74,
        }]
    );
    assert_eq!(analysis.chars, 380);
    assert_eq!(analysis.unmapped_chars, 0);
    assert_eq!(analysis.image_objects, 0);
    assert!(analysis.path_objects > 100, "{analysis:#?}");
    assert!(analysis.image_coverage.abs() < f64::EPSILON);
}

/// Every callout of the truth file is found as runs inside its region, with the exact text.
///
/// The truth `requirement_text` joins a callout's text objects with one space: main text, then
/// upper deviation, then lower deviation. Runs whose center lies in the region are ordered the
/// same way (left to right, top to bottom within one column) and joined.
#[test]
fn test_drawing_1_callouts_match_truth_regions() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let runs = doc.text_runs(0).unwrap();
    let truth = truth("test_drawing_1.truth.json");
    let mut matched_runs = 0;
    for c in &truth.characteristics {
        let r = &c.region;
        assert!(
            r.angle.abs() < f64::EPSILON,
            "{}: rotated truth region",
            c.id
        );
        let region = SheetRect::new(
            r.center.x - r.size.width / 2.0,
            r.center.y - r.size.height / 2.0,
            r.size.width,
            r.size.height,
        );
        let mut inside: Vec<&TextRun> = runs
            .iter()
            .filter(|run| contains_point(&region, center(&run.bbox)))
            .collect();
        // Column first (4 unit buckets absorb sub point jitter of stacked deviations), then top
        // to bottom.
        #[allow(clippy::cast_possible_truncation)]
        inside.sort_by_key(|run| {
            (
                (run.bbox.x / 4.0).round() as i64,
                (run.bbox.y * 100.0).round() as i64,
            )
        });
        let joined: Vec<&str> = inside.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(joined.join(" "), c.requirement_text, "{}", c.id);
        for run in &inside {
            assert!(
                within(&run.bbox, &region, 0.5),
                "{}: run {:?} at {:?} exceeds truth region {region:?}",
                c.id,
                run.text,
                run.bbox
            );
            assert!(run.rotation.abs() < 1e-6, "{}: {run:?}", c.id);
        }
        matched_runs += inside.len();
    }
    assert_eq!(truth.characteristics.len(), 24);
    assert_eq!(matched_runs, 33);
}

/// The callouts listed in `corpus/notes/test_drawing_1.md` exist as separate runs, symbols as
/// printed: `Ø` U+00D8, `°`, `±`, Unicode minus U+2212 next to hyphen minus.
#[test]
fn test_drawing_1_callouts_are_runs() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let runs = doc.text_runs(0).unwrap();
    let count = |text: &str| runs.iter().filter(|r| r.text == text).count();
    let expected = [
        ("\u{D8}30 H7", 1),
        ("+0.0203", 1),
        ("-0", 1),
        ("\u{D8}8 f7", 3),
        ("-0.0127", 1),
        ("-0.0279", 1),
        ("\u{D8}8 h6", 1),
        ("\u{D8}8 d9", 1),
        ("\u{D8}8 c10", 1),
        ("R15 H7", 1),
        ("100", 2),
        ("+0", 1),
        ("\u{2212}0.6", 1),
        ("90.0\u{B0}", 2),
        ("+0.0\u{B0}", 1),
        ("\u{2212}0.1\u{B0}", 1),
        ("50\u{B1}0.1", 1),
        ("12.39\u{B1}0.1", 1),
        ("200", 1),
        ("50", 1),
        ("40", 1),
        ("31.05", 1),
        ("52.61", 1),
        ("71.04", 1),
        ("86.16", 1),
        // Title block
        ("ANGULAR = \u{B1} \u{B0}", 1),
        ("SHEET", 1),
        ("1 of 1", 1),
    ];
    for (text, n) in expected {
        assert_eq!(count(text), n, "{text:?} in {:#?}", texts(&runs));
    }
    // Zone labels 1 to 4 and A to D on both frame sides, "30" and "20" are callouts too.
    assert_eq!(count("30"), 1);
    assert_eq!(count("20"), 1);
    assert_eq!(count("A"), 2);
    assert!(runs.iter().all(|r| r.font_name == "NotoSans"));
}

/// All runs of the sheet in content order. Geometry is checked against the truth regions
/// above; the snapshot covers text, rotation, font and size so a change in merging shows.
#[test]
fn test_drawing_1_runs_snapshot() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    let runs = doc.text_runs(0).unwrap();
    let mut out = format!("{} runs\n", runs.len());
    for run in &runs {
        writeln!(
            out,
            "{:?} rot {:.0} {} {:.1}",
            run.text, run.rotation, run.font_name, run.font_size
        )
        .unwrap();
    }
    insta::assert_snapshot!("test_drawing_1_runs", out);
}

fn texts(runs: &[TextRun]) -> Vec<&str> {
    runs.iter().map(|r| r.text.as_str()).collect()
}

// ---------------------------------------------------------------------------------------------
// Synthetic PDFs

/// Builds a PDF with a correct cross reference table. Object 1 is the catalog, 2 the page
/// tree, 3 the page; `page` is inserted into the page dictionary, `content` is object 4 and
/// `extra` objects follow from number 5.
fn pdf(page: &str, content: &str, extra: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R {page} /Contents 4 0 R >>"),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ];
    objects.extend(extra.iter().map(|s| (*s).to_owned()));
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

const HELVETICA: &str = "/Resources << /Font << /F1 5 0 R >> >>";
const HELVETICA_FONT: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

/// Bounding box of dark pixels of a full sheet render at zoom 1, in sheet units.
fn ink_box(doc: &dimo_pdf::Document) -> SheetRect {
    let size = doc.sheet_size(0).unwrap();
    let img = doc.render_region(0, SheetRect::full(size), 1.0).unwrap();
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.height() {
        for x in 0..img.width() {
            if img.pixel(x, y).unwrap()[0] < 128 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    assert!(x0 < x1, "no ink on the sheet");
    SheetRect::new(
        f64::from(x0),
        f64::from(y0),
        f64::from(x1 - x0),
        f64::from(y1 - y0),
    )
}

/// The one run of a synthetic sheet must cover the rendered text (rule 4: text geometry and
/// tiles share one coordinate system) and be no more than half an em larger on any side.
fn assert_run_matches_ink(page: &str, content: &str, text: &str, rotation: f64) {
    let engine = engine().unwrap();
    let doc = engine.open(pdf(page, content, &[HELVETICA_FONT])).unwrap();
    let runs = doc.text_runs(0).unwrap();
    assert_eq!(texts(&runs), [text], "{page}");
    let run = &runs[0];
    assert!((run.rotation - rotation).abs() < 1e-3, "{page}: {run:?}");
    assert_eq!(run.font_name, "Helvetica");
    assert!((run.font_size - 40.0).abs() < 1e-3, "{run:?}");
    let ink = ink_box(&doc);
    assert!(
        within(&ink, &run.bbox, 1.0),
        "{page}: ink {ink:?} run {run:?}"
    );
    assert!(
        within(&run.bbox, &ink, 20.0),
        "{page}: ink {ink:?} run {run:?}"
    );
}

#[test]
fn text_runs_follow_page_rotation() {
    if engine().is_none() {
        return;
    }
    let content = "BT /F1 40 Tf 100 600 Td (HELLO) Tj ET";
    for (rotate, rotation) in [(0, 0.0), (90, 270.0), (180, 180.0), (270, 90.0)] {
        let page = format!("/MediaBox [0 0 600 800] /Rotate {rotate} {HELVETICA}");
        assert_run_matches_ink(&page, content, "HELLO", rotation);
    }
}

#[test]
fn text_runs_follow_crop_box() {
    if engine().is_none() {
        return;
    }
    let page = format!("/MediaBox [0 0 600 800] /CropBox [50 100 550 700] {HELVETICA}");
    assert_run_matches_ink(&page, "BT /F1 40 Tf 100 600 Td (HELLO) Tj ET", "HELLO", 0.0);
}

#[test]
fn text_runs_follow_text_matrix_rotation() {
    if engine().is_none() {
        return;
    }
    // Text matrix rotated 90 degrees counterclockwise: reads bottom to top.
    let page = format!("/MediaBox [0 0 600 800] {HELVETICA}");
    let content = "BT /F1 40 Tf 0 1 -1 0 300 200 Tm (UP) Tj ET";
    assert_run_matches_ink(&page, content, "UP", 90.0);
}

/// Type0 font with `Identity-H`, not embedded, two byte codes equal to Unicode code points,
/// like `test_drawing_1`. `to_unicode` is put into the font dictionary.
fn identity_h_pdf(to_unicode: &str, codes: &str) -> Vec<u8> {
    let content = format!("BT /F1 20 Tf 50 100 Td <{codes}> Tj ET");
    let cmap = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
                /CMapName /Test def 1 begincodespacerange <0000> <FFFF> endcodespacerange \
                1 beginbfchar <0041> <005A> endbfchar endcmap \
                CMapName currentdict /CMap defineresource pop end end";
    let font = format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /Arial /Encoding /Identity-H {to_unicode} \
         /DescendantFonts [6 0 R] >>"
    );
    let cid_font = "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Arial \
                    /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                    /DW 600 >>";
    let cmap_stream = format!("<< /Length {} >>\nstream\n{cmap}\nendstream", cmap.len());
    pdf(
        "/MediaBox [0 0 400 200] /Resources << /Font << /F1 5 0 R >> >>",
        &content,
        &[&font, cid_font, &cmap_stream],
    )
}

/// How PDFium decodes `Identity-H` text (findings in `corpus/notes/test_drawing_1.md`): without
/// a usable `/ToUnicode` `CMap` stream (missing, or a name such as `/Identity-H`) the two byte
/// code itself becomes the Unicode value. A real `CMap` stream wins.
#[test]
fn identity_h_without_to_unicode_passes_codes_through() {
    let Some(engine) = engine() else { return };
    // A, U+2212, Ø, 1, 2, 3, 4, 5, 6, 7, 8, 9
    let codes = "0041221200D8003100320033003400350036003700380039";
    for (to_unicode, expected) in [
        ("", "A\u{2212}\u{D8}123456789"),
        ("/ToUnicode /Identity-H", "A\u{2212}\u{D8}123456789"),
        ("/ToUnicode 7 0 R", "Z\u{2212}\u{D8}123456789"),
    ] {
        let doc = engine.open(identity_h_pdf(to_unicode, codes)).unwrap();
        assert_eq!(
            texts(&doc.text_runs(0).unwrap()),
            [expected],
            "{to_unicode}"
        );
        let analysis = doc.analyze_sheet(0).unwrap();
        assert_eq!(analysis.kind, SheetKind::VectorText, "{analysis:#?}");
        assert_eq!(analysis.unmapped_chars, 0);
    }
}

/// When the codes are glyph ids instead of code points (common in CAD exports without
/// `/ToUnicode`), the pass through yields control characters: the text exists in the content
/// stream but cannot be read, so the sheet goes the OCR path.
#[test]
fn undecodable_text_is_classified_for_ocr() {
    let Some(engine) = engine() else { return };
    let codes = "000100020003000400050006000700080010001100120013001400150016";
    let doc = engine.open(identity_h_pdf("", codes)).unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    assert_eq!(analysis.text_objects, 1);
    assert_eq!(analysis.chars, 15, "{analysis:#?}");
    assert_eq!(analysis.unmapped_chars, 15, "{analysis:#?}");
    assert_eq!(analysis.kind, SheetKind::VectorOutlined);
}

#[test]
fn paths_without_text_are_vector_outlined() {
    let Some(engine) = engine() else { return };
    // Small closed paths, like glyphs exported as curves.
    let mut content = String::from("0 0 0 rg\n");
    for i in 0..50 {
        writeln!(content, "{} 100 6 9 re f", 20 + i * 7).unwrap();
    }
    let doc = engine
        .open(pdf("/MediaBox [0 0 400 200]", &content, &[]))
        .unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    assert_eq!(analysis.kind, SheetKind::VectorOutlined);
    assert_eq!(analysis.text_objects, 0);
    assert_eq!(analysis.path_objects, 50);
    assert_eq!(doc.text_runs(0).unwrap(), Vec::new());
}

#[test]
fn scan_with_invisible_ocr_layer_is_raster() {
    let Some(engine) = engine() else { return };
    // A page filling image with an invisible text layer (render mode 3), as scanners write it.
    let content = "q 400 0 0 200 0 0 cm BI /W 2 /H 2 /CS /G /BPC 8 /F /AHx ID 00FFFF00> EI Q\n\
                   BT 3 Tr /F1 12 Tf 20 100 Td (DIMENSIONS ARE IN MILLIMETERS) Tj ET";
    let page = format!("/MediaBox [0 0 400 200] {HELVETICA}");
    let doc = engine.open(pdf(&page, content, &[HELVETICA_FONT])).unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    assert_eq!(analysis.kind, SheetKind::Raster, "{analysis:#?}");
    assert_eq!(analysis.image_objects, 1);
    assert!((analysis.image_coverage - 1.0).abs() < 1e-3);
    assert_eq!(analysis.text_objects, 1);
    assert_eq!(analysis.invisible_text_objects, 1);
    assert_eq!(analysis.chars, 0);
    assert_eq!(doc.text_runs(0).unwrap(), Vec::new());
}

#[test]
fn scan_inside_form_xobject_is_raster() {
    let Some(engine) = engine() else { return };
    // The image sits in a form XObject that the page scales to the full sheet.
    let form_content = "BI /W 2 /H 2 /CS /G /BPC 8 /F /AHx ID 00FFFF00> EI";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length {} >>\nstream\n{form_content}\nendstream",
        form_content.len()
    );
    let doc = engine
        .open(pdf(
            "/MediaBox [0 0 400 200] /Resources << /XObject << /Fm1 5 0 R >> >>",
            "q 400 0 0 200 0 0 cm /Fm1 Do Q",
            &[&form],
        ))
        .unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    assert_eq!(analysis.kind, SheetKind::Raster, "{analysis:#?}");
    assert_eq!(analysis.image_objects, 1);
    assert!(
        (analysis.image_coverage - 1.0).abs() < 1e-3,
        "{analysis:#?}"
    );
}

#[test]
fn a_few_characters_are_not_a_text_layer() {
    let Some(engine) = engine() else { return };
    let page = format!("/MediaBox [0 0 400 200] {HELVETICA}");
    let doc = engine
        .open(pdf(
            &page,
            "BT /F1 12 Tf 20 20 Td (Page 1) Tj ET",
            &[HELVETICA_FONT],
        ))
        .unwrap();
    let analysis = doc.analyze_sheet(0).unwrap();
    assert_eq!(analysis.chars, 5);
    assert_eq!(analysis.kind, SheetKind::VectorOutlined);
    assert_eq!(texts(&doc.text_runs(0).unwrap()), ["Page 1"]);
}

#[test]
fn sheet_index_is_checked() {
    let Some(engine) = engine() else { return };
    let doc = engine.open(corpus_drawing(TEST_DRAWING_1)).unwrap();
    assert!(matches!(
        doc.text_runs(1),
        Err(PdfError::SheetOutOfRange { index: 1, count: 1 })
    ));
    assert!(matches!(
        doc.analyze_sheet(7),
        Err(PdfError::SheetOutOfRange { index: 7, count: 1 })
    ));
}
