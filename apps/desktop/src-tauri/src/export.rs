//! Exports of the open project: ballooned PDF (FR-EXP-01, D-33), the characteristic list as CSV
//! or XLSX (FR-EXP-09, D-32), both T1.9, and the check sheet and inspection report as PDF or
//! XLSX (FR-EXP-04, T3.8).
//!
//! `export_project` asks for the target file in a Rust save dialog, takes an
//! [`ExportSnapshot`] of the project and starts a background job, then returns its [`JobId`]
//! at once. The job reports [`JobProgress`] and ends with one [`JobFinished`] event; the
//! window keeps working meanwhile. The output depends only on the snapshot and the request:
//! dates written into the files come from the project, never from the clock (FR-EXP-11).
//!
//! A job runs in steps: build the report model, render, write (NFR-PERF-06). [`cancel_job`]
//! stops it before the next step; Typst cannot be interrupted while it renders (T3.2), so a
//! cancel during rendering takes effect when rendering ends. A cancelled job writes nothing.
//!
//! "Export as issued" (D-23):
//!
//! - Ballooned PDF and characteristic list: the numbering is locked (`LockNumbering` with
//!   reason `issued_report`) in the same session step that takes the snapshot, so the issued
//!   numbers are exactly the exported ones. The lock stays when the export fails afterwards.
//! - Check sheet and inspection report (M3 decisions 15 and 17): the report is rendered with an
//!   empty company name and inspector filled from the settings and the audit user. After the
//!   file is written, one undoable step locks the numbering, stores the filled report info and
//!   adds the [`dimo_core::IssuedReport`] with the SHA-256 of the file. If the project changed
//!   while the job ran, nothing is recorded and the outcome says so.
//!
//! The file is written to a temporary file next to the target and renamed, so a failed or
//! cancelled export never leaves a half written file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use dimo_core::{
    Command, LockReason, Project, ReportFormat, ReportInfo, ReportKind, Sha256Hex, TemplateRef,
};
use dimo_io::export::{CsvListExporter, ExportOptions, Exporter, XlsxListExporter};
use dimo_pdf::{BalloonOutput, PdfDate, PdfEngine, project_overlay};
use dimo_report::{ReportModel, ReportOptions, TemplateSet};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_specta::Event;

use crate::ipc::{CommandError, JobId, JobProgress, tile_service};
use crate::project::{SessionState, emit, emit_patched, with_session};
use crate::session::{AppSession, ExportSnapshot, ProjectPatched};
use crate::settings::{PdfBalloons, ReportLanguage};
use crate::tiles::TileState;

/// The bundled report template set (M3 decision 10).
pub const REPORT_TEMPLATE: &str = "dimo-default";

/// What to export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// A copy of the drawing with the balloons (FR-EXP-01).
    BalloonedPdf,
    /// The characteristic list as CSV (FR-EXP-09).
    Csv,
    /// The characteristic list as an Excel workbook.
    Xlsx,
    /// The check sheet as PDF (FR-EXP-04).
    CheckSheetPdf,
    /// The check sheet as an Excel workbook.
    CheckSheetXlsx,
    /// The inspection report as PDF (FR-EXP-04).
    InspectionReportPdf,
    /// The inspection report as an Excel workbook.
    InspectionReportXlsx,
}

impl ExportFormat {
    /// The export of report `kind` in file `format`.
    pub const fn report_export(kind: ReportKind, format: ReportFormat) -> Self {
        match (kind, format) {
            (ReportKind::CheckSheet, ReportFormat::Pdf) => Self::CheckSheetPdf,
            (ReportKind::CheckSheet, ReportFormat::Xlsx) => Self::CheckSheetXlsx,
            (ReportKind::InspectionReport, ReportFormat::Pdf) => Self::InspectionReportPdf,
            (ReportKind::InspectionReport, ReportFormat::Xlsx) => Self::InspectionReportXlsx,
        }
    }

