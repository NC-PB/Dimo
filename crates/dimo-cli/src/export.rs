//! `dimo export`: exports a saved project through the same registry as the desktop app
//! ([`dimo_export::plan`] and [`dimo_export::render`], MF F2), so the same project, options and
//! report date give byte identical files (FR-EXP-11).
//!
//! The command never changes the project file. A report prints the stored report date; a project
//! without one needs the date passed in (`--date`), which the app would store at its first report
//! export (T3.9). The tool reads no clock for it, so a scripted run stays reproducible.

use std::path::Path;

use dimo_core::Language;
use dimo_export::{ExportError, ExportFormat, RenderOptions, Source};
use dimo_io::project::{ProjectError, ProjectSession};
use dimo_pdf::PdfEngine;
use dimo_report::issue::{IssueDefaults, report_date};

/// Why an export of the command line tool failed.
#[derive(Debug, thiserror::Error)]
pub enum CliExportError {
    /// The project file could not be opened.
    #[error("cannot open the project: {0}")]
    Open(#[from] ProjectError),
    /// A report of a project without a report date, and no date was given.
    #[error("the project has no report date; pass --date YYYY-MM-DD")]
    NoReportDate,
    /// The drawing of the current revision is missing from the project.
    #[error("the drawing of the current revision is missing")]
    NoDrawing,
    /// Rendering failed.
    #[error(transparent)]
    Export(#[from] ExportError),
}

/// The name of `format` on the command line, e.g. `check-sheet-pdf`.
pub const fn format_name(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::BalloonedPdf => "ballooned-pdf",
        ExportFormat::Csv => "csv",
        ExportFormat::Xlsx => "xlsx",
        ExportFormat::CheckSheetPdf => "check-sheet-pdf",
        ExportFormat::CheckSheetXlsx => "check-sheet-xlsx",
        ExportFormat::InspectionReportPdf => "inspection-report-pdf",
        ExportFormat::InspectionReportXlsx => "inspection-report-xlsx",
    }
}

/// The format named `name` on the command line.
pub fn parse_format(name: &str) -> Result<ExportFormat, String> {
    ExportFormat::ALL
        .into_iter()
        .find(|f| format_name(*f) == name)
        .ok_or_else(|| {
            let names: Vec<&str> = ExportFormat::ALL.into_iter().map(format_name).collect();
            format!(
                "unknown format {name:?}, expected one of {}",
                names.join(", ")
            )
        })
}

/// The report language named `code` (`en`, `de`).
pub fn parse_language(code: &str) -> Result<Language, String> {
    [Language::En, Language::De]
        .into_iter()
        .find(|l| l.code() == code)
        .ok_or_else(|| format!("unknown language {code:?}, expected en or de"))
}

/// The bytes of exporting the project file at `path` with `options`. `today` is the report date
/// for a project without one (see the module docs); `pdf` is needed for the ballooned PDF.
pub fn export_project(
    path: &Path,
    options: RenderOptions,
    today: Option<&str>,
    pdf: Option<&PdfEngine>,
) -> Result<Vec<u8>, CliExportError> {
    let (session, _) = ProjectSession::open(path)?;
    let mut project = session.project().clone();
    let defaults = IssueDefaults {
        today: today.unwrap_or_default().to_owned(),
        ..IssueDefaults::default()
    };
    if options.format.report().is_some()
        && report_date(&project.report_info, &defaults.today)
            .trim()
            .is_empty()
    {
        return Err(CliExportError::NoReportDate);
    }
    dimo_export::plan(&project, options.format, false, &defaults).apply(&mut project);
    let drawing = if options.format.needs_drawing() {
        Some(session.current_drawing().ok_or(CliExportError::NoDrawing)?)
    } else {
        None
    };
    let modified = session.modified();
    let source = Source {
        project: &project,
        modified: &modified,
        drawing,
    };
    Ok(dimo_export::render(&source, options, pdf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_back() {
        for format in ExportFormat::ALL {
            assert_eq!(parse_format(format_name(format)), Ok(format));
        }
        assert!(parse_format("pdf").is_err());
        assert_eq!(parse_language("de"), Ok(Language::De));
        assert!(parse_language("fr").is_err());
    }
}
