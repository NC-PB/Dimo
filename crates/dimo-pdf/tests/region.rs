//! Region queries for box select against the real PDFium library (T2.6, FR-REC-01).
//!
//! Skips without PDFium like `pdfium.rs`, fails instead with `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1`.

#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::truth::TruthFile;
use dimo_pdf::{PdfEngine, PdfError};

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
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

fn boxed(x: f64, y: f64, width: f64, height: f64, angle: f64) -> OrientedBox {
    OrientedBox {
        center: Point { x, y },
        size: Size { width, height },
        angle,
    }
}

/// A one page PDF of 612 x 792 units with Helvetica as `/F1` and `content` as its stream.
fn pdf(content: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
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

/// FR-REC-01 on the corpus: the truth region of `Ø30 H7 +0.0203 -0` holds exactly its three
/// runs, the main text and the stacked deviations.
#[test]
fn test_drawing_1_region_of_a_stacked_callout() {
    let Some(engine) = engine() else { return };
    let bytes = std::fs::read(repo_root().join("corpus/drawings/test_drawing_1.pdf")).unwrap();
    let doc = engine.open(bytes).unwrap();
    let truth = TruthFile::from_json_str(
        &std::fs::read_to_string(repo_root().join("corpus/truth/test_drawing_1.truth.json"))
            .unwrap(),
    )
    .unwrap();
    let c03 = truth
        .characteristics
        .iter()
        .find(|c| c.id == "c03")
        .unwrap();
    let region = doc.region_text(0, c03.region).unwrap();
    let mut texts: Vec<&str> = region.runs.iter().map(|r| r.run.text.as_str()).collect();
    texts.sort_unstable();
    assert_eq!(texts, ["+0.0203", "-0", "Ø30 H7"]);
    assert!(
        region
            .runs
            .iter()
            .all(|r| r.frame.angle == 0.0 && !r.framed)
    );
    assert_ne!(region.chars.len(), 0);

    // An empty part of the sheet has nothing.
    let empty = doc
        .region_text(0, boxed(20.0, 20.0, 10.0, 10.0, 0.0))
        .unwrap();
    assert!(empty.runs.is_empty() && empty.chars.is_empty());
}

/// Rotated text and a basic dimension frame (M2 decision 6), drawn by hand.
#[test]
fn rotated_text_and_frames() {
    let Some(engine) = engine() else { return };
    // "25" at baseline y 700 (sheet y 92) inside a stroked rectangle; "R15" read bottom to
    // top at x 300 from y 500 (sheet y 292) upward; "40" with a rectangle far around it.
    let content = "BT /F1 12 Tf 100 700 Td (25) Tj ET \
                   0.5 w 96 694 22 18 re S \
                   BT /F1 12 Tf 0 1 -1 0 300 500 Tm (R15) Tj ET \
                   BT /F1 12 Tf 400 700 Td (40) Tj ET \
                   0.5 w 350 650 150 100 re S";
    let doc = engine.open(pdf(content)).unwrap();

    let basic = doc
        .region_text(0, boxed(107.0, 88.0, 30.0, 24.0, 0.0))
        .unwrap();
    assert_eq!(basic.runs.len(), 1);
    assert_eq!(basic.runs[0].run.text, "25");
    assert!(basic.runs[0].framed, "{basic:#?}");

    let loose = doc
        .region_text(0, boxed(407.0, 88.0, 30.0, 24.0, 0.0))
        .unwrap();
    assert_eq!(loose.runs[0].run.text, "40");
    assert!(!loose.runs[0].framed, "{loose:#?}");

    // The vertical text: an axis aligned box around it on the sheet.
    let rotated = doc
        .region_text(0, boxed(296.0, 280.0, 24.0, 40.0, 0.0))
        .unwrap();
    assert_eq!(rotated.runs.len(), 1, "{rotated:#?}");
    let run = &rotated.runs[0];
    assert_eq!(run.run.text, "R15");
    assert!((run.frame.angle - 90.0).abs() < 0.01, "{run:#?}");
    assert!(run.frame.size.width > run.frame.size.height, "{run:#?}");
}
