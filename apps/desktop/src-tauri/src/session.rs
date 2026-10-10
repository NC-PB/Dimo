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

use std::path::{Path, PathBuf};

use dimo_core::{Command, Environment, Patch, Project, ProjectInfo, Sha256Hex, Size};
use dimo_io::autosave;
use dimo_io::lock::{LockOwner, ProjectLock};
use dimo_io::project::{
    Layout, OpenReport, ProjectError, ProjectSession, SheetInfo, import_drawing,
};
use dimo_pdf::ContentHash;
use dimo_pdf::tiles::TileService;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::env::DesktopEnvironment;
use crate::ipc::{CommandError, DocumentInfo};

/// Autosave state of the open project (NFR-REL-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Autosave {
    /// No changes since the project was opened or saved.
    Clean,
    /// Unsaved changes, all written to the autosave journal.
    Journaled,
    /// Unsaved changes that are not in a journal: the last write failed, or there is no
    /// place for a journal. The timer retries.
    Failed {
        /// Why, for the user and for bug reports.
        message: String,
    },
}

/// File, undo and autosave state of the open project, for toolbar and title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ProjectStatus {
    /// File name of the project without the directory; `null` until it is saved.
    pub file_name: Option<String>,
    /// True if there are changes that are not in the project file.
    pub modified: bool,
    /// True if undo has a step.
    pub can_undo: bool,
    /// True if redo has a step.
    pub can_redo: bool,
    /// Autosave state.
    pub autosave: Autosave,
}

/// Everything the frontend shows of an open project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ProjectSnapshot {
    /// Number of patches applied since the project was loaded.
    pub revision: u32,
    /// The project state.
    pub project: Project,
    /// The drawing of the current revision, opened for tile rendering.
    pub drawing: DocumentInfo,
    /// File, undo and autosave state.
    pub status: ProjectStatus,
}

/// What opening a project found worth telling the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct OpenNotice {
    /// A project that was never saved was restored from the autosave folder after a crash.
    pub restored_unsaved: bool,
    /// Changes replayed from the autosave journal after a crash.
    pub recovered_changes: u32,
    /// The last change before the crash was written incompletely and dropped.
    pub dropped_incomplete_change: bool,
    /// A journal that did not fit the project was moved to this file and not applied.
    pub set_aside_journal: Option<String>,
    /// The file had this older schema version and was converted (NFR-REL-03).
    pub migrated_from: Option<u32>,
}

impl OpenNotice {
    /// The notice for an open report, `None` if there is nothing to tell.
    fn from_report(report: &OpenReport, restored_unsaved: bool) -> Option<Self> {
        let notice = Self {
            restored_unsaved,
            recovered_changes: u32::try_from(report.recovered_entries).unwrap_or(u32::MAX),
            dropped_incomplete_change: report.dropped_incomplete_line,
            set_aside_journal: report
                .discarded_journal
                .as_ref()
                .map(|(_, path)| path.display().to_string()),
            migrated_from: report.migrated_from,
        };
        let empty = !notice.restored_unsaved
            && notice.recovered_changes == 0
            && !notice.dropped_incomplete_change
            && notice.set_aside_journal.is_none()
            && notice.migrated_from.is_none();
        (!empty).then_some(notice)
    }
}

/// The project was created, opened, restored or closed. Event `project-loaded`; also the result
/// of `project_state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct ProjectLoaded {
    /// Increases with every load or close; patches name the session they belong to.
    pub session: u32,
    /// The project, `null` when none is open.
    pub snapshot: Option<ProjectSnapshot>,
    /// Shown once to the user, for example after crash recovery.
    pub notice: Option<OpenNotice>,
}

/// A command, undo or redo changed the project. Event `project-patched`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct ProjectPatched {
    /// Session the patch belongs to.
    pub session: u32,
    /// Revision after the patch: the previous revision plus one.
    pub revision: u32,
    /// What changed.
    pub patch: Patch,
    /// State after the patch.
    pub status: ProjectStatus,
}

/// File or autosave state changed without a patch: after save, save as, or an autosave write
/// on the timer. Event `project-status-changed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct ProjectStatusChanged {
    /// Session the status belongs to.
    pub session: u32,
    /// The new state.
    pub status: ProjectStatus,
}

/// The user asked to close the window while the project has unsaved changes. The frontend asks
/// what to do and answers with `confirm_close`. Event `close-requested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
pub struct CloseRequested;

/// An immutable copy of what an export reads, so the job runs without the session.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSnapshot {
    /// The project state when the export started.
    pub project: Project,
    /// Time of the last change of the project, for dates written into exports (FR-EXP-11).
    pub modified: dimo_core::Timestamp,
    /// The drawing file of the current revision, when the export needs it.
    pub drawing: Option<Vec<u8>>,
}

