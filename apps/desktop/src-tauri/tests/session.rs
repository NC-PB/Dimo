//! The project session behind the IPC commands (T1.5): new project from a drawing, commands,
//! undo and redo as patches, save, reopen, instance lock and recovery of unsaved projects.
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr, clippy::assert_is_empty)]

use std::path::{Path, PathBuf};

use dimo_core::{Change, Command, FieldValue, Point};
use dimo_desktop::ipc::CommandError;
use dimo_desktop::session::{AppSession, Autosave, ProjectLoaded};
use dimo_pdf::tiles::{TileConfig, TileService};
use dimo_pdf::{PdfEngine, PdfError};

/// SHA-256 of `corpus/drawings/test_drawing_1.pdf` from `corpus/PROVENANCE.md`.
const TEST_DRAWING_1_SHA256: &str =
    "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";

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

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|listing| {
            listing
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Adds a characteristic with a balloon on the first sheet.
fn add(session: &AppSession, text: &str) -> Command {
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    Command::AddCharacteristic {
        sheet,
        position: Point { x: 200.0, y: 150.0 },
        anchor: Point { x: 237.5, y: 166.1 },
        region: None,
        values: vec![FieldValue::RequirementText(text.into())],
        insert_after: None,
    }
}

fn created(session: &mut AppSession, tiles: &TileService) -> ProjectLoaded {
    session
        .create_from_drawing(tiles, &drawing(), false)
        .unwrap()
}

#[test]
fn new_project_command_undo_redo_save_and_reopen() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let autosave = dir.path().join("autosave");
    let mut session = AppSession::new(Some(autosave.clone()));

    let loaded = created(&mut session, &tiles);
    assert_eq!(loaded.session, 1);
    let snapshot = loaded.snapshot.unwrap();
    assert_eq!(snapshot.revision, 0);
    assert_eq!(snapshot.drawing.doc, TEST_DRAWING_1_SHA256);
    assert_eq!(snapshot.drawing.name, "test_drawing_1.pdf");
    assert_eq!(snapshot.drawing.sheets.len(), 1);
    assert_eq!(
        snapshot.project.revisions[0].sha256.as_str(),
        TEST_DRAWING_1_SHA256
    );
    assert_eq!(snapshot.status.file_name, None);
    assert!(!snapshot.status.modified);
    assert_eq!(snapshot.status.autosave, Autosave::Clean);
    // The tile service knows the drawing under its content hash.
    assert!(
        tiles
            .document(&dimo_pdf::ContentHash::from_hex(TEST_DRAWING_1_SHA256).unwrap())
            .is_some()
    );

    // Command: one patch with the new characteristic and its balloon.
    let patched = session.execute(add(&session, "Ø8 f7")).unwrap();
    assert_eq!((patched.session, patched.revision), (1, 1));
    let inserted = match &patched.patch.changes[..] {
        [
            Change::CharacteristicInserted { characteristic, .. },
            Change::BalloonInserted { balloon, .. },
        ] => {
            assert_eq!(balloon.characteristic, characteristic.id);
            characteristic.clone()
        }
        other => panic!("unexpected changes {other:?}"),
    };
    assert_eq!(inserted.number.to_string(), "1");
    assert!(patched.status.modified && patched.status.can_undo && !patched.status.can_redo);
    assert_eq!(patched.status.autosave, Autosave::Journaled);
    let journal = session.journal_path().unwrap();
    assert!(journal.starts_with(&autosave), "{}", journal.display());

    // Undo and redo reach the frontend as patches with increasing revisions.
    let undone = session.undo().unwrap();
    assert_eq!(undone.revision, 2);
    assert!(matches!(
        &undone.patch.changes[..],
        [Change::BalloonRemoved { .. }, Change::CharacteristicRemoved { characteristic, .. }]
            if characteristic.id == inserted.id
    ));
    assert!(undone.status.can_redo);
    assert!(session.project().unwrap().characteristics.is_empty());
    let redone = session.redo().unwrap();
    assert_eq!(redone.revision, 3);
    assert_eq!(redone.patch, patched.patch);
    assert!(!redone.status.can_redo);

    // Save as: the project leaves the autosave folder.
    let path = dir.path().join("part.dimo");
    let saved = session.save_as(&path).unwrap();
    assert_eq!(saved.status.file_name.as_deref(), Some("part.dimo"));
    assert!(!saved.status.modified);
    assert_eq!(saved.status.autosave, Autosave::Clean);
    assert_eq!(files_in(&autosave), Vec::<String>::new());
    assert_eq!(
        files_in(dir.path()),
        vec!["autosave", "part.dimo", "part.dimo.lock"]
    );
    assert_eq!(session.suggested_file_name(), "part.dimo");

    // Reopen.
    let closed = session.close(Some(&tiles), false).unwrap();
    assert!(closed.snapshot.is_none());
    assert_eq!(files_in(dir.path()), vec!["autosave", "part.dimo"]);
    let reopened = session.open_file(&tiles, &path, false).unwrap();
    assert_eq!(reopened.session, 3);
    assert_eq!(reopened.notice, None);
    let project = reopened.snapshot.unwrap().project;
    assert_eq!(project.characteristics, vec![*inserted]);
    assert_eq!(project.balloons.len(), 1);
}

