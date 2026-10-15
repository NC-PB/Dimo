//! The open project of the app window and its IPC payloads (T1.5, 05 Architecture principle 1).
//!
//! [`AppSession`] holds at most one open project: the [`ProjectSession`] from `dimo-io` (document,
//! undo history, journal), the lock that keeps other instances away, and the drawing opened in
//! the tile service. It has no Tauri types, so tests drive it directly; `project.rs` wraps it in
//! commands and events.
//!
//! # What the frontend receives
//!
//! - [`ProjectLoaded`] when a project is created, opened, restored or closed: a full
//!   [`ProjectSnapshot`] or `null`. `session` increases with every load or close.
//! - [`ProjectPatched`] after every command, undo and redo that changed something: the
//!   [`Patch`] and a `revision` that increases by one per patch within a session.
//! - [`ProjectStatusChanged`] after save and when the autosave state changes.
//!
//! The frontend applies them in one place (`src/lib/stores/project.svelte.ts`). A patch whose
//! revision does not follow the last one makes it fetch the full state again.
//!
//! # Autosave (NFR-REL-01, D-28)
//!
//! The journal is written after every command, undo and redo, and by a 30 s timer that retries
//! failed writes. A saved project journals next to its file. A new project journals against an
//! autosave base file in `<app data>/autosave/` ([`dimo_io::autosave`]); after a crash
//! [`AppSession::recover_unsaved`] restores it at the next start.

mod open;
mod payload;
mod reads;

use std::path::{Path, PathBuf};

use dimo_core::{Command, Environment, Patch, ProjectInfo, Sha256Hex};
use dimo_io::autosave;
use dimo_io::lock::{LockOwner, ProjectLock};
use dimo_io::project::{
    AutosaveTarget, Layout, NewProject, ProjectSession, create_from_drawing_file, open_drawing,
};
use dimo_pdf::tiles::TileService;
use dimo_report::issue::IssueDefaults;

use crate::env::DesktopEnvironment;
use crate::ipc::{AutosaveReason, CommandError, DocumentInfo};
use open::{OpenProject, same_file};
pub use payload::{
    Autosave, CloseRequested, ExportSnapshot, OpenNotice, ProjectLoaded, ProjectPatched,
    ProjectSnapshot, ProjectStatus, ProjectStatusChanged, RecognitionTarget, SnapshotState,
};

/// The project state of the app window. See the module docs.
#[derive(Debug)]
pub struct AppSession {
    env: DesktopEnvironment,
    /// Company name of the app settings, the report info of new projects starts with it.
    company_name: String,
    autosave_dir: Option<PathBuf>,
    session: u32,
    open: Option<OpenProject>,
    /// Notice of a project restored at startup, before the frontend listened to events.
    pending_notice: Option<OpenNotice>,
}

impl AppSession {
    /// A session without a project. New projects journal in `autosave_dir`; without one they
    /// are not protected before their first save.
    pub fn new(autosave_dir: Option<PathBuf>) -> Self {
        Self {
            env: DesktopEnvironment::new(),
            company_name: String::new(),
            autosave_dir,
            session: 0,
            open: None,
            pending_notice: None,
        }
    }

    /// Sets the audit user name (D-27, for the settings view of T1.9).
    pub fn set_user_name(&mut self, name: &str) {
        self.env.set_user_name(name);
    }

    /// Sets the company name that new projects start with in their report info (M3 decision 15).
    pub fn set_company_name(&mut self, name: &str) {
        name.trim().clone_into(&mut self.company_name);
    }

    /// What report issuing fills in (M3 decision 15, T3.9): the company name of the app
    /// settings, the audit user (D-27) as inspector and today's local date. The rules that use
    /// them are in `dimo_report::issue`.
    pub fn issue_defaults(&self) -> IssueDefaults {
        IssueDefaults {
            company_name: self.company_name.clone(),
            inspector: self.env.user_name(),
            today: self.env.local_date(),
        }
    }

    /// The audit user name.
    pub fn user_name(&self) -> String {
        self.env.user_name()
    }

