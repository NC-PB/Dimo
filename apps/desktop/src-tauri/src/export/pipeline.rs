//! The steps of an export without Tauri: take the snapshot following [`dimo_export::plan`],
//! render through [`dimo_export`], write the file. Tests drive them directly.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use dimo_core::{ReportInfo, Sha256Hex};
use dimo_pdf::PdfEngine;

use super::ExportRequest;
use crate::ipc::{CommandError, ExportReason};
use crate::job::JobStop;
use crate::session::{AppSession, ExportSnapshot, ProjectPatched};

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

/// Writes `bytes` to `path` through a unique temporary file in the same directory
/// ([`crate::fsio::write_atomic`]), so a failed or cancelled export never leaves half a file.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    if path.file_name().is_none() {
        return Err(CommandError::export(
            ExportReason::InvalidFileName,
            format_args!("{} is not a file name", path.display()),
        ));
    }
    crate::fsio::write_atomic(path, bytes).map_err(|e| CommandError::export_io(path, &e))
}

/// What an export starts from.
#[derive(Debug)]
pub struct ExportStart {
    /// The copy the job renders.
    pub snapshot: ExportSnapshot,
    /// For a report exported as issued: the report info it prints, recorded after the write.
    pub issued: Option<ReportInfo>,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
