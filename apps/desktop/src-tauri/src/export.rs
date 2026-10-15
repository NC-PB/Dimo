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
//! Formats, rendering and the rules of "export as issued" and of the report date live in the
//! crates (MF F2): `dimo_export` (registry, [`dimo_export::render`], [`dimo_export::plan`]) and
//! `dimo_report::issue`. This module executes the plan on the session, runs the job and writes
//! the file. A report exported as issued is recorded after the write; if the project changed
//! while the job ran, nothing is recorded and the outcome says so (M3 decision 17).
//!
//! The file is written to a temporary file next to the target and renamed, so a failed or
//! cancelled export never leaves a half written file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use dimo_core::{ReportInfo, ReportKind, Sha256Hex};
use dimo_export::{BalloonMode, RenderOptions};
use dimo_pdf::PdfEngine;
use dimo_report::issue::ReportWarnings;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_specta::Event;

use crate::ipc::{CommandError, ExportReason, JobId, JobProgress, tile_service};
use crate::project::{SessionState, emit, emit_patched, with_session};
use crate::session::{AppSession, ExportSnapshot, ProjectPatched};
use crate::settings::PdfBalloons;
use crate::tiles::TileState;
use dimo_core::Language;

pub use dimo_export::ExportFormat;

/// An export the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ExportRequest {
    /// What to export.
    pub format: ExportFormat,
    /// Language of the characteristic list headers and of the reports (D-32); ignored for the
    /// ballooned PDF.
    pub language: Language,
    /// Balloons as page content or annotations (D-33); only for the ballooned PDF.
    pub pdf_balloons: PdfBalloons,
    /// Blank result columns of a check sheet for a project without samples, 1 to 10 (M3
    /// decision 12); ignored otherwise.
    pub blank_columns: u8,
    /// Export as issued (D-23): locks the numbering; a report is also recorded as issued.
    pub issued: bool,
    /// After a successful job, open the written file with the system default application.
    pub open_after: bool,
}

impl ExportRequest {
    /// The render options of this request.
    pub const fn render_options(&self) -> RenderOptions {
        RenderOptions {
            format: self.format,
            language: self.language,
            balloons: match self.pdf_balloons {
                PdfBalloons::PageContent => BalloonMode::PageContent,
                PdfBalloons::Annotations => BalloonMode::Annotations,
            },
            blank_columns: self.blank_columns,
        }
    }
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
        /// If the request asked to open the file and that failed: why. The file was written.
        /// `null` if the file was opened or not asked for.
        open_error: Option<CommandError>,
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

/// The bytes of an export of `snapshot`, through [`dimo_export::render`] like the CLI. Pure
/// apart from PDFium: same snapshot and request, same bytes (FR-EXP-11).
pub fn render(
    snapshot: &ExportSnapshot,
    request: &ExportRequest,
    engine: Option<&PdfEngine>,
) -> Result<Vec<u8>, CommandError> {
    Ok(dimo_export::render(
        &snapshot.source(),
        request.render_options(),
        engine,
    )?)
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
    let (source, options) = (snapshot.source(), request.render_options());
    step(0.0, "building")?;
    let prepared = dimo_export::prepare(&source, options).map_err(CommandError::from)?;
    step(0.2, "rendering")?;
    let bytes = prepared
        .render(&source, options, engine)
        .map_err(CommandError::from)?;
    step(0.8, "writing")?;
    write_file(path, &bytes)?;
    Ok(dimo_io::project::sha256(&bytes))
}

/// Writes `bytes` to `path` through a temporary file in the same directory.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    let name = path.file_name().ok_or_else(|| {
        CommandError::export(
            ExportReason::InvalidFileName,
            format_args!("{} is not a file name", path.display()),
        )
    })?;
    let mut temporary_name = std::ffi::OsString::from(".");
    temporary_name.push(name);
    temporary_name.push(".part");
    let temporary = path.with_file_name(temporary_name);
    let result = std::fs::write(&temporary, bytes).and_then(|()| std::fs::rename(&temporary, path));
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(CommandError::export_io(path, &error));
    }
    Ok(())
}