    /// Report kind and file format, `None` for the drawing and list exports.
    pub const fn report(self) -> Option<(ReportKind, ReportFormat)> {
        match self {
            Self::BalloonedPdf | Self::Csv | Self::Xlsx => None,
            Self::CheckSheetPdf => Some((ReportKind::CheckSheet, ReportFormat::Pdf)),
            Self::CheckSheetXlsx => Some((ReportKind::CheckSheet, ReportFormat::Xlsx)),
            Self::InspectionReportPdf => Some((ReportKind::InspectionReport, ReportFormat::Pdf)),
            Self::InspectionReportXlsx => Some((ReportKind::InspectionReport, ReportFormat::Xlsx)),
        }
    }

    /// File name extension without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::BalloonedPdf | Self::CheckSheetPdf | Self::InspectionReportPdf => "pdf",
            Self::Csv => "csv",
            Self::Xlsx | Self::CheckSheetXlsx | Self::InspectionReportXlsx => "xlsx",
        }
    }

    /// Suggested file name for a project whose files are named `stem`.
    pub fn file_name(self, stem: &str) -> String {
        let suffix = match self {
            Self::BalloonedPdf => "ballooned",
            Self::Csv | Self::Xlsx => "characteristics",
            Self::CheckSheetPdf | Self::CheckSheetXlsx => "check-sheet",
            Self::InspectionReportPdf | Self::InspectionReportXlsx => "inspection-report",
        };
        format!("{stem}_{suffix}.{}", self.extension())
    }

    fn filter_name(self) -> &'static str {
        match self.extension() {
            "pdf" => "PDF",
            "csv" => "CSV",
            _ => "Excel",
        }
    }
}

/// An export the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ExportRequest {
    /// What to export.
    pub format: ExportFormat,
    /// Language of the characteristic list headers and of the reports (D-32); ignored for the
    /// ballooned PDF.
    pub language: ReportLanguage,
    /// Balloons as page content or annotations (D-33); only for the ballooned PDF.
    pub pdf_balloons: PdfBalloons,
    /// Blank result columns of a check sheet for a project without samples, 1 to 10 (M3
    /// decision 12); ignored otherwise.
    pub blank_columns: u8,
    /// Export as issued (D-23): locks the numbering; a report is also recorded as issued.
    pub issued: bool,
}

/// What happened to the "issued" record of a report exported as issued (M3 decision 17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum IssuedRecord {
    /// Numbering locked and report recorded, as one undo step.
    Recorded,
    /// The project changed while the job ran, so the file no longer shows it: nothing was
    /// recorded. The file was written.
    ProjectChanged,
    /// The record was refused; the file was written.
    Refused {
        /// Why.
        error: CommandError,
    },
}

/// How a job ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JobOutcome {
    /// The file was written.
    Done {
        /// File name of the written file, without the directory.
        file_name: String,
        /// For a report exported as issued: whether it was recorded. `null` otherwise.
        issued: Option<IssuedRecord>,
    },
    /// Nothing was written.
    Failed {
        /// Why.
        error: CommandError,
    },
    /// Cancelled with `cancel_job`; nothing was written.
    Cancelled,
}

/// A background job ended. Event `job-finished`; sent once per job, after its last progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct JobFinished {
    /// The job.
    pub id: JobId,
    /// Result.
    pub outcome: JobOutcome,
}

/// Warnings before a report is exported as issued (M3 decision 17). They never block the
/// export; the user confirms them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ReportWarnings {
    /// Limits of at least one printed characteristic come from a draft table (D-43).
    pub draft_limits: bool,
    /// Inspection report: results are missing or not evaluated, or there are no samples, so
    /// the overall verdict is open. Always false for the check sheet.
    pub open_results: bool,
    /// Limits or result type of a characteristic changed after the last issued report (M3
    /// decision 16).
    pub limits_changed: bool,
}

impl ReportWarnings {
    /// True if any warning applies.
    pub const fn any(self) -> bool {
        self.draft_limits || self.open_results || self.limits_changed
    }
}

/// Cancel flags of the running export jobs.
#[derive(Debug, Default)]
pub struct JobRegistry {
    running: Mutex<HashMap<JobId, Arc<AtomicBool>>>,
}