/// A project that is open in the app.
#[derive(Debug)]
struct OpenProject {
    project: ProjectSession,
    /// Held while open; dropping it releases the project for other instances.
    #[allow(dead_code, reason = "held for its Drop")]
    lock: Option<ProjectLock>,
    drawing: DocumentInfo,
    hash: ContentHash,
    revision: u32,
    /// Error of the last journal write, cleared by the next successful one.
    journal_error: Option<String>,
    /// Autosave state last sent to the frontend.
    reported: Autosave,
}

impl OpenProject {
    fn autosave(&self) -> Autosave {
        if let Some(message) = &self.journal_error {
            return Autosave::Failed {
                message: message.clone(),
            };
        }
        if !self.project.is_modified() {
            return Autosave::Clean;
        }
        if self.project.location().is_some() || self.project.autosave_path().is_some() {
            Autosave::Journaled
        } else {
            Autosave::Failed {
                message: "no autosave folder available".to_owned(),
            }
        }
    }

    fn status(&mut self) -> ProjectStatus {
        let autosave = self.autosave();
        self.reported = autosave.clone();
        let document = self.project.document();
        ProjectStatus {
            file_name: self
                .project
                .location()
                .and_then(|(path, _)| path.file_name())
                .map(|name| name.to_string_lossy().into_owned()),
            modified: self.project.is_modified(),
            can_undo: document.can_undo(),
            can_redo: document.can_redo(),
            autosave,
        }
    }

    fn snapshot(&mut self) -> ProjectSnapshot {
        ProjectSnapshot {
            revision: self.revision,
            project: self.project.project().clone(),
            drawing: self.drawing.clone(),
            status: self.status(),
        }
    }

    /// Writes pending audit entries to the journal and remembers a failure (D-28).
    fn flush(&mut self) {
        self.journal_error = match self.project.flush_journal() {
            Ok(_) => None,
            Err(error) => {
                tracing::warn!("autosave journal write failed: {error}");
                Some(error.to_string())
            }
        };
    }

    fn path(&self) -> Option<&Path> {
        self.project.location().map(|(path, _)| path)
    }
}

/// The project state of the app window. See the module docs.
#[derive(Debug)]
pub struct AppSession {
    env: DesktopEnvironment,
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

    /// What an export job needs: a copy of the project, its last change time and, if
    /// `with_drawing`, the drawing file of the current revision.
    pub fn export_snapshot(&self, with_drawing: bool) -> Result<ExportSnapshot, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        let drawing = if with_drawing {
            Some(
                open.project
                    .current_drawing()
                    .ok_or_else(|| CommandError::Project {
                        message: "the drawing of the current revision is missing".to_owned(),
                    })?
                    .to_vec(),
            )
        } else {
            None
        };
        Ok(ExportSnapshot {
            project: open.project.project().clone(),
            modified: open.project.modified(),
            drawing,
        })
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

    /// Creates a new, unsaved project from the drawing at `pdf` (FR-DOC-07) and replaces the
    /// open project. `discard` allows dropping unsaved changes of the open project.
    pub fn create_from_drawing(
        &mut self,
        tiles: &TileService,
        pdf: &Path,
        discard: bool,
    ) -> Result<ProjectLoaded, CommandError> {
        self.ensure_can_replace(discard)?;
        let bytes = std::fs::read(pdf).map_err(|e| CommandError::Io {
            message: format!("{}: {e}", pdf.display()),
        })?;
        let name = pdf
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let doc = tiles.open_document(bytes.clone())?;
        let hash = *doc.content_hash();
        let imported = (|| {
            let mut sheets = Vec::with_capacity(doc.sheet_count());
            for (index, size) in doc.sheet_sizes().iter().enumerate() {
                sheets.push(SheetInfo {
                    size: Size {
                        width: size.width,
                        height: size.height,
                    },
                    kind: doc.analyze_sheet(index)?.kind,
                });
            }
            let analyzed = Sha256Hex::parse(&hash.to_hex()).map_err(|e| CommandError::Io {
                message: e.to_string(),
            })?;
            Ok::<_, CommandError>(import_drawing(
                bytes,
                &name,
                &analyzed,
                &sheets,
                &mut self.env,
            )?)
        })();
        let drawing = match imported {
            Ok(drawing) => drawing,
            Err(error) => {
                self.close_unused_document(tiles, &hash);
                return Err(error);
            }
        };
        let (project, lock, journal_error) = match self.autosave_base() {
            Ok(Some((base, lock))) => {
                match ProjectSession::create_autosaved(
                    ProjectInfo::default(),
                    drawing.clone(),
                    &base,
                ) {
                    Ok(project) => (project, Some(lock), None),
                    Err(error) => {
                        tracing::warn!("new project without autosave: {error}");
                        let project = ProjectSession::create(ProjectInfo::default(), drawing);
                        (project, None, Some(error.to_string()))
                    }
                }
            }
            Ok(None) => (
                ProjectSession::create(ProjectInfo::default(), drawing),
                None,
                None,
            ),
            Err(error) => {
                tracing::warn!("new project without autosave: {error}");
                let project = ProjectSession::create(ProjectInfo::default(), drawing);
                (project, None, Some(error.to_string()))
            }
        };
        let open = OpenProject {
            project,
            lock,
            drawing: DocumentInfo::of(&doc, &name),
            hash,
            revision: 0,
            journal_error,
            reported: Autosave::Clean,
        };
        Ok(self.replace(tiles, open, discard, None))
    }

