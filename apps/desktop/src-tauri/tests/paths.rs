//! NFR-PORT-02 (R.12): a project is created, saved, changed, saved again, reopened and exported
//! in folders with spaces and umlauts, in a folder whose paths are longer than 260 characters
//! (the Windows limit without the verbatim prefix) and, on Windows, on a network share
//! (`\\localhost\C$\...`). The lock file and the journal stay next to the project and every
//! export lands where it was asked to.
//!
//! Needs PDFium for the project (drawing import). Skips without it, fails instead with
//! `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use dimo_core::{Command, FieldValue, Language, Point};
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

/// Adds a characteristic with limits on the first sheet.
fn add(session: &mut AppSession, x: f64) {
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    session
        .execute(Command::AddCharacteristic {
            sheet,
            position: Point { x, y: 120.0 },
            anchor: Point { x, y: 150.0 },
            region: None,
            values: vec![
                FieldValue::RequirementText("Ø8 ±0.1".into()),
                FieldValue::Nominal(Some(Decimal::new(80, 1)).into()),
                FieldValue::UpperDev(Some(Decimal::new(1, 1)).into()),
                FieldValue::LowerDev(Some(Decimal::new(-1, 1)).into()),
            ],
            insert_after: None,
        })
        .unwrap();
}

/// The whole life of a project in `folder`, which is created first.
fn round_trip(tiles: &TileService, folder: &Path) {
    std::fs::create_dir_all(folder).unwrap();
    // The drawing itself also comes from the folder.
    let source = folder.join("Prüfzeichnung Größe 1.pdf");
    std::fs::copy(drawing(), &source).unwrap();
    let path = folder.join("Prüfteil 1.dimo");

    {
        let mut session = AppSession::new(None);
        session.set_fixed_local_date(Some("2026-10-17"));
        session.create_from_drawing(tiles, &source, false).unwrap();
        add(&mut session, 150.0);
        let saved = session.save_as(&path).unwrap();
        assert_eq!(saved.status.file_name.as_deref(), Some("Prüfteil 1.dimo"));
        assert!(path.is_file(), "{}", path.display());
        assert!(folder.join("Prüfteil 1.dimo.lock").is_file());

        // A change after saving goes to the journal next to the project.
        add(&mut session, 270.0);
        let journal = session.journal_path().unwrap();
        assert_eq!(journal, folder.join("Prüfteil 1.dimo.journal"));
        assert!(journal.is_file());
        session.save().unwrap();
        assert!(!journal.exists());

        for format in ExportFormat::ALL {
            let request = ExportRequest {
                format,
                language: Language::De,
                pdf_balloons: PdfBalloons::PageContent,
                blank_columns: 4,
                issued: false,
                open_after: false,
            };
            let start = take_snapshot(&mut session, request).unwrap();
            let target = folder.join(format.file_name("Prüfteil 1"));
            run_steps(
                &start.snapshot,
                &request,
                tiles.engine(),
                &target,
                &AtomicBool::new(false),
                &mut |_, _| {},
            )
            .unwrap();
            assert!(
                std::fs::metadata(&target).unwrap().len() > 0,
                "{}",
                target.display()
            );
        }
        session.save().unwrap();
        // Dropping the session releases the lock.
    }
    assert!(!folder.join("Prüfteil 1.dimo.lock").exists());

    let mut session = AppSession::new(None);
    let reopened = session.open_file(tiles, &path, false).unwrap();
    let snapshot = reopened.snapshot.unwrap();
    assert_eq!(snapshot.project.characteristics.len(), 2);
    assert_eq!(
        snapshot.status.file_name.as_deref(),
        Some("Prüfteil 1.dimo")
    );
    assert!(!snapshot.status.modified);
    assert!(reopened.notice.is_none());
    session.close(Some(tiles), false).unwrap();

    // Only the expected files, no temporary leftovers.
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut expected: Vec<String> = ExportFormat::ALL
        .iter()
        .map(|f| f.file_name("Prüfteil 1"))
        .chain(["Prüfteil 1.dimo".into(), "Prüfzeichnung Größe 1.pdf".into()])
        .collect();
    expected.sort();
    expected.dedup();
    assert_eq!(names, expected);
}

#[test]
fn projects_live_in_folders_with_spaces_and_umlauts() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    round_trip(
        &tiles,
        &dir.path()
            .join("Zeichnungen Müller")
            .join("Kunde Ärger & Söhne"),
    );
}

#[test]
fn projects_live_in_folders_with_paths_over_260_characters() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let mut folder = dir.path().join("Zeichnungen Müller");
    // Every export path in the folder is longer than 260 characters, counted in UTF-16 units
    // as Windows does.
    while folder.to_string_lossy().encode_utf16().count() < 270 {
        folder.push("Baugruppe Größe 1 Prüfteile");
    }
    round_trip(&tiles, &folder);
}

/// A network share: the temporary folder through the administrative share of its drive,
/// `\\localhost\C$\...`. Skipped where the share is not available (it is switched off on
/// some computers).
#[cfg(windows)]
#[test]
fn projects_live_on_network_shares() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().to_string_lossy().into_owned();
    let mut chars = local.chars();
    let (Some(drive), Some(':')) = (chars.next(), chars.next()) else {
        eprintln!("SKIPPED (temporary folder {local} has no drive letter)");
        return;
    };
    let share = PathBuf::from(format!(r"\\localhost\{drive}${}", chars.as_str()));
    if std::fs::metadata(&share).is_err() {
        eprintln!(
            "SKIPPED (administrative share {} not available)",
            share.display()
        );
        return;
    }
    round_trip(&tiles, &share.join("Zeichnungen Müller").join("Prüfteile"));
}