impl JobRegistry {
    /// Registers job `id` and returns its cancel flag.
    pub fn start(&self, id: JobId) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.lock().insert(id, Arc::clone(&flag));
        flag
    }

    /// Asks job `id` to stop before its next step. False if no such job runs.
    pub fn cancel(&self, id: JobId) -> bool {
        self.lock().get(&id).is_some_and(|flag| {
            flag.store(true, Ordering::SeqCst);
            true
        })
    }

    /// Forgets job `id` when it ended.
    pub fn finish(&self, id: JobId) {
        self.lock().remove(&id);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<JobId, Arc<AtomicBool>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// IDs of background jobs within one app session.
static NEXT_JOB: AtomicU32 = AtomicU32::new(1);

/// A new job ID, also for recognition runs that answer directly (T2.6).
pub(crate) fn next_job_id() -> JobId {
    JobId(NEXT_JOB.fetch_add(1, Ordering::SeqCst))
}

fn export_error(message: impl std::fmt::Display) -> CommandError {
    CommandError::Export {
        message: message.to_string(),
    }
}

/// Why a job stopped before it wrote its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStop {
    /// [`cancel_job`] was called.
    Cancelled,
    /// A step failed.
    Failed(CommandError),
}

impl From<CommandError> for JobStop {
    fn from(error: CommandError) -> Self {
        Self::Failed(error)
    }
}

/// What the build step produced for the render step.
enum Prepared {
    /// Nothing to prepare: drawing and list exports.
    Direct,
    /// A report model and the template set to render it with.
    Report(Box<ReportModel>, TemplateSet),
}

/// The bundled report template set.
pub fn report_template() -> Result<TemplateSet, CommandError> {
    TemplateSet::bundled(REPORT_TEMPLATE).map_err(export_error)
}

/// Report options of `request` for `snapshot`: the date falls back to the project's last
/// change, the `modified` time of its manifest (M3 decision 14).
fn report_options(
    snapshot: &ExportSnapshot,
    request: ExportRequest,
    kind: ReportKind,
) -> ReportOptions {
    ReportOptions {
        blank_columns: request.blank_columns,
        fallback_date: Some(snapshot.modified.clone()),
        ..ReportOptions::new(kind, request.language.into())
    }
}

/// Step 1: builds the report model of a report export.
fn prepare(snapshot: &ExportSnapshot, request: ExportRequest) -> Result<Prepared, CommandError> {
    let Some((kind, _)) = request.format.report() else {
        return Ok(Prepared::Direct);
    };
    let model = ReportModel::build(&snapshot.project, &report_options(snapshot, request, kind))
        .map_err(export_error)?;
    Ok(Prepared::Report(Box::new(model), report_template()?))
}

/// Step 2: the bytes of the file.
fn render_prepared(
    prepared: &Prepared,
    snapshot: &ExportSnapshot,
    request: ExportRequest,
    engine: Option<&PdfEngine>,
) -> Result<Vec<u8>, CommandError> {
    if let Prepared::Report(model, template) = prepared {
        return match request.format.report() {
            Some((_, ReportFormat::Xlsx)) => dimo_report::render_xlsx(template, model),
            _ => dimo_report::render_report(template, model),
        }
        .map_err(export_error);
    }
    let options = ExportOptions {
        language: request.language.into(),
    };
    let mut bytes = Vec::new();
    match request.format {
        ExportFormat::Csv => CsvListExporter
            .export(&snapshot.project, &options, &mut bytes)
            .map_err(export_error)?,
        ExportFormat::Xlsx => XlsxListExporter
            .export(&snapshot.project, &options, &mut bytes)
            .map_err(export_error)?,
        ExportFormat::BalloonedPdf => {
            let engine = engine.ok_or_else(|| CommandError::PdfiumUnavailable {
                message: "no PDF engine".to_owned(),
            })?;
            let drawing = snapshot
                .drawing
                .clone()
                .ok_or_else(|| export_error("the drawing is missing"))?;
            let output = match request.pdf_balloons {
                PdfBalloons::PageContent => BalloonOutput::PageContent,
                PdfBalloons::Annotations => BalloonOutput::Annotations {
                    date: PdfDate::from_timestamp(&snapshot.modified).ok_or_else(|| {
                        export_error(format!("invalid project time {}", snapshot.modified))
                    })?,
                },
            };
            let overlay = project_overlay(&snapshot.project, output);
            bytes = engine
                .write_ballooned(drawing, overlay)
                .map_err(export_error)?;
        }
        ExportFormat::CheckSheetPdf
        | ExportFormat::CheckSheetXlsx
        | ExportFormat::InspectionReportPdf
        | ExportFormat::InspectionReportXlsx => {
            return Err(export_error("the report model was not built"));
        }
    }
    Ok(bytes)
}