/// Opens the file `path` that an export job just wrote, through `opener`. Refused if the
/// extension is not the one of `format`: the user can type any file name in the save dialog, and
/// a file of another type (a script, say) must never be started from here.
pub(crate) fn open_written_file(
    path: &Path,
    format: ExportFormat,
    opener: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), CommandError> {
    let matches = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(format.extension()));
    if !matches {
        return Err(CommandError::export(
            ExportReason::WrongExtension,
            format_args!(
                "not opened: {} is not a .{} file",
                path.display(),
                format.extension()
            ),
        ));
    }
    opener(path).map_err(|e| {
        CommandError::export(
            ExportReason::OpenFailed,
            format_args!("cannot open {}: {e}", path.display()),
        )
    })
}

/// The process that opens `path` with the default application of the operating system. The path
/// is one argument, never part of a shell string. It must be absolute, so it cannot be read as
/// an option.
fn open_command(path: &Path) -> std::io::Result<std::process::Command> {
    let path = std::path::absolute(path)?;
    let mut command;
    if cfg!(target_os = "macos") {
        command = std::process::Command::new("open");
        command.arg(&path);
    } else if cfg!(target_os = "windows") {
        // `start` is a command of cmd.exe. Rust quotes an argument only if it has a space, so the
        // path is quoted by hand and passed raw: inside double quotes `& | < > ^ ( )` are plain
        // characters to cmd.
        let quoted = windows_start_argument(&path)?;
        command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::raw_arg(&mut command, quoted);
        #[cfg(not(windows))]
        command.arg(quoted);
    } else {
        command = std::process::Command::new("xdg-open");
        command.arg(&path);
    }
    Ok(command)
}

/// The path as the quoted argument of `cmd /C start "" <path>`. Inside double quotes cmd treats
/// most operators as text, but `%` is still expanded and `"` would end the quote, so a path
/// with either is refused. Not Windows specific code, so every platform tests it.
fn windows_start_argument(path: &Path) -> std::io::Result<String> {
    let text = path.to_string_lossy();
    if text.contains(['%', '"', '\n', '\r', '\0']) {
        return Err(std::io::Error::other(
            "the file name contains %, a double quote or a line break",
        ));
    }
    Ok(format!("\"{text}\""))
}

/// Starts the default application for `path` and does not wait for it.
pub(crate) fn open_with_system(path: &Path) -> std::io::Result<()> {
    let mut child = open_command(path)?
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    // Reap the launcher (`open`, `xdg-open`) without blocking the job thread for long; the
    // application itself keeps running.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// `path` with the extension of `format` if it has none.
fn with_extension(mut path: PathBuf, format: ExportFormat) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(format.extension());
    }
    path
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
        Ok(dimo_report::issue::report_warnings(project, kind, changed)?)
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
    /// For a normal report export of a project without a report date: the patch that stored
    /// today's date, to emit (T3.9).
    pub date: Option<ProjectPatched>,
}

