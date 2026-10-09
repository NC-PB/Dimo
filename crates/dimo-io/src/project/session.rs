//! An open project: the document, its file, and the autosave journal (NFR-REL-01, D-28).
//!
//! The desktop shell keeps one [`ProjectSession`] per window and
//!
//! 1. executes commands on [`ProjectSession::document_mut`],
//! 2. calls [`ProjectSession::flush_journal`] after each command batch and every
//!    [`AUTOSAVE_INTERVAL`](crate::journal::AUTOSAVE_INTERVAL),
//! 3. calls [`ProjectSession::save`] or [`ProjectSession::save_as`] on request, which writes the
//!    project file and deletes the journal (compaction),
//! 4. calls [`ProjectSession::discard_journal`] when the user closes without saving.
//!
//! If the process dies in between, the journal survives and [`ProjectSession::open`] replays it.
//! A project that was never saved has no journal: it has no file to continue.

use std::path::{Path, PathBuf};

use dimo_core::{Document, Project, ProjectInfo, Sha256Hex};

use super::error::ProjectError;
use super::import::ImportedDrawing;
use super::{Layout, ProjectFile};
use crate::journal::{self, Journal, Recovery};

/// What happened while opening a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpenReport {
    /// Schema version of the file if it was older and migrated (NFR-REL-03).
    pub migrated_from: Option<u32>,
    /// Number of journal entries replayed after a crash (NFR-REL-01).
    pub recovered_entries: usize,
    /// True if the last journal line was incomplete and dropped.
    pub dropped_incomplete_line: bool,
    /// A journal that did not fit the project: why, and where it was moved.
    pub discarded_journal: Option<(String, PathBuf)>,
}

/// An open project with undo history, audit log and autosave journal.
#[derive(Debug)]
pub struct ProjectSession {
    document: Document,
    /// Created time, audit log and drawings. `file.project` is the last saved state.
    file: ProjectFile,
    location: Option<(PathBuf, Layout)>,
    journal: Option<Journal>,
    /// The project differs from the file because the journal was replayed.
    recovered: bool,
}

impl ProjectSession {
    /// A new, unsaved project for one imported drawing. The project counts as created at the
    /// drawing's import time.
    pub fn create(info: ProjectInfo, drawing: ImportedDrawing) -> Self {
        let created = drawing.revision.imported_at.clone();
        let mut file = ProjectFile::new(Project::new(info, drawing.revision), created);
        file.insert_drawing(drawing.bytes);
        Self::from_file(file, None)
    }

    /// Wraps project content that has no file yet (or whose file is managed elsewhere).
    pub fn from_file(file: ProjectFile, location: Option<(PathBuf, Layout)>) -> Self {
        Self {
            document: Document::new(file.project.clone()),
            file,
            location,
            journal: None,
            recovered: false,
        }
    }

    /// Opens a project file or folder and replays its journal if the last session crashed.
    pub fn open(path: &Path) -> Result<(Self, OpenReport), ProjectError> {
        let loaded = ProjectFile::load(path)?;
        let mut report = OpenReport {
            migrated_from: loaded.migrated_from(),
            ..OpenReport::default()
        };
        let mut file = loaded.file;
        let mut current = file.project.clone();
        let mut journal = None;
        match journal::recover(&journal::journal_path(path), &mut current)? {
            Recovery::Nothing => {}
            Recovery::Replayed {
                entries,
                dropped_incomplete_line,
                journal: open,
            } => {
                report.recovered_entries = entries.len();
                report.dropped_incomplete_line = dropped_incomplete_line;
                file.audit.extend(entries);
                journal = Some(open);
            }
            Recovery::Discarded { reason, moved_to } => {
                report.discarded_journal = Some((reason, moved_to));
            }
        }
        let recovered = journal.is_some();
        let session = Self {
            document: Document::new(current),
            file,
            location: Some((path.to_owned(), Layout::of(path))),
            journal,
            recovered,
        };
        Ok((session, report))
    }

    /// The document: project state and undo history.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The document, for executing commands, undo and redo. Call
    /// [`ProjectSession::flush_journal`] after each batch.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The current project state.
    pub fn project(&self) -> &Project {
        self.document.project()
    }

    /// The drawing file with this hash, for opening it in the PDF engine.
    pub fn drawing(&self, hash: &Sha256Hex) -> Option<&[u8]> {
        self.file.drawing(hash)
    }

    /// The saved audit log plus everything flushed since.
    pub fn audit(&self) -> &[dimo_core::AuditEntry] {
        &self.file.audit
    }

    /// The project file and its layout, `None` before the first save.
    pub fn location(&self) -> Option<(&Path, Layout)> {
        self.location
            .as_ref()
            .map(|(path, layout)| (path.as_path(), *layout))
    }

    /// The journal file, if one is open.
    pub fn journal_path(&self) -> Option<&Path> {
        self.journal.as_ref().map(Journal::path)
    }

    /// True if there are changes that are not in the project file.
    pub fn is_modified(&self) -> bool {
        self.recovered || self.document.is_modified()
    }

    /// Moves new audit entries of the document into the audit log and appends them to the
    /// journal (D-28). Returns the number of entries written.
    ///
    /// The journal is created on the first entry after a save, so a project without changes
    /// leaves no file. Unsaved projects have no journal; their entries only go to the audit
    /// log. On a write error the entries stay pending and the next call retries.
    pub fn flush_journal(&mut self) -> Result<usize, ProjectError> {
        let pending = self.document.audit().len();
        if pending == 0 {
            return Ok(0);
        }
        if let Some((path, _)) = &self.location {
            if self.journal.is_none() {
                self.journal = Some(Journal::create(
                    &journal::journal_path(path),
                    &self.file.project,
                )?);
            }
            if let Some(journal) = &mut self.journal {
                journal.append(self.document.audit())?;
            }
        }
        self.file.audit.extend(self.document.take_audit());
        Ok(pending)
    }

    /// Saves to the current file, then deletes the journal (compaction).
    pub fn save(&mut self) -> Result<(), ProjectError> {
        let (path, layout) = self.location.clone().ok_or(ProjectError::NoPath)?;
        self.write(&path, layout)
    }

    /// Saves to a new file, which becomes the current file. The journal of the old file is
    /// deleted after the save, since its changes are now in the new file.
    pub fn save_as(&mut self, path: &Path, layout: Layout) -> Result<(), ProjectError> {
        self.write(path, layout)?;
        self.location = Some((path.to_owned(), layout));
        Ok(())
    }

    fn write(&mut self, path: &Path, layout: Layout) -> Result<(), ProjectError> {
        // The save is what makes the changes safe, so a failing journal must not stop it.
        // Entries the journal could not take stay pending in the document.
        let _ = self.flush_journal();
        let saved_len = self.file.audit.len();
        self.file.audit.extend_from_slice(self.document.audit());
        let saved_project =
            std::mem::replace(&mut self.file.project, self.document.project().clone());
        if let Err(error) = self.file.save(path, layout) {
            self.file.project = saved_project;
            self.file.audit.truncate(saved_len);
            return Err(error);
        }
        self.document.take_audit();
        self.document.mark_saved();
        self.recovered = false;
        self.discard_journal()
    }

    /// Deletes the journal, for closing without saving. The unsaved changes are then lost.
    pub fn discard_journal(&mut self) -> Result<(), ProjectError> {
        if let Some(journal) = self.journal.take() {
            journal.remove()?;
        }
        Ok(())
    }
}