/// The bytes of an export of `snapshot`. Pure apart from PDFium: same snapshot and request,
/// same bytes (FR-EXP-11).
pub fn render(
    snapshot: &ExportSnapshot,
    request: &ExportRequest,
    engine: Option<&PdfEngine>,
) -> Result<Vec<u8>, CommandError> {
    let prepared = prepare(snapshot, *request)?;
    render_prepared(&prepared, snapshot, *request, engine)
}

/// Runs the steps of an export job: build, render, write (NFR-PERF-06). Checks `cancel`
/// before each step and reports progress through `progress`. Returns the SHA-256 of the
/// written file; on [`JobStop`] no file was written.
pub fn run_steps(
    snapshot: &ExportSnapshot,
    request: &ExportRequest,
    engine: Option<&PdfEngine>,
    path: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<Sha256Hex, JobStop> {
    let mut step = |fraction: f64, message: &str| {
        if let Some(delay) = crate::dev::dev_export_delay() {
            std::thread::sleep(delay);
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(JobStop::Cancelled);
        }
        progress(fraction, message);
        Ok(())
    };
    step(0.0, "building")?;
    let prepared = prepare(snapshot, *request)?;
    step(0.2, "rendering")?;
    let bytes = render_prepared(&prepared, snapshot, *request, engine)?;
    step(0.8, "writing")?;
    write_file(path, &bytes)?;
    Ok(dimo_io::project::sha256(&bytes))
}

/// Writes `bytes` to `path` through a temporary file in the same directory.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    let name = path
        .file_name()
        .ok_or_else(|| export_error(format!("{} is not a file name", path.display())))?;
    let mut temporary_name = std::ffi::OsString::from(".");
    temporary_name.push(name);
    temporary_name.push(".part");
    let temporary = path.with_file_name(temporary_name);
    let result = std::fs::write(&temporary, bytes).and_then(|()| std::fs::rename(&temporary, path));
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(export_error(format!("{}: {error}", path.display())));
    }
    Ok(())
}

/// `path` with the extension of `format` if it has none.
fn with_extension(mut path: PathBuf, format: ExportFormat) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(format.extension());
    }
    path
}

/// The warnings of [`ReportWarnings`] for report `kind` of `project`; `limits_changed` from
/// the audit log (M3 decision 16).
pub fn report_warnings(
    project: &Project,
    kind: ReportKind,
    limits_changed: bool,
) -> Result<ReportWarnings, CommandError> {
    let model = ReportModel::build(
        project,
        &ReportOptions::new(kind, dimo_report::Language::En),
    )
    .map_err(export_error)?;
    Ok(ReportWarnings {
        draft_limits: model.flags.draft_limits,
        open_results: model.flags.open_results,
        limits_changed,
    })
}

/// The warnings to confirm before report `kind` of the open project is exported as issued
/// (M3 decision 17). They never block the export.
#[tauri::command]
#[specta::specta]
pub async fn report_export_warnings(
    app: AppHandle,
    kind: ReportKind,
) -> Result<ReportWarnings, CommandError> {
    with_session(app, move |_, session| {
        let project = session.project().ok_or(CommandError::NoProject)?;
        let changed = !session.changed_since_issued_report()?.is_empty();
        report_warnings(project, kind, changed)
    })
    .await
}

/// Asks the export job `id` to stop before its next step. Returns false if no such job runs
/// (it may just have ended). A job that is already writing its file finishes it and ends
/// with its normal outcome.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub fn cancel_job(jobs: State<'_, JobRegistry>, id: JobId) -> bool {
    jobs.cancel(id)
}

/// The report info printed on a report exported as issued, `None` for other exports.
type IssuedInfo = Option<ReportInfo>;

