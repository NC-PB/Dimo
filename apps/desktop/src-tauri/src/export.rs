//! Exports of the open project (T1.9): ballooned PDF (FR-EXP-01, D-33) and the characteristic
//! list as CSV or XLSX (FR-EXP-09, D-32).
//!
//! `export_project` asks for the target file in a Rust save dialog, takes an
//! [`ExportSnapshot`] of the project and starts a background job, then returns its [`JobId`]
//! at once. The job reports [`JobProgress`] and ends with one [`JobFinished`] event; the
//! window keeps working meanwhile. The output depends only on the snapshot and the request:
//! dates written into the PDF are the project's last change time, never the clock (FR-EXP-11).
//!
//! "Export as issued" locks the numbering (D-23, `LockNumbering` with reason `issued_report`)
//! in the same session step that takes the snapshot, so the issued numbers are exactly the
//! exported ones. The lock is an ordinary undoable command and part of the audit log; it stays
//! when the export fails afterwards.
//!
//! The file is written to a temporary file next to the target and renamed, so a failed export
//! never leaves a half written file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use dimo_core::{Command, LockReason};
use dimo_io::export::{CsvListExporter, ExportOptions, Exporter, XlsxListExporter};
use dimo_pdf::{BalloonOutput, PdfDate, PdfEngine, project_overlay};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_specta::Event;

use crate::ipc::{CommandError, JobId, JobProgress, tile_service};
use crate::project::{emit, emit_patched, with_session};
use crate::session::ExportSnapshot;
use crate::settings::{PdfBalloons, ReportLanguage};
use crate::tiles::TileState;

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
}

impl ExportFormat {
    /// File name extension without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::BalloonedPdf => "pdf",
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
        }
    }

    /// Suggested file name for a project whose files are named `stem`.
    pub fn file_name(self, stem: &str) -> String {
        let suffix = match self {
            Self::BalloonedPdf => "ballooned",
            Self::Csv | Self::Xlsx => "characteristics",
        };
        format!("{stem}_{suffix}.{}", self.extension())
    }

    fn filter_name(self) -> &'static str {
        match self {
            Self::BalloonedPdf => "PDF",
            Self::Csv => "CSV",
            Self::Xlsx => "Excel",
        }
    }
}

/// An export the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ExportRequest {
    /// What to export.
    pub format: ExportFormat,
    /// Header language of the characteristic list (D-32); ignored for the PDF.
    pub language: ReportLanguage,
    /// Balloons as page content or annotations (D-33); only for the PDF.
    pub pdf_balloons: PdfBalloons,
    /// Export as issued: locks the numbering first (D-23).
    pub issued: bool,
}

/// How a job ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JobOutcome {
    /// The file was written.
    Done {
        /// File name of the written file, without the directory.
        file_name: String,
    },
    /// Nothing was written.
    Failed {
        /// Why.
        error: CommandError,
    },
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

/// IDs of background jobs within one app session.
static NEXT_JOB: AtomicU32 = AtomicU32::new(1);

fn export_error(message: impl std::fmt::Display) -> CommandError {
    CommandError::Export {
        message: message.to_string(),
    }
}

/// The bytes of an export of `snapshot`. Pure apart from PDFium: same snapshot and request,
/// same bytes (FR-EXP-11).
pub fn render(
    snapshot: &ExportSnapshot,
    request: &ExportRequest,
    engine: Option<&PdfEngine>,
) -> Result<Vec<u8>, CommandError> {
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
    }
    Ok(bytes)
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
        ExportFormat::Csv | ExportFormat::Xlsx => None,
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
    let snapshot = with_session(app.clone(), move |app, session| {
        if request.issued {
            let patched = session.execute(Command::LockNumbering {
                reason: LockReason::IssuedReport,
            })?;
            emit_patched(app, patched);
        }
        session.export_snapshot(format == ExportFormat::BalloonedPdf)
    })
    .await?;
    let id = JobId(NEXT_JOB.fetch_add(1, Ordering::SeqCst));
    let job_app = app.clone();
    std::thread::Builder::new()
        .name("dimo-export".into())
        .spawn(move || run_job(&job_app, id, &snapshot, request, engine.as_ref(), &path))
        .map_err(|e| CommandError::Io {
            message: e.to_string(),
        })?;
    Ok(Some(id))
}

fn progress(app: &AppHandle, id: JobId, fraction: f64, message: &str) {
    emit(
        app,
        &JobProgress {
            id,
            fraction,
            message: message.to_owned(),
        },
    );
}

fn run_job(
    app: &AppHandle,
    id: JobId,
    snapshot: &ExportSnapshot,
    request: ExportRequest,
    engine: Option<&PdfEngine>,
    path: &Path,
) {
    progress(app, id, 0.0, "rendering");
    let result = render(snapshot, &request, engine).and_then(|bytes| {
        progress(app, id, 0.8, "writing");
        write_file(path, &bytes)
    });
    let outcome = match result {
        Ok(()) => {
            progress(app, id, 1.0, "done");
            JobOutcome::Done {
                file_name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            }
        }
        Err(error) => {
            tracing::warn!("export to {} failed: {error}", path.display());
            JobOutcome::Failed { error }
        }
    };
    emit(app, &JobFinished { id, outcome });
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
            with_extension(PathBuf::from("/tmp/x"), ExportFormat::Csv),
            PathBuf::from("/tmp/x.csv")
        );
        assert_eq!(
            with_extension(PathBuf::from("/tmp/x.txt"), ExportFormat::Csv),
            PathBuf::from("/tmp/x.txt")
        );
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
}
