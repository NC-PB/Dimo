//! Exports of the open project: ballooned PDF (FR-EXP-01, D-33), the characteristic list as CSV
//! or XLSX (FR-EXP-09, D-32), both T1.9, and the check sheet and inspection report as PDF or
//! XLSX (FR-EXP-04, T3.8).
//!
//! `export_project` asks for the target file in a Rust save dialog, takes an
//! [`ExportSnapshot`](crate::session::ExportSnapshot) of the project and starts a background
//! job, then returns its [`JobId`] at once. The job reports progress and ends with one
//! [`JobFinished`] event; the window keeps working meanwhile. The output depends only on the
//! snapshot and the request: dates written into the files come from the project, never from the
//! clock (FR-EXP-11).
//!
//! A job ([`crate::job`]) runs in steps: build the report model, render, write
//! (NFR-PERF-06). [`crate::job::cancel_job`] stops it before the next step; Typst cannot be
//! interrupted while it renders (T3.2), so a cancel during rendering takes effect when rendering
//! ends. A cancelled job writes nothing.
//!
//! Formats, rendering and the rules of "export as issued" and of the report date live in the
//! crates (MF F2): `dimo_export` (registry, [`dimo_export::render`], [`dimo_export::plan`]) and
//! `dimo_report::issue`. The shell keeps:
//!
//! - [`request`]: the IPC request, its dialog filter and the outcome events;
//! - [`pipeline`]: the snapshot following the plan, the job steps and the file write;
//! - this module: the Tauri commands and the job thread. A report exported as issued is
//!   recorded after the write; if the project changed while the job ran, nothing is recorded and
//!   the outcome says so (M3 decision 17).

mod pipeline;
mod request;

use std::path::Path;
use std::sync::atomic::AtomicBool;

use dimo_core::{ReportInfo, ReportKind, Sha256Hex};
use dimo_pdf::PdfEngine;
use dimo_report::issue::ReportWarnings;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::dialogs::{self, DevOverride};
use crate::ipc::{CommandError, ExportReason, JobId, tile_service};
use crate::job;
pub(crate) use crate::job::next_job_id;
pub use crate::job::{JobRegistry, JobStop};
pub(crate) use crate::opener::{open_with_system, open_written_file};
use crate::project::{SessionState, emit, emit_patched, with_session};
use crate::session::ExportSnapshot;
use crate::tiles::TileState;
pub use dimo_export::ExportFormat;
pub use pipeline::{ExportStart, render, run_steps, take_snapshot, write_file};
pub use request::{ExportRequest, IssuedRecord, JobFinished, JobOutcome, filter};

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
        Ok(dimo_report::issue::report_warnings(project, kind, changed)?)
    })
    .await
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
    let stem = with_session(app.clone(), move |_, session| {
        let project = session.project().ok_or(CommandError::NoProject)?;
        // Refused before the save dialog, so nobody picks a file for nothing (R.19).
        if format == ExportFormat::BalloonedPdf && dimo_export::is_image_drawing(project) {
            return Err(dimo_export::ExportError::ImageDrawing.into());
        }
        Ok(session.file_stem())
    })
    .await?;
    let suggested = format.file_name(&stem);
    let engine = if format.needs_drawing() {
        let engine = tile_service(&app.state::<TileState>())
            .ok()
            .and_then(|tiles| tiles.engine().cloned());
        Some(engine.ok_or_else(|| {
            CommandError::export(ExportReason::NoPdfEngine, "PDFium is not loaded")
        })?)
    } else {
        None
    };
    let Some(path) =
        dialogs::save_file(&window, filter(format), &suggested, DevOverride::ExportDir).await
    else {
        return Ok(None);
    };
    let ExportStart {
        snapshot,
        issued,
        lock,
        date,
    } = with_session(app.clone(), move |_, session| {
        take_snapshot(session, request)
    })
    .await?;
    for patched in [lock, date].into_iter().flatten() {
        emit_patched(&app, patched);
    }
    let job_app = app.clone();
    let id = job::spawn(&app, "dimo-export", move |id, cancel| {
        let job = Job {
            app: &job_app,
            id,
            snapshot: &snapshot,
            request,
            engine: engine.as_ref(),
            path: &path,
        };
        job.run(cancel, issued);
    })?;
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
    fn run(&self, cancel: &AtomicBool, issued: Option<ReportInfo>) {
        let mut progress =
            |fraction: f64, message: &str| job::progress(self.app, self.id, fraction, message);
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
                let issued = issued.map(|info| self.record(&info, sha256));
                let open_error = self
                    .request
                    .open_after
                    .then(|| {
                        open_written_file(
                            self.path,
                            self.request.format.extension(),
                            open_with_system,
                        )
                    })
                    .and_then(Result::err);
                progress(1.0, "done");
                JobOutcome::Done {
                    file_name: self
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    issued,
                    open_error,
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
    fn record(&self, info: &ReportInfo, output_sha256: Sha256Hex) -> IssuedRecord {
        // Loads the template reference before the session lock; the record reuses it.
        if let Err(error) = dimo_export::bundled_template_ref() {
            return IssuedRecord::Refused {
                error: error.into(),
            };
        }
        let state = self.app.state::<SessionState>();
        let mut session = state.lock();
        match session.record_issued_report(
            self.snapshot.state,
            info,
            self.request.format,
            output_sha256,
        ) {
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
