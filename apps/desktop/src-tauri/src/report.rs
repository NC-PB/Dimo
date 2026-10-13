//! The Report tab (T3.19, ADR 0007): a live preview of the report that "Save copy" writes, the
//! checks before a report is issued, and Print.
//!
//! - **Preview.** [`report_preview`] renders the PDF of the chosen report through the same path
//!   as a normal report export ([`crate::export::render`]) from [`save_copy_snapshot`], the
//!   snapshot "Save copy" would take, and opens it in the tile service. The webview shows its
//!   pages through the existing `dimo://tile/...` route, like a drawing. Same project, options
//!   and day give the same bytes as the saved copy (FR-EXP-11, rule 11). The last few preview
//!   documents stay open, so a late answer of an older preview never closes the one on screen.
//! - **Checks.** [`report_checks`] lists what to look at before issuing: the warnings of M3
//!   decision 17 with counts, and the report header fields that are still empty after issuing
//!   would fill company and inspector (decision 15). Computed here, never in the webview
//!   (rule 2).
//! - **Print.** [`print_report`] writes the PDF that "Save copy" would write into a temporary
//!   folder and opens it with the system's PDF viewer, which prints it with its own dialog. It
//!   reuses the open after export path: no print API, no new capability (NFR-SEC-01). Like
//!   "Save copy" it stores today's date as the report date first when the project has none
//!   (T3.9), so the paper and later exports show the same date.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use dimo_core::{CharId, Project, ReportFormat, ReportInfo, ReportKind, measurement_state};
use dimo_pdf::ContentHash;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::export::{
    ExportFormat, ExportRequest, ExportStart, ReportWarnings, open_with_system, open_written_file,
    render, report_warnings, take_snapshot, write_file,
};
use crate::ipc::{CommandError, DocumentInfo, tile_service};
use crate::project::{emit_patched, with_session};
use crate::session::{AppSession, ExportSnapshot};
use crate::settings::{PdfBalloons, ReportLanguage};
use crate::tiles::TileState;

/// How many preview documents stay open in the tile service.
const OPEN_PREVIEWS: usize = 4;

/// The report the Report tab shows or prints, with the options of a normal export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ReportView {
    /// Check sheet or inspection report.
    pub kind: ReportKind,
    /// Language of the report (D-32).
    pub language: ReportLanguage,
    /// Blank result columns of a check sheet without samples, 1 to 10 (M3 decision 12).
    pub blank_columns: u8,
}

impl ReportView {
    /// The export request of "Save copy" as PDF for this report.
    pub const fn save_copy(self) -> ExportRequest {
        ExportRequest {
            format: ExportFormat::report_export(self.kind, ReportFormat::Pdf),
            language: self.language,
            pdf_balloons: PdfBalloons::PageContent,
            blank_columns: self.blank_columns,
            issued: false,
            open_after: false,
        }
    }
}

/// The snapshot "Save copy" would export, without changing the project: a project without a
/// report date gets today's date, as the export stores it first (T3.9). Used by the preview,
/// so it shows the file that "Save copy" writes today.
pub fn save_copy_snapshot(session: &AppSession) -> Result<ExportSnapshot, CommandError> {
    let mut snapshot = session.export_snapshot(false)?;
    let info = &mut snapshot.project.report_info;
    if info.report_date.trim().is_empty() {
        info.report_date = session.local_date();
    }
    Ok(snapshot)
}

/// The PDF bytes of the preview of `view`, from [`save_copy_snapshot`].
pub fn preview_bytes(snapshot: &ExportSnapshot, view: ReportView) -> Result<Vec<u8>, CommandError> {
    render(snapshot, &view.save_copy(), None)
}

/// Preview documents still open in the tile service, oldest first.
#[derive(Debug, Default)]
pub struct PreviewState {
    open: Mutex<VecDeque<ContentHash>>,
}

