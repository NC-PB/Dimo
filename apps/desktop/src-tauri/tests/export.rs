//! Exports of an open project (T1.9): ballooned PDF, CSV and XLSX from a session snapshot,
//! deterministic (FR-EXP-11), with the balloon layout shared with the viewport (D-24).
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::{BalloonShape, BalloonStyleOverride, Command, FieldValue, Point};
use dimo_desktop::export::{ExportFormat, ExportRequest, render};
use dimo_desktop::session::AppSession;
use dimo_desktop::settings::{PdfBalloons, ReportLanguage};
use dimo_pdf::tiles::{TileConfig, TileService};
use dimo_pdf::{PdfEngine, PdfError};

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn service() -> Option<TileService> {
    match PdfEngine::start() {
        Ok(engine) => Some(TileService::new(engine, TileConfig::default())),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn drawing() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../../corpus/drawings/test_drawing_1.pdf")
}

fn request(format: ExportFormat) -> ExportRequest {
    ExportRequest {
        format,
        language: ReportLanguage::En,
        pdf_balloons: PdfBalloons::PageContent,
        issued: false,
    }
}

/// A project on `test_drawing_1` with three balloons, the second one a flag.
fn session(tiles: &TileService) -> AppSession {
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    for (x, text) in [(150.0, "Ø8 f7"), (270.0, "R15"), (390.0, "30 ±0.1")] {
        session
            .execute(Command::AddCharacteristic {
                sheet,
                position: Point { x, y: 120.0 },
                anchor: Point {
                    x: x + 25.0,
                    y: 150.0,
                },
                region: None,
                values: vec![FieldValue::RequirementText(text.into())],
            })
            .unwrap();
    }
    let flag = session.project().unwrap().balloons[1].id;
    session
        .execute(Command::RestyleBalloons {
            ids: vec![flag],
            style: BalloonStyleOverride {
                shape: Some(BalloonShape::Flag),
                ..Default::default()
            },
        })
        .unwrap();
    session
}

#[test]
fn ballooned_pdf_has_the_numbers_and_is_deterministic() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let snapshot = session.export_snapshot(true).unwrap();
    let engine = tiles.engine();
    let pdf = render(
        &snapshot,
        &request(ExportFormat::BalloonedPdf),
        Some(engine),
    )
    .unwrap();
    let again = render(
        &snapshot,
        &request(ExportFormat::BalloonedPdf),
        Some(engine),
    )
    .unwrap();
    assert_eq!(pdf, again, "same snapshot, same bytes");

    let doc = engine.open(pdf).unwrap();
    let runs = doc.text_runs(0).unwrap();
    for (x, number) in [(150.0, "1"), (270.0, "2"), (390.0, "3")] {
        assert!(
            runs.iter().any(|r| r.text == number
                && (r.bbox.x + r.bbox.width / 2.0 - x).abs() < 3.0
                && (r.bbox.y + r.bbox.height / 2.0 - 120.0).abs() < 3.0),
            "balloon {number} not found at its position"
        );
    }

    let annotated = ExportRequest {
        pdf_balloons: PdfBalloons::Annotations,
        ..request(ExportFormat::BalloonedPdf)
    };
    let first = render(&snapshot, &annotated, Some(engine)).unwrap();
    assert_eq!(first, render(&snapshot, &annotated, Some(engine)).unwrap());
    let text = String::from_utf8_lossy(&first);
    assert!(text.contains("/Annot"), "annotations written");
    // The annotation date is the project's last change, not the export time.
    let stamp = snapshot.modified.as_str();
    let pdf_date = format!(
        "D:{}{}{}{}{}{}",
        &stamp[0..4],
        &stamp[5..7],
        &stamp[8..10],
        &stamp[11..13],
        &stamp[14..16],
        &stamp[17..19]
    );
    assert!(text.contains(&pdf_date), "missing {pdf_date}");
}

#[test]
fn characteristic_lists_contain_the_rows() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let snapshot = session.export_snapshot(false).unwrap();
    assert!(snapshot.drawing.is_none());
    let csv = render(&snapshot, &request(ExportFormat::Csv), None).unwrap();
    let csv = String::from_utf8(csv).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 4, "{csv}");
    assert!(lines[0].starts_with("No,"), "{csv}");
    assert!(
        lines[1].starts_with("1,") && lines[1].contains("Ø8 f7"),
        "{csv}"
    );
    let german = ExportRequest {
        language: ReportLanguage::De,
        ..request(ExportFormat::Csv)
    };
    let csv_de = String::from_utf8(render(&snapshot, &german, None).unwrap()).unwrap();
    assert!(csv_de.starts_with("Nr"), "{csv_de}");
    let xlsx = render(&snapshot, &request(ExportFormat::Xlsx), None).unwrap();
    assert_eq!(&xlsx[..2], b"PK");
    assert_eq!(
        xlsx,
        render(&snapshot, &request(ExportFormat::Xlsx), None).unwrap()
    );
    // The PDF needs the engine and the drawing.
    assert!(render(&snapshot, &request(ExportFormat::BalloonedPdf), None).is_err());
}
