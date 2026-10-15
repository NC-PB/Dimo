//! MF F2: the command line tool exports a check sheet and an inspection report byte identical to
//! the app. The app path is the export job (`take_snapshot`, `run_steps`); the CLI path is
//! `dimo_cli::export::export_project` on the saved project file. Both render through
//! `dimo_export::render`.
//!
//! Needs PDFium for the project (drawing import). Skips without it, fails instead with
//! `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use dimo_cli::export::{CliExportError, export_project};
use dimo_core::{Command, FieldValue, Language, Point, ResultEntry};
use dimo_desktop::export::{ExportFormat, ExportRequest, run_steps, take_snapshot};
use dimo_desktop::session::AppSession;
use dimo_desktop::settings::PdfBalloons;
use dimo_pdf::tiles::{TileConfig, TileService};
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

const TODAY: &str = "2026-10-13";

/// Two characteristics with limits, two samples, three of four results, no report date.
fn session(tiles: &TileService) -> AppSession {
    let mut session = AppSession::new(None);
    session.set_user_name("Anna");
    session.set_company_name("Muster GmbH");
    session.set_fixed_local_date(Some(TODAY));
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

fn request(format: ExportFormat) -> ExportRequest {
    ExportRequest {
        format,
        language: Language::De,
        pdf_balloons: PdfBalloons::PageContent,
        blank_columns: 4,
        issued: false,
        open_after: false,
    }
}

/// The file the app's export job writes for `request` (storing the report date when empty).
fn app_export(session: &mut AppSession, request: ExportRequest, dir: &Path) -> Vec<u8> {
    let start = take_snapshot(session, request).unwrap();
    let path = dir.join(request.format.file_name("app"));
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

#[test]
fn cli_exports_reports_byte_identical_to_the_app() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("part.dimo");
    session.save_as(&project).unwrap();
    assert_eq!(session.project().unwrap().report_info.report_date, "");

    // A project without a report date: the CLI needs the date the app stores.
    let options = request(ExportFormat::CheckSheetPdf).render_options();
    assert!(matches!(
        export_project(&project, options, None, None),
        Err(CliExportError::NoReportDate)
    ));

    for format in [
        ExportFormat::CheckSheetPdf,
        ExportFormat::InspectionReportPdf,
        ExportFormat::CheckSheetXlsx,
        ExportFormat::InspectionReportXlsx,
        ExportFormat::Csv,
    ] {
        let request = request(format);
        let cli = export_project(&project, request.render_options(), Some(TODAY), None).unwrap();
        let app = app_export(&mut session, request, dir.path());
        assert_eq!(cli, app, "{format:?}");
    }

    // The app stored the date; saved, the CLI prints it without being told.
    assert_eq!(session.project().unwrap().report_info.report_date, TODAY);
    session.save().unwrap();
    for format in [
        ExportFormat::CheckSheetPdf,
        ExportFormat::InspectionReportPdf,
    ] {
        let request = request(format);
        let cli = export_project(&project, request.render_options(), None, None).unwrap();
        let other_day =
            export_project(&project, request.render_options(), Some("2026-11-02"), None).unwrap();
        let app = app_export(&mut session, request, dir.path());
        assert_eq!(cli, app, "{format:?}");
        assert_eq!(other_day, app, "a stored date wins");
    }
}
