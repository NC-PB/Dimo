//! The log file (F7, D-06): a journal write that fails is written to the log.
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use dimo_core::{Command, FieldValue, Point};
use dimo_desktop::logging::{FileLog, LOG_FILE, LogLevel, MAX_LOG_BYTES};
use dimo_desktop::session::{AppSession, Autosave};
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
fn a_failed_journal_write_is_logged() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let autosave = dir.path().join("autosave");
    let mut session = AppSession::new(Some(autosave.clone()));
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();

    // Block the journal: a folder (not empty) where the journal file of the autosave base goes.
    let base = std::fs::read_dir(&autosave)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "dimo"))
        .unwrap();
    let mut journal = base.into_os_string();
    journal.push(".journal");
    let journal = PathBuf::from(journal);
    std::fs::create_dir(&journal).unwrap();
    std::fs::write(journal.join("blocker"), b"x").unwrap();

    let log_path = dir.path().join("logs").join(LOG_FILE);
    let level = LogLevel {
        mirror: false,
        ..LogLevel::for_build()
    };
    let log = FileLog::open(&log_path, MAX_LOG_BYTES, level).unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let patched = tracing::subscriber::with_default(log, || {
        session
            .execute(Command::AddCharacteristic {
                sheet,
                position: Point { x: 200.0, y: 150.0 },
                anchor: Point { x: 237.5, y: 166.1 },
                region: None,
                values: vec![FieldValue::RequirementText("Ø8 f7".into())],
                insert_after: None,
            })
            .unwrap()
    });
    assert!(
        matches!(patched.status.autosave, Autosave::Failed { .. }),
        "{:?}",
        patched.status.autosave
    );
    let text = std::fs::read_to_string(&log_path).unwrap();
    let line = text
        .lines()
        .find(|l| l.contains("autosave journal write failed"))
        .unwrap_or_else(|| panic!("not logged: {text}"));
    assert!(line.contains(" WARN dimo_desktop::session: "), "{line}");
    assert!(line.contains(".journal"), "names the file: {line}");
}
