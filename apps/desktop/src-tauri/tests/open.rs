//! Opening a document by path, the part of `open_document_dialog` after the dialog (T0.8).
//!
//! Skips with a message when PDFium is missing, unless `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_desktop::ipc::{CommandError, open_path_blocking};
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

fn corpus(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../corpus/drawings")
        .join(name)
}

#[test]
fn opens_a_drawing_by_path() {
    let Some(service) = service() else { return };
    let info = open_path_blocking(&service, &corpus("test_drawing_1.pdf")).unwrap();
    assert_eq!(info.name, "test_drawing_1.pdf");
    assert_eq!(info.doc.len(), 64);
    assert_eq!(info.sheets.len(), 1);
    let sheet = info.sheets[0];
    assert!(sheet.width > 0.0 && sheet.height > 0.0);
    // Opening the same file again yields the same document.
    let again = open_path_blocking(&service, &corpus("test_drawing_1.pdf")).unwrap();
    assert_eq!(again, info);
}

#[test]
fn reports_missing_and_invalid_files() {
    let Some(service) = service() else { return };
    let missing = open_path_blocking(&service, &corpus("does_not_exist.pdf"));
    assert!(
        matches!(missing, Err(CommandError::Io { .. })),
        "{missing:?}"
    );

    let dir = tempfile::tempdir().unwrap();
    let not_pdf = dir.path().join("note.pdf");
    std::fs::write(&not_pdf, b"not a pdf").unwrap();
    let invalid = open_path_blocking(&service, &not_pdf);
    assert!(
        matches!(invalid, Err(CommandError::InvalidDocument { .. })),
        "{invalid:?}"
    );
}
