//! The Report tab (T3.19): the preview is the file "Save copy" writes (FR-EXP-11, rule 11), its
//! pages render through the tile service, the checks before issuing count what is open, and
//! Print stores the report date as "Save copy" does.
//!
//! Needs PDFium for the project (drawing import) and the preview pages. Skips without it, fails
//! instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use dimo_core::{Command, FieldValue, Point, ReportKind, ResultEntry};
use dimo_desktop::export::{render, run_steps, take_snapshot};
use dimo_desktop::report::{
    ReportField, ReportView, checks, preview_bytes, print_snapshot, save_copy_snapshot,
};
use dimo_desktop::session::AppSession;
use dimo_desktop::settings::ReportLanguage;
use dimo_pdf::tiles::{TileConfig, TileKey, TileService};
use dimo_pdf::{ContentHash, PdfEngine, PdfError};
use rust_decimal::Decimal;

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

fn view(kind: ReportKind, language: ReportLanguage) -> ReportView {
    ReportView {
        kind,
        language,
        blank_columns: 4,
    }
}

const VIEWS: [(ReportKind, ReportLanguage); 4] = [
    (ReportKind::CheckSheet, ReportLanguage::En),
    (ReportKind::CheckSheet, ReportLanguage::De),
    (ReportKind::InspectionReport, ReportLanguage::En),
    (ReportKind::InspectionReport, ReportLanguage::De),
];

