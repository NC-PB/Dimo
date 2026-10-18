//! The project that is open in the app: its session from `dimo-io`, lock, drawing, revision
//! and autosave state.

use std::path::Path;
use std::sync::Arc;

use dimo_core::Sha256Hex;
use dimo_io::lock::ProjectLock;
use dimo_io::project::ProjectSession;
use dimo_pdf::RasterSource;

use super::{Autosave, ProjectSnapshot, ProjectStatus};
use crate::ipc::{AutosaveReason, DocumentInfo};

/// A project that is open in the app.
#[derive(Debug)]
pub(super) struct OpenProject {
    pub(super) project: ProjectSession,
    /// Held while open; dropping it releases the project for other instances.
    #[allow(dead_code, reason = "held for its Drop")]
    pub(super) lock: Option<ProjectLock>,
    pub(super) drawing: DocumentInfo,
    pub(super) hash: Sha256Hex,
    /// The drawing source registered in the tile service under `hash`.
    pub(super) source: Arc<dyn RasterSource>,
    pub(super) revision: u32,
    /// Error of the last journal write, cleared by the next successful one.
    pub(super) journal_error: Option<(AutosaveReason, String)>,
    /// Autosave state last sent to the frontend.
    pub(super) reported: Autosave,
}

impl OpenProject {
    pub(super) fn autosave(&self) -> Autosave {
        if let Some((reason, message)) = &self.journal_error {
            return Autosave::Failed {
                reason: *reason,
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
                reason: AutosaveReason::NoFolder,
                message: "no autosave folder available".to_owned(),
            }
        }
    }

    pub(super) fn status(&mut self) -> ProjectStatus {
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

    pub(super) fn snapshot(&mut self) -> ProjectSnapshot {
        ProjectSnapshot {
            revision: self.revision,
            project: self.project.project().clone(),
            drawing: self.drawing.clone(),
            status: self.status(),
        }
    }

    /// Writes pending audit entries to the journal and remembers a failure (D-28).
    pub(super) fn flush(&mut self) {
        self.journal_error = match self.project.flush_journal() {
            Ok(_) => None,
            Err(error) => {
                // The session target, as before the split, so log searches keep working.
                tracing::warn!(target: "dimo_desktop::session", "autosave journal write failed: {error}");
                Some((AutosaveReason::of(&error), error.to_string()))
            }
        };
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.project.location().map(|(path, _)| path)
    }
}

/// True if both paths name the same existing file or folder.
pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