    /// The current state for a frontend that just started listening. Hands out the notice of
    /// a project restored at startup once.
    pub fn state(&mut self) -> ProjectLoaded {
        ProjectLoaded {
            session: self.session,
            snapshot: self.open.as_mut().map(OpenProject::snapshot),
            notice: self.pending_notice.take(),
        }
    }

    /// The session number: increases with every load or close.
    pub fn session(&self) -> u32 {
        self.session
    }

    /// True if a project is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// True if closing would lose changes, so the user must be asked first.
    pub fn has_unsaved_changes(&self) -> bool {
        self.open.as_ref().is_some_and(|o| o.project.is_modified())
    }

    /// True if the open project has a file, so `save` needs no file dialog.
    pub fn has_file(&self) -> bool {
        self.open.as_ref().is_some_and(|o| o.path().is_some())
    }

    /// The file (or folder) of the open project, `None` without a project or before the first
    /// save. For the recent projects list (T3.16).
    pub fn file_path(&self) -> Option<PathBuf> {
        self.open
            .as_ref()
            .and_then(OpenProject::path)
            .map(Path::to_path_buf)
    }

    /// Suggested file name for "save as": the project file name, else the drawing name with the
    /// project extension.
    pub fn suggested_file_name(&self) -> String {
        if let Some(name) = self
            .open
            .as_ref()
            .and_then(OpenProject::path)
            .and_then(Path::file_name)
        {
            return name.to_string_lossy().into_owned();
        }
        format!("{}.{}", self.file_stem(), dimo_io::project::EXTENSION)
    }

    /// Base of suggested file names: the project file name, else the drawing name, without
    /// the extension; `project` without either.
    pub fn file_stem(&self) -> String {
        let Some(open) = &self.open else {
            return "project".to_owned();
        };
        let name = open.path().and_then(Path::file_name).map_or_else(
            || open.drawing.name.clone(),
            |n| n.to_string_lossy().into_owned(),
        );
        Path::new(&name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "project".to_owned())
    }

    /// Fixes today's date of [`Self::issue_defaults`]; `None` restores the real one.
    /// For tests and scripted runs.
    pub fn set_fixed_local_date(&mut self, date: Option<&str>) {
        self.env.set_fixed_local_date(date);
    }

    fn owner(&mut self) -> LockOwner {
        LockOwner {
            user: self.env.user_name(),
            pid: std::process::id(),
            since: self.env.now(),
        }
    }

    fn ensure_can_replace(&self, discard: bool) -> Result<(), CommandError> {
        if self.has_unsaved_changes() && !discard {
            return Err(CommandError::UnsavedChanges);
        }
        Ok(())
    }

    /// Creates a new, unsaved project from the drawing file at `path` (FR-DOC-07) and
    /// replaces the open project. `discard` allows dropping unsaved changes of the open
    /// project. Reading, opening and importing the drawing is `dimo-io`'s job
    /// ([`dimo_io::project::create_from_drawing_file`]); the drawing opens in the tile service.
    pub fn create_from_drawing(
        &mut self,
        tiles: &TileService,
        path: &Path,
        discard: bool,
    ) -> Result<ProjectLoaded, CommandError> {
        self.ensure_can_replace(discard)?;
        let autosave = self.autosave_dir.clone().map(|dir| AutosaveTarget {
            dir,
            owner: self.owner(),
        });
        let options = NewProject {
            info: ProjectInfo::default(),
            // M3 decision 15: part of the initial state, so no undo step and not modified.
            report_info: dimo_report::issue::initial_report_info(&self.issue_defaults()),
            autosave,
        };
        let created = create_from_drawing_file(path, tiles, options, &mut self.env)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let open = OpenProject {
            drawing: DocumentInfo::of(created.source.as_ref(), &name)?,
            hash: created.source.render_key(),
            project: created.session,
            lock: created.lock,
            revision: 0,
            journal_error: created
                .autosave_error
                .map(|e| (AutosaveReason::of(&e), e.to_string())),
            reported: Autosave::Clean,
        };
        Ok(self.replace(tiles, open, discard, None))
    }