/// What an export starts from.
#[derive(Debug)]
pub struct ExportStart {
    /// The copy the job renders.
    pub snapshot: ExportSnapshot,
    /// For a report exported as issued: the report info it prints, recorded after the write.
    pub issued: IssuedInfo,
    /// For an issued drawing or list export: the patch of the numbering lock, to emit.
    pub lock: Option<ProjectPatched>,
}

/// Takes the snapshot of an export. For an issued drawing or list export the numbering is
/// locked first (T1.9); a report is not changed here, its snapshot gets the filled report info
/// and the lock follows after the write (M3 decisions 15 and 17).
pub fn take_snapshot(
    session: &mut AppSession,
    request: ExportRequest,
) -> Result<ExportStart, CommandError> {
    let report = request.format.report();
    let lock = if request.issued && report.is_none() {
        Some(session.execute(Command::LockNumbering {
            reason: LockReason::IssuedReport,
        })?)
    } else {
        None
    };
    let mut snapshot = session.export_snapshot(request.format == ExportFormat::BalloonedPdf)?;
    let issued = (request.issued && report.is_some()).then(|| {
        let filled = session.issued_report_info(&snapshot.project.report_info);
        snapshot.project.report_info = filled.clone();
        filled
    });
    Ok(ExportStart {
        snapshot,
        issued,
        lock,
    })
}

/// Asks for the target file and starts the export as a background job. Returns `null` if the
/// dialog was cancelled, else the job ID; progress and result follow as `job-progress` and
/// `job-finished` events.
#[tauri::command]
#[specta::specta]
pub async fn export_project(
    app: AppHandle,
    window: WebviewWindow,
    request: ExportRequest,
) -> Result<Option<JobId>, CommandError> {
    let format = request.format;
    let stem = with_session(app.clone(), |_, session| {
        if session.is_open() {
            Ok(session.file_stem())
        } else {
            Err(CommandError::NoProject)
        }
    })
    .await?;
    let suggested = format.file_name(&stem);
    let engine = match format {
        ExportFormat::BalloonedPdf => {
            Some(tile_service(&app.state::<TileState>())?.engine().clone())
        }
        _ => None,
    };
    let path = if let Some(dir) = crate::dev::dev_export_dir() {
        let _ = std::fs::create_dir_all(&dir);
        dir.join(&suggested)
    } else {
        let picked = rfd::AsyncFileDialog::new()
            .add_filter(format.filter_name(), &[format.extension()])
            .set_file_name(&suggested)
            .set_parent(&window)
            .save_file()
            .await;
        let Some(file) = picked else {
            return Ok(None);
        };
        file.path().to_path_buf()
    };
    let path = with_extension(path, format);
    let ExportStart {
        snapshot,
        issued,
        lock,
    } = with_session(app.clone(), move |_, session| {
        take_snapshot(session, request)
    })
    .await?;
    if let Some(patched) = lock {
        emit_patched(&app, patched);
    }
    let id = next_job_id();
    let cancel = app.state::<JobRegistry>().start(id);
    let job_app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("dimo-export".into())
        .spawn(move || {
            let job = Job {
                app: &job_app,
                id,
                snapshot: &snapshot,
                request,
                engine: engine.as_ref(),
                path: &path,
            };
            job.run(&cancel, issued);
            job_app.state::<JobRegistry>().finish(id);
        });
    if let Err(e) = spawned {
        app.state::<JobRegistry>().finish(id);
        return Err(CommandError::Io {
            message: e.to_string(),
        });
    }
    Ok(Some(id))
}

/// One export job on its own thread.
struct Job<'a> {
    app: &'a AppHandle,
    id: JobId,
    snapshot: &'a ExportSnapshot,
    request: ExportRequest,
    engine: Option<&'a PdfEngine>,
    path: &'a Path,
}