impl PreviewState {
    /// Notes `doc` as the newest preview. Returns the previews to close: the oldest beyond
    /// [`OPEN_PREVIEWS`].
    pub fn push(&self, doc: ContentHash) -> Vec<ContentHash> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        open.retain(|d| *d != doc);
        open.push_back(doc);
        let excess = open.len().saturating_sub(OPEN_PREVIEWS);
        open.drain(..excess).collect()
    }
}

/// Renders the PDF of report `view` of the open project, as "Save copy" would write it today,
/// and opens it for the `dimo://tile/...` route. Returns its content hash and page sizes; the
/// pages are the sheets of the returned document.
#[tauri::command]
#[specta::specta]
pub async fn report_preview(
    app: AppHandle,
    view: ReportView,
) -> Result<DocumentInfo, CommandError> {
    let (snapshot, stem) = with_session(app.clone(), |_, session| {
        Ok((save_copy_snapshot(session)?, session.file_stem()))
    })
    .await?;
    let tiles = tile_service(&app.state::<TileState>())?;
    let render_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = preview_bytes(&snapshot, view)?;
        let doc = tiles.open_document(bytes)?;
        let name = view.save_copy().format.file_name(&stem);
        let info = DocumentInfo::of(&doc, &name);
        for old in render_app.state::<PreviewState>().push(*doc.content_hash()) {
            tiles.close_document(&old);
        }
        Ok(info)
    })
    .await
    .map_err(|e| CommandError::Io {
        message: e.to_string(),
    })?
}

/// A report header field the Report tab asks for before issuing (T3.19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ReportField {
    /// Part number (project info).
    PartNumber,
    /// Report number.
    ReportNumber,
    /// Company name; filled from the settings when a report is issued.
    CompanyName,
    /// Inspected by; filled with the user name when a report is issued.
    InspectedBy,
}

/// What to look at before report `kind` is issued (T3.19, M3 decisions 15 to 17). Nothing here
/// blocks issuing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ReportChecks {
    /// The warnings confirmed before an issued export (decision 17).
    pub warnings: ReportWarnings,
    /// Parts (samples) of the project.
    pub samples: u32,
    /// Characteristics the report lists (measured ones, decision 5).
    pub rows: u32,
    /// Inspection report: cells without a result. Zero for the check sheet.
    pub missing_values: u32,
    /// Inspection report: values without limits, not evaluated. Zero for the check sheet.
    pub not_evaluated: u32,
    /// Listed characteristics whose limits come from a draft tolerance table (D-43).
    pub draft_rows: u32,
    /// Listed characteristics whose limits or result type changed after the last issued report
    /// (decision 16).
    pub changed_rows: u32,
    /// Header fields that would still be empty on the issued report, in header order.
    pub missing_fields: Vec<ReportField>,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The checks of report `kind` of `project`. `changed` lists the characteristics changed since
/// the last issued report, `filled` is the report info as issuing would print it.
pub fn checks_of(
    project: &Project,
    kind: ReportKind,
    changed: &[CharId],
    filled: &ReportInfo,
) -> Result<ReportChecks, CommandError> {
    let state = measurement_state(project);
    let warnings = report_warnings(project, kind, !changed.is_empty())?;
    let measured = |id: &CharId| state.rows.iter().any(|r| r.characteristic == *id);
    let inspection = kind == ReportKind::InspectionReport;
    let mut missing_fields = Vec::new();
    for (field, value) in [
        (ReportField::PartNumber, &project.info.part_number),
        (ReportField::ReportNumber, &filled.report_number),
        (ReportField::CompanyName, &filled.company_name),
        (ReportField::InspectedBy, &filled.inspected_by),
    ] {
        if value.trim().is_empty() {
            missing_fields.push(field);
        }
    }
    Ok(ReportChecks {
        warnings,
        samples: count(project.samples.len()),
        rows: state.to_inspect,
        missing_values: if inspection { state.counts.missing } else { 0 },
        not_evaluated: if inspection {
            state.counts.not_evaluated
        } else {
            0
        },
        draft_rows: count(state.rows.iter().filter(|r| r.draft).count()),
        changed_rows: count(changed.iter().filter(|id| measured(id)).count()),
        missing_fields,
    })
}