    /// Opens the project file or folder at `path` and replaces the open project. Replays its
    /// journal after a crash. Opening the open project again reloads it from disk.
    pub fn open_file(
        &mut self,
        tiles: &TileService,
        path: &Path,
        discard: bool,
    ) -> Result<ProjectLoaded, CommandError> {
        self.ensure_can_replace(discard)?;
        let reopening = self
            .open
            .as_ref()
            .and_then(OpenProject::path)
            .is_some_and(|open| same_file(open, path));
        if reopening {
            // Our own lock would refuse the second open.
            self.close(Some(tiles), discard)?;
        }
        let owner = self.owner();
        let lock = ProjectLock::acquire(path, &owner)?;
        let (project, report) = ProjectSession::open(path)?;
        let open = Self::with_drawing(tiles, project, Some(lock))?;
        let notice = OpenNotice::from_report(&report, false);
        Ok(self.replace(tiles, open, discard, notice))
    }

    /// Restores the newest project that was never saved and is left in the autosave folder by
    /// a crash. Its notice is handed out by the next [`AppSession::state`]. Does nothing if a
    /// project is open.
    pub fn recover_unsaved(&mut self, tiles: &TileService) -> Result<bool, CommandError> {
        let Some(dir) = self.autosave_dir.clone() else {
            return Ok(false);
        };
        if self.open.is_some() {
            return Ok(false);
        }
        let owner = self.owner();
        let Some(recovered) = autosave::recover_unsaved(&dir, &owner)? else {
            return Ok(false);
        };
        let open = Self::with_drawing(tiles, recovered.session, Some(recovered.lock))?;
        let notice = OpenNotice::from_report(&recovered.report, true);
        let loaded = self.replace(tiles, open, false, notice);
        self.pending_notice = loaded.notice;
        Ok(true)
    }

    /// Opens the drawing of the current revision in the tile service, through the source its
    /// revision names ([`dimo_io::project::open_drawing`]).
    fn with_drawing(
        tiles: &TileService,
        project: ProjectSession,
        lock: Option<ProjectLock>,
    ) -> Result<OpenProject, CommandError> {
        let opened = open_drawing(&project, tiles)?;
        let name = project
            .project()
            .current_revision()
            .map(|r| r.file_name.clone())
            .unwrap_or_default();
        Ok(OpenProject {
            drawing: DocumentInfo::of(opened.source.as_ref(), &name)?,
            hash: opened.source.render_key(),
            project,
            lock,
            revision: 0,
            journal_error: None,
            reported: Autosave::Clean,
        })
    }

    /// Puts `open` in place of the open project, which is closed: its journal is discarded if
    /// it had no unsaved changes or `discard` is set, else kept for recovery.
    fn replace(
        &mut self,
        tiles: &TileService,
        open: OpenProject,
        discard: bool,
        notice: Option<OpenNotice>,
    ) -> ProjectLoaded {
        let keep = open.hash;
        if let Some(old) = self.open.replace(open) {
            Self::release(old, discard, Some(tiles), Some(&keep));
        }
        self.session += 1;
        ProjectLoaded {
            session: self.session,
            snapshot: self.open.as_mut().map(OpenProject::snapshot),
            notice,
        }
    }

    /// Closes the open project. Unsaved changes need `discard`; they are then lost.
    pub fn close(
        &mut self,
        tiles: Option<&TileService>,
        discard: bool,
    ) -> Result<ProjectLoaded, CommandError> {
        self.ensure_can_replace(discard)?;
        if let Some(old) = self.open.take() {
            Self::release(old, discard, tiles, None);
        }
        self.session += 1;
        Ok(ProjectLoaded {
            session: self.session,
            snapshot: None,
            notice: None,
        })
    }

    /// Ends the app: releases the project. Unsaved changes stay in the journal and are
    /// recovered at the next open or start, since the user did not choose to discard them.
    pub fn shutdown(&mut self, tiles: Option<&TileService>) {
        if let Some(mut old) = self.open.take() {
            old.flush();
            Self::release(old, false, tiles, None);
        }
    }