#[test]
fn second_instance_cannot_open_the_same_project() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.dimo");
    let mut first = AppSession::new(None);
    created(&mut first, &tiles);
    first.save_as(&path).unwrap();

    let mut second = AppSession::new(None);
    let refused = second.open_file(&tiles, &path, false);
    assert!(
        matches!(refused, Err(CommandError::InUse { ref user }) if user.as_deref() == Some(first.user_name().as_str()) || cfg!(windows)),
        "{refused:?}"
    );
    // Opening it again in the same session reloads it instead.
    assert!(first.open_file(&tiles, &path, false).is_ok());
    first.close(Some(&tiles), false).unwrap();
    assert!(second.open_file(&tiles, &path, false).is_ok());
}

#[test]
fn unsaved_changes_need_discard() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.dimo");
    let mut session = AppSession::new(None);
    created(&mut session, &tiles);
    session.save_as(&path).unwrap();
    session.execute(add(&session, "10")).unwrap();
    assert!(session.has_unsaved_changes());

    assert!(matches!(
        session.create_from_drawing(&tiles, &drawing(), false),
        Err(CommandError::UnsavedChanges)
    ));
    assert!(matches!(
        session.close(Some(&tiles), false),
        Err(CommandError::UnsavedChanges)
    ));
    let journal = session.journal_path().unwrap();
    assert!(journal.exists());
    // Close without saving: the journal goes, the file keeps the saved state.
    session.close(Some(&tiles), true).unwrap();
    assert!(!journal.exists());
    let reopened = session.open_file(&tiles, &path, false).unwrap();
    assert!(
        reopened
            .snapshot
            .unwrap()
            .project
            .characteristics
            .is_empty()
    );
}

#[test]
fn saved_project_is_recovered_from_its_journal() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.dimo");
    {
        let mut session = AppSession::new(None);
        created(&mut session, &tiles);
        session.save_as(&path).unwrap();
        session.execute(add(&session, "10")).unwrap();
        // Crash: dropped without shutdown or save.
    }
    let mut session = AppSession::new(None);
    let reopened = session.open_file(&tiles, &path, false).unwrap();
    let notice = reopened.notice.unwrap();
    assert_eq!(notice.recovered_changes, 1);
    assert!(!notice.restored_unsaved);
    let snapshot = reopened.snapshot.unwrap();
    assert_eq!(snapshot.project.characteristics.len(), 1);
    assert!(snapshot.status.modified);
}

#[test]
fn unsaved_project_is_restored_at_the_next_start() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let autosave = dir.path().join("autosave");
    let expected = {
        let mut session = AppSession::new(Some(autosave.clone()));
        created(&mut session, &tiles);
        session.execute(add(&session, "Ø8 f7")).unwrap();
        session.execute(add(&session, "R15")).unwrap();
        session.project().unwrap().clone()
        // Crash.
    };

    let mut session = AppSession::new(Some(autosave.clone()));
    assert!(session.recover_unsaved(&tiles).unwrap());
    let state = session.state();
    let notice = state.notice.unwrap();
    assert!(notice.restored_unsaved);
    assert_eq!(notice.recovered_changes, 2);
    let snapshot = state.snapshot.unwrap();
    assert_eq!(snapshot.project, expected);
    assert_eq!(snapshot.status.file_name, None);
    assert!(snapshot.status.modified);
    // The notice is handed out once.
    assert_eq!(session.state().notice, None);

    // Closing without saving deletes the autosave files, so nothing is restored again.
    session.close(Some(&tiles), true).unwrap();
    assert_eq!(files_in(&autosave), Vec::<String>::new());
    let mut later = AppSession::new(Some(autosave));
    assert!(!later.recover_unsaved(&tiles).unwrap());
}

#[test]
fn clean_shutdown_leaves_no_autosave_files() {
    let Some(tiles) = service() else { return };
    let dir = tempfile::tempdir().unwrap();
    let autosave = dir.path().join("autosave");
    let mut session = AppSession::new(Some(autosave.clone()));
    created(&mut session, &tiles);
    session.shutdown(Some(&tiles));
    assert_eq!(files_in(&autosave), Vec::<String>::new());
}

#[test]
fn refused_commands_change_nothing() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    created(&mut session, &tiles);
    let refused = session.execute(Command::DeleteCharacteristics {
        ids: vec![dimo_core::CharId::from_uuid(uuid_one())],
    });
    assert!(
        matches!(refused, Err(CommandError::Rejected { .. })),
        "{refused:?}"
    );
    assert!(matches!(session.undo(), Err(CommandError::Rejected { .. })));
    assert_eq!(session.state().snapshot.unwrap().revision, 0);
}

fn uuid_one() -> uuid::Uuid {
    uuid::Uuid::from_u128(1)
}