impl Job<'_> {
    fn run(&self, cancel: &AtomicBool, issued: IssuedInfo) {
        let mut progress = |fraction: f64, message: &str| {
            emit(
                self.app,
                &JobProgress {
                    id: self.id,
                    fraction,
                    message: message.to_owned(),
                },
            );
        };
        let result = run_steps(
            self.snapshot,
            &self.request,
            self.engine,
            self.path,
            cancel,
            &mut progress,
        );
        let outcome = match result {
            Ok(sha256) => {
                let issued = issued.map(|info| self.record(info, sha256));
                progress(1.0, "done");
                JobOutcome::Done {
                    file_name: self
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    issued,
                }
            }
            Err(JobStop::Cancelled) => JobOutcome::Cancelled,
            Err(JobStop::Failed(error)) => {
                tracing::warn!("export to {} failed: {error}", self.path.display());
                JobOutcome::Failed { error }
            }
        };
        emit(
            self.app,
            &JobFinished {
                id: self.id,
                outcome,
            },
        );
    }

    /// Records the written report as issued (M3 decision 17).
    fn record(&self, info: ReportInfo, output_sha256: Sha256Hex) -> IssuedRecord {
        let Some((kind, format)) = self.request.format.report() else {
            return IssuedRecord::Refused {
                error: export_error("not a report"),
            };
        };
        let template = match report_template() {
            Ok(set) => TemplateRef {
                id: set.id().to_owned(),
                version: set.version(),
            },
            Err(error) => return IssuedRecord::Refused { error },
        };
        let command = Command::RecordIssuedReport {
            kind,
            template,
            format,
            output_sha256,
        };
        let state = self.app.state::<SessionState>();
        let mut session = state.lock();
        match session.record_issued_report(self.snapshot.state, info, command) {
            Ok(Some(patched)) => {
                emit_patched(self.app, patched);
                IssuedRecord::Recorded
            }
            Ok(None) => IssuedRecord::ProjectChanged,
            Err(error) => {
                tracing::warn!("cannot record the issued report: {error}");
                IssuedRecord::Refused { error }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_follow_the_project() {
        assert_eq!(
            ExportFormat::BalloonedPdf.file_name("part"),
            "part_ballooned.pdf"
        );
        assert_eq!(
            ExportFormat::Csv.file_name("part"),
            "part_characteristics.csv"
        );
        assert_eq!(
            ExportFormat::Xlsx.file_name("part"),
            "part_characteristics.xlsx"
        );
        assert_eq!(
            ExportFormat::CheckSheetPdf.file_name("part"),
            "part_check-sheet.pdf"
        );
        assert_eq!(
            ExportFormat::CheckSheetXlsx.file_name("part"),
            "part_check-sheet.xlsx"
        );
        assert_eq!(
            ExportFormat::InspectionReportPdf.file_name("part"),
            "part_inspection-report.pdf"
        );
        assert_eq!(
            ExportFormat::InspectionReportXlsx.file_name("part"),
            "part_inspection-report.xlsx"
        );
        assert_eq!(
            with_extension(PathBuf::from("/tmp/x"), ExportFormat::Csv),
            PathBuf::from("/tmp/x.csv")
        );
        assert_eq!(
            with_extension(PathBuf::from("/tmp/x.txt"), ExportFormat::Csv),
            PathBuf::from("/tmp/x.txt")
        );
    }

    #[test]
    fn report_formats_map_both_ways() {
        for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
            for format in [ReportFormat::Pdf, ReportFormat::Xlsx] {
                let export = ExportFormat::report_export(kind, format);
                assert_eq!(export.report(), Some((kind, format)));
            }
        }
        assert_eq!(ExportFormat::Csv.report(), None);
        assert_eq!(ExportFormat::InspectionReportXlsx.filter_name(), "Excel");
        assert_eq!(ExportFormat::CheckSheetPdf.filter_name(), "PDF");
    }

    #[test]
    fn files_are_written_whole_or_not_at_all() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("list.csv");
        write_file(&path, b"a,b\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"a,b\n");
        let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(names.len(), 1, "no temporary file is left");
        let missing = dir.path().join("missing").join("list.csv");
        assert!(matches!(
            write_file(&missing, b"x"),
            Err(CommandError::Export { .. })
        ));
        assert!(!missing.exists());
    }

    #[test]
    fn cancel_flags_follow_the_jobs() {
        let jobs = JobRegistry::default();
        let flag = jobs.start(JobId(4));
        assert!(!jobs.cancel(JobId(5)), "unknown job");
        assert!(jobs.cancel(JobId(4)));
        assert!(flag.load(Ordering::SeqCst));
        jobs.finish(JobId(4));
        assert!(!jobs.cancel(JobId(4)), "ended job");
    }
}
