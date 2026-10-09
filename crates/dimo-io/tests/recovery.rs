//! Autosave journal and crash recovery (T1.3, NFR-REL-01, D-28).
//!
//! A crash is simulated by dropping the session without saving: nothing runs on drop, so
//! the files on disk are exactly what a killed process leaves behind.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use dimo_core::{Command, FixedEnvironment, LockReason, Project, ProjectInfo};
use dimo_io::journal::{self, journal_path, stale_path};
use dimo_io::project::{Layout, ProjectFile, ProjectSession};

/// A saved project with a drawing and no characteristics; returns its path.
fn saved_project(dir: &Path, layout: Layout) -> (PathBuf, FixedEnvironment) {
    let mut env = common::env();
    let mut session = ProjectSession::create(common::info(), common::drawing(&mut env));
    let path = dir.join("part.dimo");
    session.save_as(&path, layout).unwrap();
    (path, env)
}

fn rename(session: &mut ProjectSession, env: &mut FixedEnvironment, name: &str) {
    session
        .document_mut()
        .execute(
            Command::UpdateProjectInfo {
                info: ProjectInfo {
                    part_name: name.into(),
                    ..common::info()
                },
            },
            env,
        )
        .unwrap();
}

#[test]
fn crash_after_journal_write_restores_all_commands() {
    for layout in [Layout::Zip, Layout::Folder] {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut env) = saved_project(dir.path(), layout);
        let saved = fs::read(&path).ok();

        let (mut session, report) = ProjectSession::open(&path).unwrap();
        assert_eq!(report.recovered_entries, 0);
        assert!(
            session.journal_path().is_none(),
            "no journal without changes"
        );

        // Two command batches, each flushed as the shell does after every batch.
        common::edit(session.document_mut(), &mut env);
        let written = session.flush_journal().unwrap();
        assert!(written >= 8, "{written}");
        session.document_mut().undo(&mut env).unwrap();
        session.document_mut().undo(&mut env).unwrap();
        session.document_mut().redo(&mut env).unwrap();
        session.flush_journal().unwrap();
        let expected: Project = session.project().clone();
        let expected_audit = session.audit().to_vec();
        assert!(journal_path(&path).exists());
        drop(session); // crash

        assert_eq!(fs::read(&path).ok(), saved, "the project file is untouched");
        let (session, report) = ProjectSession::open(&path).unwrap();
        assert_eq!(report.recovered_entries, written + 3);
        assert!(!report.dropped_incomplete_line);
        assert_eq!(*session.project(), expected, "{layout:?}");
        assert_eq!(session.audit(), expected_audit.as_slice());
        assert!(session.is_modified());
    }
}

#[test]
fn recovery_survives_repeated_crashes_and_save_compacts() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);

    let (mut session, _) = ProjectSession::open(&path).unwrap();
    rename(&mut session, &mut env, "first");
    session.flush_journal().unwrap();
    drop(session); // crash

    let (mut session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 1);
    rename(&mut session, &mut env, "second");
    session.flush_journal().unwrap();
    drop(session); // crash again

    let (mut session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 2);
    assert_eq!(session.project().info.part_name, "second");

    session.save().unwrap();
    assert!(!journal_path(&path).exists(), "save deletes the journal");
    assert!(!session.is_modified());
    let loaded = ProjectFile::load(&path).unwrap().file;
    assert_eq!(loaded.project, *session.project());
    assert_eq!(
        loaded.audit.len(),
        2,
        "recovered entries are in the audit log"
    );

    let (session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 0);
    assert_eq!(session.project().info.part_name, "second");
}

#[test]
fn an_incomplete_last_line_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);
    let (mut session, _) = ProjectSession::open(&path).unwrap();
    rename(&mut session, &mut env, "kept");
    session.flush_journal().unwrap();
    drop(session);

    // The crash hit in the middle of the next append.
    let mut file = OpenOptions::new()
        .append(true)
        .open(journal_path(&path))
        .unwrap();
    file.write_all(br#"{"timestamp":"2026-03-01T09:00:00Z","us"#)
        .unwrap();
    drop(file);

    let (mut session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 1);
    assert!(report.dropped_incomplete_line);
    assert_eq!(session.project().info.part_name, "kept");

    // Appending continues after the cut, so the next recovery reads every line.
    rename(&mut session, &mut env, "later");
    session.flush_journal().unwrap();
    drop(session);
    let (session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 2);
    assert!(!report.dropped_incomplete_line);
    assert_eq!(session.project().info.part_name, "later");
}

