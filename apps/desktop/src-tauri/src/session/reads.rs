//! What the shell reads from the open project, and the record of an issued report.

use std::path::{Path, PathBuf};

use dimo_core::{Project, ReportInfo, Sha256Hex};
use dimo_export::ExportFormat;
use dimo_io::project::ProjectSession;

use super::{AppSession, ExportSnapshot, ProjectPatched, RecognitionTarget, SnapshotState};
use crate::ipc::{CommandError, ProjectReason, RejectReason};

impl AppSession {
    /// What an export job needs: a copy of the project, its last change time and, if
    /// `with_drawing`, the drawing file of the current revision.
    pub fn export_snapshot(&self, with_drawing: bool) -> Result<ExportSnapshot, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        let drawing = if with_drawing {
            Some(
                open.project
                    .current_drawing()
                    .ok_or_else(|| CommandError::Project {
                        reason: ProjectReason::MissingDrawing,
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
            state: SnapshotState {
                session: self.session,
                revision: open.revision,
            },
        })
    }

    /// Records a report written from the snapshot taken at `state` as issued (D-23, M3
    /// decision 17): locks the numbering if it is not locked yet, stores `printed` (the report
    /// info the file shows, see `dimo_export::plan`) and adds the [`dimo_core::IssuedReport`],
    /// all as one undo step (`dimo_export::record_command`). Returns `None` and changes nothing
    /// if the project changed since the snapshot, because the file then no longer shows it.
    pub fn record_issued_report(
        &mut self,
        state: SnapshotState,
        printed: &ReportInfo,
        format: ExportFormat,
        output_sha256: Sha256Hex,
    ) -> Result<Option<ProjectPatched>, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        if self.session != state.session || open.revision != state.revision {
            return Ok(None);
        }
        let command =
            dimo_export::record_command(open.project.project(), printed, format, output_sha256)?;
        self.execute(command).map(Some)
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
            .ok_or_else(|| CommandError::Rejected {
                reason: RejectReason::UnknownSheet,
                message: format!("sheet {sheet} is not a sheet of the current drawing"),
            })?;
        Ok(RecognitionTarget {
            drawing: open.hash,
            page: usize::try_from(found.index).unwrap_or(usize::MAX),
            unit: found.unit,
            settings: project.settings.clone(),
            custom_tables: Self::custom_tables_of(&open.project),
        })
    }

    /// The custom tolerance tables the settings of `project` list, as (id, file text).
    fn custom_tables_of(project: &ProjectSession) -> Vec<(String, String)> {
        project
            .project()
            .settings
            .tolerance
            .custom_tables
            .iter()
            .filter_map(|t| {
                let bytes = project.table(&t.sha256)?;
                Some((
                    t.table.id.clone(),
                    String::from_utf8_lossy(bytes).into_owned(),
                ))
            })
            .collect()
    }

    /// The custom tolerance tables of the open project as (id, file text) (T2.8).
    pub fn custom_tables(&self) -> Result<Vec<(String, String)>, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        Ok(Self::custom_tables_of(&open.project))
    }

    /// Change history of one characteristic, oldest first (FR-CHR-10).
    pub fn characteristic_history(
        &self,
        id: dimo_core::CharId,
    ) -> Result<Vec<dimo_core::HistoryEntry>, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        Ok(open.project.characteristic_history(id))
    }

    /// Characteristics of the open project whose limits or result type changed after the last
    /// issued report (M3 decision 16).
    pub fn changed_since_issued_report(&self) -> Result<Vec<dimo_core::CharId>, CommandError> {
        let open = self.open.as_ref().ok_or(CommandError::NoProject)?;
        Ok(open.project.changed_since_issued_report())
    }

    /// Keeps a custom tolerance table file in the open project (M2 decision 4) and returns its
    /// hash. Validate it first; it is journaled with the first command whose settings list it.
    pub fn insert_table(&mut self, bytes: Vec<u8>) -> Result<Sha256Hex, CommandError> {
        let open = self.open.as_mut().ok_or(CommandError::NoProject)?;
        Ok(open.project.insert_table(bytes))
    }
}