/// The checks of report `kind` of the project open in `session`.
pub fn checks(session: &AppSession, kind: ReportKind) -> Result<ReportChecks, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    let changed = session.changed_since_issued_report()?;
    let filled = session.issued_report_info(&project.report_info);
    checks_of(project, kind, &changed, &filled)
}

/// What to look at before report `kind` of the open project is issued: warnings with counts
/// and empty header fields. Never blocks issuing.
#[tauri::command]
#[specta::specta]
pub async fn report_checks(app: AppHandle, kind: ReportKind) -> Result<ReportChecks, CommandError> {
    with_session(app, move |_, session| checks(session, kind)).await
}

/// Where Print writes its file: a `dimo-print` folder in the temporary directory, or in debug
/// builds the folder of `DIMO_DEV_EXPORT_DIR`.
fn print_dir() -> PathBuf {
    crate::dev::dev_export_dir().unwrap_or_else(|| std::env::temp_dir().join("dimo-print"))
}

/// Takes the "Save copy" snapshot for printing `view`, storing today's report date first when it
/// is empty (T3.9). Returns where the PDF goes, `<stem>_<report>.pdf` in `dir`, and the snapshot
/// with the patch of the stored date, if any.
pub fn print_snapshot(
    session: &mut AppSession,
    view: ReportView,
    dir: &Path,
) -> Result<(PathBuf, ExportStart), CommandError> {
    let request = view.save_copy();
    let start = take_snapshot(session, request)?;
    let path = dir.join(request.format.file_name(&session.file_stem()));
    Ok((path, start))
}

/// Opens the PDF of report `view` of the open project with the system's PDF viewer for
/// printing: the file "Save copy" would write, in a temporary folder. Returns its file name.
/// Debug builds with `DIMO_DEV_EXPORT_DIR` write it there and do not open it.
#[tauri::command]
#[specta::specta]
pub async fn print_report(app: AppHandle, view: ReportView) -> Result<String, CommandError> {
    let dir = print_dir();
    let (path, start) = with_session(app.clone(), move |_, session| {
        print_snapshot(session, view, &dir)
    })
    .await?;
    if let Some(patched) = start.date {
        emit_patched(&app, patched);
    }
    let format = view.save_copy().format;
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = render(&start.snapshot, &view.save_copy(), None)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CommandError::Io {
                message: format!("{}: {e}", parent.display()),
            })?;
        }
        write_file(&path, &bytes)?;
        if crate::dev::dev_export_dir().is_none() {
            open_written_file(&path, format, open_with_system)?;
        }
        Ok(path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    })
    .await
    .map_err(|e| CommandError::Io {
        message: e.to_string(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_keep_the_newest_few_open() {
        let state = PreviewState::default();
        let docs: Vec<ContentHash> = (0u8..6).map(|i| ContentHash::of(&[i])).collect();
        for doc in &docs[..OPEN_PREVIEWS] {
            assert_eq!(state.push(*doc), Vec::<ContentHash>::new());
        }
        assert_eq!(state.push(docs[4]), [docs[0]]);
        // The same document again only becomes the newest.
        assert_eq!(state.push(docs[2]), Vec::<ContentHash>::new());
        assert_eq!(state.push(docs[5]), [docs[1]]);
        assert_eq!(state.push(ContentHash::of(b"x")), [docs[3]]);
    }

    #[test]
    fn save_copy_is_a_plain_pdf_export() {
        for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
            let view = ReportView {
                kind,
                language: ReportLanguage::De,
                blank_columns: 5,
            };
            let request = view.save_copy();
            assert_eq!(request.format.report(), Some((kind, ReportFormat::Pdf)));
            assert!(!request.issued && !request.open_after);
            assert_eq!(
                (request.language, request.blank_columns),
                (ReportLanguage::De, 5)
            );
        }
    }

    #[test]
    fn print_goes_to_a_temporary_folder() {
        if crate::dev::dev_export_dir().is_none() {
            assert_eq!(print_dir(), std::env::temp_dir().join("dimo-print"));
        }
    }
}