    /// A new autosave base path in the autosave folder, locked.
    fn autosave_base(&mut self) -> Result<Option<(PathBuf, ProjectLock)>, ProjectError> {
        let Some(dir) = self.autosave_dir.clone() else {
            return Ok(None);
        };
        std::fs::create_dir_all(&dir).map_err(|e| ProjectError::Io {
            path: dir.clone(),
            source: e,
        })?;
        let base = autosave::base_path(&dir, &self.env.new_uuid().to_string());
        let lock = ProjectLock::acquire(&base, &self.owner())?;
        Ok(Some((base, lock)))
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

    /// Opens the drawing of the current revision in the tile service.
    fn with_drawing(
        tiles: &TileService,
        project: ProjectSession,
        lock: Option<ProjectLock>,
    ) -> Result<OpenProject, CommandError> {
        let revision =
            project
                .project()
                .current_revision()
                .ok_or_else(|| CommandError::Project {
                    message: "the project has no current drawing revision".to_owned(),
                })?;
        let bytes = project
            .drawing(&revision.sha256)
            .ok_or_else(|| CommandError::Project {
                message: format!("drawing {} is missing", revision.sha256),
            })?;
        let doc = tiles.open_document(bytes.to_vec())?;
        Ok(OpenProject {
            drawing: DocumentInfo::of(&doc, &revision.file_name),
            hash: *doc.content_hash(),
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
        keep: Option<&ContentHash>,
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

    fn close_unused_document(&self, tiles: &TileService, hash: &ContentHash) {
        if self.open.as_ref().is_none_or(|o| o.hash != *hash) {
            tiles.close_document(hash);
        }
    }

    /// Saves to the project file. Fails with [`CommandError::Project`] if it has none: use
    /// [`AppSession::save_as`].
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
        // M2 decision 4: a custom table must be stored before the settings list it.
        if let Some(id) = open.project.missing_table(&command) {
            return Err(CommandError::Rejected {
                reason: crate::ipc::RejectReason::InvalidProjectSetting,
                message: format!("custom tolerance table {id} is not stored in the project"),
            });
        }
        let patch = open
            .project
            .document_mut()
            .execute(command, &mut self.env)?;
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

    /// The journal file of the open project, if one is open.
    pub fn journal_path(&self) -> Option<PathBuf> {
        self.open
            .as_ref()
            .and_then(|o| o.project.journal_path().map(Path::to_path_buf))
    }

    /// The open project, for tests and dev tools.
    pub fn project(&self) -> Option<&Project> {
        self.open.as_ref().map(|o| o.project.project())
    }

    /// What recognition needs for `sheet` of the current drawing revision (T2.6): the drawing
    /// in the tile service, the page, the unit and the project settings. Reads only.
    pub fn recognition_target(
        &self,
        sheet: dimo_core::SheetId,
    ) -> Result<RecognitionTarget, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        let project = open.project.project();
        let found = project
            .current_revision()
            .and_then(|r| r.sheets.iter().find(|s| s.id == sheet))
            .ok_or_else(|| CommandError::InvalidArgument {
                message: format!("sheet {sheet} is not a sheet of the current drawing"),
            })?;
        let custom_tables = project
            .settings
            .tolerance
            .custom_tables
            .iter()
            .filter_map(|t| {
                let bytes = open.project.table(&t.sha256)?;
                Some((
                    t.table.id.clone(),
                    String::from_utf8_lossy(bytes).into_owned(),
                ))
            })
            .collect();
        Ok(RecognitionTarget {
            drawing: open.hash,
            page: usize::try_from(found.index).unwrap_or(usize::MAX),
            unit: found.unit,
            settings: project.settings.clone(),
            custom_tables,
        })
    }
}

/// What recognition reads from the open project for one sheet (T2.6).
#[derive(Debug, Clone, PartialEq)]
pub struct RecognitionTarget {
    /// Content hash of the drawing, its key in the tile service.
    pub drawing: ContentHash,
    /// Zero based page of the sheet in the drawing.
    pub page: usize,
    /// Unit of the sheet (D-20).
    pub unit: dimo_core::Unit,
    /// Project settings, for the tolerance engine.
    pub settings: dimo_core::ProjectSettings,
    /// Custom tolerance tables stored in the project as (id, file text) (M2 decision 4).
    pub custom_tables: Vec<(String, String)>,
}

/// True if both paths name the same existing file or folder.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