#[test]
fn a_journal_for_another_state_is_moved_aside() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);
    let (mut session, _) = ProjectSession::open(&path).unwrap();
    rename(&mut session, &mut env, "lost");
    session.flush_journal().unwrap();
    drop(session);

    // The project file changed behind the journal's back (for example replaced by a copy).
    let mut other = ProjectFile::load(&path).unwrap().file;
    other.project.info.customer = "someone else".into();
    other.save(&path, Layout::Zip).unwrap();

    let (session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report.recovered_entries, 0);
    let (reason, moved_to) = report.discarded_journal.unwrap();
    assert!(reason.contains("another state"), "{reason}");
    assert_eq!(moved_to, stale_path(&journal_path(&path)));
    assert!(moved_to.exists());
    assert!(!journal_path(&path).exists());
    assert_eq!(session.project().info.customer, "someone else");
    assert_eq!(session.project().info.part_name, "Bracket");
}

#[test]
fn entries_that_do_not_apply_are_never_half_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);
    let base = ProjectFile::load(&path).unwrap().file.project;

    let (mut session, _) = ProjectSession::open(&path).unwrap();
    rename(&mut session, &mut env, "one");
    session.flush_journal().unwrap();
    let mut journal = OpenOptions::new()
        .append(true)
        .open(journal_path(&path))
        .unwrap();
    drop(session);
    // A second copy of the same entry cannot apply: its "before" state no longer matches.
    let lines = fs::read_to_string(journal_path(&path)).unwrap();
    let entry = lines.lines().nth(1).unwrap();
    writeln!(journal, "{entry}").unwrap();
    drop(journal);

    let mut project = base.clone();
    let recovery = journal::recover(&journal_path(&path), &mut project).unwrap();
    assert!(matches!(recovery, journal::Recovery::Discarded { .. }));
    assert_eq!(project, base, "nothing was applied");
}

#[test]
fn save_as_moves_the_project_and_drops_the_old_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);
    let (mut session, _) = ProjectSession::open(&path).unwrap();
    session
        .document_mut()
        .execute(
            Command::LockNumbering {
                reason: LockReason::Manual,
            },
            &mut env,
        )
        .unwrap();
    session.flush_journal().unwrap();
    assert!(journal_path(&path).exists());

    let copy = dir.path().join("copy");
    session.save_as(&copy, Layout::Folder).unwrap();
    assert!(!journal_path(&path).exists());
    assert_eq!(
        session.location().unwrap(),
        (copy.as_path(), Layout::Folder)
    );
    assert!(
        ProjectFile::load(&copy)
            .unwrap()
            .file
            .project
            .is_numbering_locked()
    );
    assert!(
        !ProjectFile::load(&path)
            .unwrap()
            .file
            .project
            .is_numbering_locked()
    );
}

#[test]
fn closing_without_saving_discards_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut env) = saved_project(dir.path(), Layout::Zip);
    let (mut session, _) = ProjectSession::open(&path).unwrap();
    rename(&mut session, &mut env, "unwanted");
    session.flush_journal().unwrap();
    session.discard_journal().unwrap();
    drop(session);
    let (session, report) = ProjectSession::open(&path).unwrap();
    assert_eq!(report, dimo_io::project::OpenReport::default());
    assert_eq!(session.project().info.part_name, "Bracket");
}

#[test]
fn unsaved_projects_keep_entries_in_the_audit_log() {
    let mut env = common::env();
    let mut session = ProjectSession::create(common::info(), common::drawing(&mut env));
    rename(&mut session, &mut env, "draft");
    assert_eq!(session.flush_journal().unwrap(), 1);
    assert!(session.journal_path().is_none());
    assert_eq!(session.audit().len(), 1);
    assert!(matches!(
        session.save(),
        Err(dimo_io::project::ProjectError::NoPath)
    ));
}
