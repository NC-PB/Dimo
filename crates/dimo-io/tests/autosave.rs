//! Autosave of never saved projects and the instance lock (T1.5, NFR-REL-01).
//!
//! A crash is simulated by dropping the session without saving, and by `mem::forget` on a lock
//! where the files a killed process leaves behind matter.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use dimo_core::Timestamp;
use dimo_io::autosave::{self, recover_unsaved};
use dimo_io::journal::journal_path;
use dimo_io::lock::{LockOwner, ProjectLock, lock_path};
use dimo_io::project::{Layout, ProjectError, ProjectSession};

fn owner(user: &str) -> LockOwner {
    LockOwner {
        user: user.into(),
        pid: 7,
        since: Timestamp::parse("2026-10-09T10:00:00Z").unwrap(),
    }
}

/// Creates an autosaved project in `dir`, edits it and journals the edits.
fn crashed_unsaved(dir: &Path, id: &str) -> dimo_core::Project {
    let mut env = common::env();
    let base = autosave::base_path(dir, id);
    let mut session =
        ProjectSession::create_autosaved(common::info(), common::drawing(&mut env), &base).unwrap();
    assert_eq!(session.autosave_path(), Some(base.as_path()));
    common::edit(session.document_mut(), &mut env);
    assert_eq!(session.flush_journal().unwrap(), 8);
    assert!(journal_path(&base).exists());
    session.project().clone()
    // Dropped without saving: the crash.
}

#[test]
fn unsaved_project_is_restored_after_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let edited = crashed_unsaved(dir.path(), "a");

    let recovered = recover_unsaved(dir.path(), &owner("anna"))
        .unwrap()
        .unwrap();
    assert_eq!(recovered.report.recovered_entries, 8);
    assert_eq!(*recovered.session.project(), edited);
    assert!(recovered.session.is_modified());
    assert!(recovered.session.location().is_none());
    assert!(lock_path(&autosave::base_path(dir.path(), "a")).exists());
}

#[test]
fn saving_a_restored_project_removes_the_autosave_files() {
    let dir = tempfile::tempdir().unwrap();
    let autosave_dir = dir.path().join("autosave");
    fs::create_dir(&autosave_dir).unwrap();
    let edited = crashed_unsaved(&autosave_dir, "a");

    let recovered = recover_unsaved(&autosave_dir, &owner("anna"))
        .unwrap()
        .unwrap();
    let mut session = recovered.session;
    let target = dir.path().join("part.dimo");
    session.save_as(&target, Layout::Zip).unwrap();
    drop(recovered.lock);
    assert_eq!(fs::read_dir(&autosave_dir).unwrap().count(), 0);
    let (reopened, report) = ProjectSession::open(&target).unwrap();
    assert_eq!(*reopened.project(), edited);
    assert_eq!(report.recovered_entries, 0);
}

#[test]
fn discarding_removes_the_autosave_files() {
    let dir = tempfile::tempdir().unwrap();
    crashed_unsaved(dir.path(), "a");
    let mut recovered = recover_unsaved(dir.path(), &owner("anna"))
        .unwrap()
        .unwrap();
    recovered.session.discard_journal().unwrap();
    drop(recovered.lock);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn unchanged_unsaved_projects_are_deleted_not_restored() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = common::env();
    let base = autosave::base_path(dir.path(), "empty");
    drop(
        ProjectSession::create_autosaved(common::info(), common::drawing(&mut env), &base).unwrap(),
    );
    assert!(base.exists());
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn projects_held_by_a_running_instance_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    crashed_unsaved(dir.path(), "a");
    let held = ProjectLock::acquire(&autosave::base_path(dir.path(), "a"), &owner("ben")).unwrap();
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_none()
    );
    drop(held);
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_crashed_lock_does_not_block_recovery() {
    let dir = tempfile::tempdir().unwrap();
    crashed_unsaved(dir.path(), "a");
    // A killed process leaves its lock file, but the operating system released the lock.
    fs::write(
        lock_path(&autosave::base_path(dir.path(), "a")),
        b"{\"user\":\"gone\",\"pid\":1,\"since\":\"2026-01-01T00:00:00Z\"}\n",
    )
    .unwrap();
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn unreadable_base_files_are_moved_aside() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(autosave::base_path(dir.path(), "broken"), b"not a zip").unwrap();
    assert!(
        recover_unsaved(dir.path(), &owner("anna"))
            .unwrap()
            .is_none()
    );
    assert!(dir.path().join("broken.dimo.stale").exists());
    assert_eq!(
        autosave::candidates(dir.path()).unwrap(),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn second_instance_cannot_lock_an_open_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.dimo");
    let _held = ProjectLock::acquire(&path, &owner("anna")).unwrap();
    let error = ProjectLock::acquire(&path, &owner("ben")).unwrap_err();
    assert!(matches!(error, ProjectError::InUse { .. }), "{error:?}");
    if cfg!(unix) {
        assert_eq!(
            error.to_string(),
            "the project is open in another Dimo instance (user anna)"
        );
    }
}

#[test]
fn missing_autosave_folder_has_no_candidates() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        autosave::candidates(&dir.path().join("missing")).unwrap(),
        Vec::<PathBuf>::new()
    );
}