/// Takes the snapshot of an export following [`dimo_export::plan`]: executes the command the plan
/// puts before the snapshot (the numbering lock of an issued drawing or list export, T1.9, or
/// today's report date that a normal report export stores, T3.9), then gives the snapshot the
/// report info the report prints. A report exported as issued changes nothing here; it is
/// recorded after the write (M3 decisions 15 and 17).
pub fn take_snapshot(
    session: &mut AppSession,
    request: ExportRequest,
) -> Result<ExportStart, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    let plan = dimo_export::plan(
        project,
        request.format,
        request.issued,
        &session.issue_defaults(),
    );
    let before = plan
        .before
        .clone()
        .map(|command| session.execute(command))
        .transpose()?;
    let mut snapshot = session.export_snapshot(request.format.needs_drawing())?;
    plan.apply(&mut snapshot.project);
    let (lock, date) = if request.format.report().is_some() {
        (None, before)
    } else {
        (before, None)
    };
    Ok(ExportStart {
        snapshot,
        issued: plan.issued.then_some(plan.report_info).flatten(),
        lock,
        date,
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
            let engine = tile_service(&app.state::<TileState>())
                .ok()
                .and_then(|tiles| tiles.engine().cloned());
            Some(engine.ok_or_else(|| {
                CommandError::export(ExportReason::NoPdfEngine, "PDFium is not loaded")
            })?)
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
        date,
    } = with_session(app.clone(), move |_, session| {
        take_snapshot(session, request)
    })
    .await?;
    for patched in [lock, date].into_iter().flatten() {
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
        return Err(CommandError::task_failed(e));
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
                let issued = issued.map(|info| self.record(&info, sha256));
                let open_error = self
                    .request
                    .open_after
                    .then(|| open_written_file(self.path, self.request.format, open_with_system))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_after_export_opens_only_files_of_the_exported_type() {
        let mut opened = Vec::new();
        let ok = open_written_file(
            Path::new("/tmp/part_check-sheet.PDF"),
            ExportFormat::CheckSheetPdf,
            |p| {
                opened.push(p.to_path_buf());
                Ok(())
            },
        );
        assert!(ok.is_ok());
        assert_eq!(opened, [PathBuf::from("/tmp/part_check-sheet.PDF")]);
        for (name, format) in [
            ("/tmp/run.bat", ExportFormat::CheckSheetPdf),
            ("/tmp/noextension", ExportFormat::Csv),
            ("/tmp/list.xlsx", ExportFormat::Csv),
        ] {
            let refused = open_written_file(Path::new(name), format, |_| {
                panic!("{name} must not be opened")
            });
            assert!(
                matches!(
                    refused,
                    Err(CommandError::Export {
                        reason: ExportReason::WrongExtension,
                        ..
                    })
                ),
                "{name}"
            );
        }
        let failed = open_written_file(Path::new("/tmp/a.csv"), ExportFormat::Csv, |_| {
            Err(std::io::Error::other("no application"))
        });
        assert!(
            matches!(&failed, Err(CommandError::Export { reason: ExportReason::OpenFailed, message }) if message.contains("no application")),
            "{failed:?}"
        );
    }

    #[test]
    fn windows_start_argument_is_quoted_and_refuses_expansion() {
        let quoted = windows_start_argument(Path::new(r"C:\Temp\a&calc (1) ^b.pdf")).unwrap();
        assert_eq!(quoted, "\"C:\\Temp\\a&calc (1) ^b.pdf\"");
        for bad in ["a%PATH%.pdf", "a\"b.pdf", "a\nb.pdf"] {
            assert!(windows_start_argument(Path::new(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn open_command_passes_the_path_as_one_argument() {
        let path = Path::new("/tmp/a b; touch x & c.pdf");
        let command = open_command(path).unwrap();
        let args: Vec<_> = command.get_args().collect();
        let last = args.last().unwrap();
        assert_eq!(
            std::path::Path::new(last),
            std::path::absolute(path).unwrap()
        );
        if cfg!(target_os = "macos") {
            assert_eq!(command.get_program(), "open");
            assert_eq!(args.len(), 1);
        } else if cfg!(target_os = "windows") {
            assert_eq!(command.get_program(), "cmd");
            assert_eq!(args.len(), 4);
        } else {
            assert_eq!(command.get_program(), "xdg-open");
            assert_eq!(args.len(), 1);
        }
        // A relative path becomes absolute, so it cannot look like an option.
        let relative = open_command(Path::new("-n.pdf")).unwrap();
        let arg = relative.get_args().last().unwrap().to_owned();
        assert!(Path::new(&arg).is_absolute());
    }

    #[test]
    fn missing_extensions_are_added() {
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
            Err(CommandError::Export {
                reason: ExportReason::WriteFailed,
                ..
            })
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
