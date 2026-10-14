//! Initial report info of new projects (T3.4, M3 decision 15): the company name of the app
//! settings and the audit user as inspector, as part of the initial state. Opened projects are
//! left alone.
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::{Command, ReportInfoField};
use dimo_desktop::session::AppSession;
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

#[test]
fn new_project_starts_with_company_and_audit_user() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session.set_user_name("Anna");
    session.set_company_name("  Muster GmbH ");
    let loaded = session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let snapshot = loaded.snapshot.unwrap();
    assert_eq!(snapshot.project.report_info.company_name, "Muster GmbH");
    assert_eq!(snapshot.project.report_info.inspected_by, "Anna");
    assert_eq!(snapshot.project.report_info.report_number, "");
    assert!(!snapshot.status.modified);
    assert!(!snapshot.status.can_undo);
}

#[test]
fn opening_an_existing_project_with_empty_values_changes_nothing() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("part.dimo");

    // A project whose company and inspector were cleared by the user.
    let mut first = AppSession::new(None);
    first.set_user_name("Anna");
    first.set_company_name("Muster GmbH");
    first
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    first
        .execute(Command::UpdateReportInfo {
            values: vec![
                ReportInfoField::CompanyName(String::new()),
                ReportInfoField::InspectedBy(String::new()),
            ],
        })
        .unwrap();
    first.save_as(&file).unwrap();

    drop(first);

    // Another session with settings opens it: nothing is filled in, nothing is modified.
    let mut second = AppSession::new(None);
    second.set_user_name("Ben");
    second.set_company_name("Other AG");
    let loaded = second.open_file(&tiles, &file, false).unwrap();
    let snapshot = loaded.snapshot.unwrap();
    assert_eq!(snapshot.project.report_info.company_name, "");
    assert_eq!(snapshot.project.report_info.inspected_by, "");
    assert!(!snapshot.status.modified);
    assert!(!snapshot.status.can_undo);
}
