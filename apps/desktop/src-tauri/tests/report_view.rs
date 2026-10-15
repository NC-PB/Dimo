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

use dimo_core::Language;
use dimo_core::Sha256Hex;
use dimo_core::{
    Command, FieldValue, Point, ProjectInfoField, ReportInfoField, ReportKind, ResultEntry,
};
use dimo_desktop::export::{ExportRequest, render, run_steps, take_snapshot};
use dimo_desktop::report::{
    ReportField, ReportView, checks, preview_bytes, print_snapshot, save_copy_snapshot,
    write_print_file,
};
use dimo_desktop::session::AppSession;
use dimo_pdf::tiles::{TileConfig, TileKey, TileService};
use dimo_pdf::{PdfEngine, PdfError};
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

fn view(kind: ReportKind, language: Language) -> ReportView {
    ReportView {
        kind,
        language,
        blank_columns: 4,
    }
}

const VIEWS: [(ReportKind, Language); 4] = [
    (ReportKind::CheckSheet, Language::En),
    (ReportKind::CheckSheet, Language::De),
    (ReportKind::InspectionReport, Language::En),
    (ReportKind::InspectionReport, Language::De),
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
        let preview = preview_bytes(&save_copy_snapshot(&session, view).unwrap(), view).unwrap();
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
        let again = preview_bytes(&save_copy_snapshot(&session, view).unwrap(), view).unwrap();
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
    let view = view(ReportKind::InspectionReport, Language::En);
    let bytes = preview_bytes(&save_copy_snapshot(&session, view).unwrap(), view).unwrap();
    let hash = Sha256Hex::of(&bytes);
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
    session
        .execute(Command::UpdateReportInfo {
            values: vec![ReportInfoField::ReportNumber("IR-1".into())],
        })
        .unwrap();
    session
        .execute(Command::UpdateProjectInfo {
            values: vec![ProjectInfoField::PartNumber("BR-1".into())],
        })
        .unwrap();
    assert_eq!(
        checks(&session, ReportKind::InspectionReport)
            .unwrap()
            .missing_fields,
        Vec::<ReportField>::new()
    );
    // Without a company name in the project or the settings the field stays open.
    session.set_company_name("");
    session
        .execute(Command::UpdateReportInfo {
            values: vec![ReportInfoField::CompanyName(String::new())],
        })
        .unwrap();
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
    let view = view(ReportKind::InspectionReport, Language::De);
    let preview = preview_bytes(&save_copy_snapshot(&session, view).unwrap(), view).unwrap();
    let (name, start) = print_snapshot(&mut session, view).unwrap();
    assert!(start.date.is_some(), "the date was stored");
    assert!(start.issued.is_none() && start.lock.is_none());
    assert!(name.ends_with("_inspection-report.pdf"), "{name}");
    let printed = render(&start.snapshot, &view.save_copy(), None).unwrap();
    assert_eq!(printed, preview);
    let dir = tempfile::tempdir().unwrap();
    let path = write_print_file(dir.path(), &name, &printed).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), preview);
    let (_, again) = print_snapshot(&mut session, view).unwrap();
    assert!(again.date.is_none(), "stored once");
}

/// MF F2: one report date rule (`dimo_report::issue::report_date`). Preview, Save copy, Print and
/// the issued export print the same date: today while none is stored, else the stored one.
#[test]
fn preview_save_copy_print_and_issued_export_print_one_report_date() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    let view = view(ReportKind::InspectionReport, Language::En);
    let date = |project: &dimo_core::Project| project.report_info.report_date.clone();
    for stored in ["", "2026-01-05"] {
        session
            .execute(Command::UpdateReportInfo {
                values: vec![ReportInfoField::ReportDate(stored.into())],
            })
            .unwrap();
        let info = session.project().unwrap().report_info.clone();
        let expected = dimo_report::issue::report_date(&info, "2026-10-13").to_owned();
        let want = if stored.is_empty() {
            "2026-10-13"
        } else {
            stored
        };
        assert_eq!(expected, want);
        let before = session.project().unwrap().clone();

        let preview = save_copy_snapshot(&session, view).unwrap();
        assert_eq!(date(&preview.project), expected, "preview");
        assert_eq!(
            session.project().unwrap(),
            &before,
            "the preview stores nothing"
        );

        let issued = take_snapshot(
            &mut session,
            ExportRequest {
                issued: true,
                ..view.save_copy()
            },
        )
        .unwrap();
        assert_eq!(date(&issued.snapshot.project), expected, "issued export");
        assert_eq!(issued.issued.unwrap().report_date, expected);
        assert_eq!(
            session.project().unwrap(),
            &before,
            "recorded after the write"
        );

        let copy = take_snapshot(&mut session, view.save_copy()).unwrap();
        assert_eq!(date(&copy.snapshot.project), expected, "save copy");
        assert_eq!(date(session.project().unwrap()), expected, "stored");
        assert_eq!(copy.date.is_some(), stored.is_empty());
        if copy.date.is_some() {
            session.undo().unwrap();
        }

        let (_, print) = print_snapshot(&mut session, view).unwrap();
        assert_eq!(date(&print.snapshot.project), expected, "print");
        assert_eq!(print.date.is_some(), stored.is_empty());
        if print.date.is_some() {
            session.undo().unwrap();
        }
    }
}