    fn release(
        mut old: OpenProject,
        discard: bool,
        tiles: Option<&TileService>,
        keep: Option<&Sha256Hex>,
    ) {
        if (discard || !old.project.is_modified())
            && let Err(error) = old.project.discard_journal()
        {
            tracing::warn!("cannot delete the journal of the closed project: {error}");
        }
        if let Some(tiles) = tiles
            && keep != Some(&old.hash)
        {
            tiles.close_document(&old.hash);
        }
        // Dropping `old` releases its lock.
    }

    /// Saves to the project file. Fails with [`CommandError::Project`] (`no_file`) if it has
    /// none: use [`AppSession::save_as`].
    pub fn save(&mut self) -> Result<ProjectStatusChanged, CommandError> {
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        open.project.save()?;
        open.journal_error = None;
        Ok(ProjectStatusChanged {
            session: self.session,
            status: open.status(),
        })
    }

    /// Saves to `path` as a ZIP file (or a project folder, if `path` is one), which becomes the
    /// project file. Locks the new file first.
    pub fn save_as(&mut self, path: &Path) -> Result<ProjectStatusChanged, CommandError> {
        if self
            .open
            .as_ref()
            .and_then(OpenProject::path)
            .is_some_and(|open| same_file(open, path))
        {
            return self.save();
        }
        let owner = self.owner();
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        let lock = ProjectLock::acquire(path, &owner)?;
        open.project.save_as(path, Layout::of(path))?;
        open.lock = Some(lock);
        open.journal_error = None;
        Ok(ProjectStatusChanged {
            session: self.session,
            status: open.status(),
        })
    }

    /// Executes a document command as one undo step and journals it (D-28).
    pub fn execute(&mut self, command: Command) -> Result<ProjectPatched, CommandError> {
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        let patch = open.project.execute(command, &mut self.env)?;
        Ok(Self::patched(self.session, open, patch))
    }

    /// Undoes the last command (NFR-REL-02).
    pub fn undo(&mut self) -> Result<ProjectPatched, CommandError> {
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        let patch = open.project.document_mut().undo(&mut self.env)?;
        Ok(Self::patched(self.session, open, patch))
    }

    /// Applies the last undone command again.
    pub fn redo(&mut self) -> Result<ProjectPatched, CommandError> {
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        let patch = open.project.document_mut().redo(&mut self.env)?;
        Ok(Self::patched(self.session, open, patch))
    }

    fn patched(session: u32, open: &mut OpenProject, patch: Patch) -> ProjectPatched {
        if !patch.is_empty() {
            open.revision += 1;
            open.flush();
        }
        ProjectPatched {
            session,
            revision: open.revision,
            patch,
            status: open.status(),
        }
    }

    /// The autosave timer (D-28): writes pending journal entries. Returns the new status if the
    /// autosave state changed since it was last sent.
    pub fn autosave_tick(&mut self) -> Option<ProjectStatusChanged> {
        let open = self.open.as_mut()?;
        open.flush();
        if open.autosave() == open.reported {
            return None;
        }
        Some(ProjectStatusChanged {
            session: self.session,
            status: open.status(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dimo_io::project::OpenReport;

    #[test]
    fn notice_only_when_there_is_something_to_tell() {
        assert_eq!(OpenNotice::from_report(&OpenReport::default(), false), None);
        let report = OpenReport {
            recovered_entries: 3,
            ..OpenReport::default()
        };
        let notice = OpenNotice::from_report(&report, false).unwrap();
        assert_eq!(notice.recovered_changes, 3);
        assert!(OpenNotice::from_report(&OpenReport::default(), true).is_some());
    }

    #[test]
    fn empty_session_has_no_project() {
        let mut session = AppSession::new(None);
        let state = session.state();
        assert_eq!(state.session, 0);
        assert!(state.snapshot.is_none());
        assert!(matches!(
            session.execute(Command::UnlockNumbering),
            Err(CommandError::NoProject)
        ));
        assert!(matches!(session.undo(), Err(CommandError::NoProject)));
        assert!(!session.has_unsaved_changes());
        assert_eq!(session.autosave_tick(), None);
        assert_eq!(session.suggested_file_name(), "project.dimo");
    }
}