/// Two characteristics with limits, two samples, three of four results, no report date.
fn session(tiles: &TileService) -> AppSession {
    let mut session = AppSession::new(None);
    session.set_user_name("Anna");
    session.set_company_name("Muster GmbH");
    session.set_fixed_local_date(Some("2026-10-13"));
    session
        .create_from_drawing(tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    for (x, text, nominal) in [(150.0, "Ø8 ±0.1", 80), (270.0, "30 ±0.1", 300)] {
        session
            .execute(Command::AddCharacteristic {
                sheet,
                position: Point { x, y: 120.0 },
                anchor: Point { x, y: 150.0 },
                region: None,
                values: vec![
                    FieldValue::RequirementText(text.into()),
                    FieldValue::Nominal(Some(Decimal::new(nominal, 1)).into()),
                    FieldValue::UpperDev(Some(Decimal::new(1, 1)).into()),
                    FieldValue::LowerDev(Some(Decimal::new(-1, 1)).into()),
                ],
                insert_after: None,
            })
            .unwrap();
    }
    session.execute(Command::AddSamples { count: 2 }).unwrap();
    let project = session.project().unwrap();
    let (a, b) = (project.samples[0].id, project.samples[1].id);
    let (c1, c2) = (project.characteristics[0].id, project.characteristics[1].id);
    let entry = |sample, characteristic, text: &str| ResultEntry {
        sample,
        characteristic,
        text: text.to_owned(),
        comment: None,
    };
    session
        .execute(Command::SetResults {
            results: vec![
                entry(a, c1, "8.05"),
                entry(a, c2, "30.05"),
                entry(b, c1, "7,95"),
            ],
        })
        .unwrap();
    session
}

/// The bytes "Save copy" writes for `view`: snapshot (storing the date when empty), then the
/// job steps into a file.
fn saved_copy(session: &mut AppSession, view: ReportView) -> Vec<u8> {
    let request = view.save_copy();
    let start = take_snapshot(session, request).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy.pdf");
    run_steps(
        &start.snapshot,
        &request,
        None,
        &path,
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .unwrap();
    std::fs::read(path).unwrap()
}

/// Rule 11: the preview is byte for byte the PDF that "Save copy" writes, before the first
/// export (the report date is still empty, the preview shows today's date as the export will
/// store it) and after it.
#[test]
fn preview_is_the_file_save_copy_writes() {
    let Some(tiles) = service() else { return };
    for (kind, language) in VIEWS {
        let mut session = session(&tiles);
        let view = view(kind, language);
        let before = session.project().unwrap().clone();
        let preview = preview_bytes(&save_copy_snapshot(&session).unwrap(), view).unwrap();
        assert_eq!(
            session.project().unwrap(),
            &before,
            "the preview changes nothing"
        );
        assert!(preview.starts_with(b"%PDF"));
        let saved = saved_copy(&mut session, view);
        assert_eq!(
            preview, saved,
            "{kind:?} {language:?} before the first export"
        );
        assert_eq!(
            session.project().unwrap().report_info.report_date,
            "2026-10-13"
        );
        // After the export stored the date, the preview still equals a new copy.
        let again = preview_bytes(&save_copy_snapshot(&session).unwrap(), view).unwrap();
        assert_eq!(
            again,
            saved_copy(&mut session, view),
            "{kind:?} {language:?}"
        );
        assert_eq!(again, preview);
    }
}

/// The preview pages render through the tile service, like a drawing.
#[test]
fn preview_pages_render_as_tiles() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let bytes = preview_bytes(
        &save_copy_snapshot(&session).unwrap(),
        view(ReportKind::InspectionReport, ReportLanguage::En),
    )
    .unwrap();
    let hash = ContentHash::of(&bytes);
    let doc = tiles.open_document(bytes).unwrap();
    assert!(doc.sheet_count() >= 1);
    let page = doc.sheet_size(0).unwrap();
    assert!(page.width > page.height, "A4 landscape (M3 decision 12)");
    let tile = tiles
        .get(TileKey {
            doc: hash,
            sheet: 0,
            zoom: 0,
            x: 0,
            y: 0,
        })
        .unwrap();
    assert!(tile.as_bytes().starts_with(b"\x89PNG"));
    assert!(tiles.close_document(&hash));
}

/// The checks before issuing: counts of missing values, draft and changed rows, and the header
/// fields that issuing cannot fill.
#[test]
fn checks_count_what_is_open_before_issuing() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    let report = checks(&session, ReportKind::InspectionReport).unwrap();
    assert!(report.warnings.open_results);
    assert_eq!((report.samples, report.rows), (2, 2));
    assert_eq!((report.missing_values, report.not_evaluated), (1, 0));
    assert_eq!((report.draft_rows, report.changed_rows), (0, 0));
    // Company and inspector are filled when issuing, so only these two are asked for.
    assert_eq!(
        report.missing_fields,
        [ReportField::PartNumber, ReportField::ReportNumber]
    );
    let sheet = checks(&session, ReportKind::CheckSheet).unwrap();
    assert_eq!((sheet.missing_values, sheet.not_evaluated), (0, 0));
    assert!(!sheet.warnings.any());

    // Filled in: nothing left to ask for.
    let mut info = session.project().unwrap().report_info.clone();
    info.report_number = "IR-1".into();
    session
        .execute(Command::UpdateReportInfo { info: info.clone() })
        .unwrap();
    let mut part = session.project().unwrap().info.clone();
    part.part_number = "BR-1".into();
    session
        .execute(Command::UpdateProjectInfo { info: part })
        .unwrap();
    assert_eq!(
        checks(&session, ReportKind::InspectionReport)
            .unwrap()
            .missing_fields,
        Vec::<ReportField>::new()
    );
    // Without a company name in the project or the settings the field stays open.
    session.set_company_name("");
    info.company_name = String::new();
    session.execute(Command::UpdateReportInfo { info }).unwrap();
    assert_eq!(
        checks(&session, ReportKind::InspectionReport)
            .unwrap()
            .missing_fields,
        [ReportField::CompanyName]
    );
}

/// Print takes the "Save copy" snapshot: the report date is stored once, as one undo step, and
/// the printed file is the preview.
#[test]
fn print_prints_the_preview_and_stores_the_date_like_save_copy() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    let view = view(ReportKind::InspectionReport, ReportLanguage::De);
    let preview = preview_bytes(&save_copy_snapshot(&session).unwrap(), view).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (path, start) = print_snapshot(&mut session, view, dir.path()).unwrap();
    assert!(start.date.is_some(), "the date was stored");
    assert!(start.issued.is_none() && start.lock.is_none());
    assert_eq!(path.parent(), Some(dir.path()));
    assert!(
        path.file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with("_inspection-report.pdf")
    );
    assert_eq!(
        render(&start.snapshot, &view.save_copy(), None).unwrap(),
        preview
    );
    let (_, again) = print_snapshot(&mut session, view, dir.path()).unwrap();
    assert!(again.date.is_none(), "stored once");
}
