//! The IPC payloads of the open project (events and their parts) and the copies that exports
//! and recognition read from it.

use dimo_core::{Patch, Project, Sha256Hex};
use dimo_export::Source;
use dimo_io::project::OpenReport;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::ipc::{AutosaveReason, DocumentInfo};

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
        /// Why, for the frontend to translate.
        reason: AutosaveReason,
        /// Diagnostic text for the "Details" line and bug reports.
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
    pub(super) fn from_report(report: &OpenReport, restored_unsaved: bool) -> Option<Self> {
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
    /// Session and patch revision the copy was taken at. An issued report is recorded only if
    /// the project is still in this state when the file is written (M3 decision 17).
    pub state: SnapshotState,
}

impl ExportSnapshot {
    /// What the export renders from (`dimo_export::render`).
    pub fn source(&self) -> Source<'_> {
        Source {
            project: &self.project,
            modified: &self.modified,
            drawing: self.drawing.as_deref(),
        }
    }
}

/// Where in the history of the app session a snapshot was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotState {
    /// [`AppSession::session`].
    pub session: u32,
    /// Patch revision within the session.
    pub revision: u32,
}

/// What recognition reads from the open project for one sheet (T2.6).
#[derive(Debug, Clone, PartialEq)]
pub struct RecognitionTarget {
    /// Content hash of the drawing, its key in the tile service.
    pub drawing: Sha256Hex,
    /// Zero based page of the sheet in the drawing.
    pub page: usize,
    /// Unit of the sheet (D-20).
    pub unit: dimo_core::Unit,
    /// Project settings, for the tolerance engine.
    pub settings: dimo_core::ProjectSettings,
    /// Custom tolerance tables stored in the project as (id, file text) (M2 decision 4).
    pub custom_tables: Vec<(String, String)>,
}
