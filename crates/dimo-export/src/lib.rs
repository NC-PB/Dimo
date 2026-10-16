//! Export registry of DimoMark (MF F2): the export formats, one [`render`] function used by the
//! desktop app and the command line tool, and the [`plan`] of an export (what to store before
//! the snapshot, what the report prints, what to record when it is issued).
//!
//! Formats: ballooned PDF (FR-EXP-01, D-33), the characteristic list as CSV or XLSX
//! (FR-EXP-09, D-32), and the check sheet and inspection report as PDF or XLSX (FR-EXP-04).
//! The bytes depend only on the [`Source`] and the [`RenderOptions`]: dates written into the
//! files come from the project, never from a clock (FR-EXP-11, rule 11).
//!
//! A facade over `dimo-io` (list exporters), `dimo-report` (report model and templates) and
//! `dimo-pdf` (ballooned PDF). It does no file IO; writing the bytes is the caller's job.

pub mod plan;

use dimo_core::{Language, Project, ReportFormat, ReportKind, TemplateRef, Timestamp};
use dimo_io::export::{CsvListExporter, ExportOptions, Exporter, XlsxListExporter};
use dimo_pdf::{BalloonOutput, PdfDate, PdfEngine, PdfError, project_overlay};
use dimo_report::{ReportError, ReportModel, ReportOptions, TemplateSet};
use serde::{Deserialize, Serialize};

pub use plan::{ExportPlan, plan, record_command};

/// The bundled report template set (M3 decision 10).
pub const REPORT_TEMPLATE: &str = "dimo-default";

/// What to export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
    /// Every format, in menu order.
    pub const ALL: [Self; 7] = [
        Self::BalloonedPdf,
        Self::Csv,
        Self::Xlsx,
        Self::CheckSheetPdf,
        Self::CheckSheetXlsx,
        Self::InspectionReportPdf,
        Self::InspectionReportXlsx,
    ];

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

    /// True if the export needs the drawing file (the ballooned PDF).
    pub const fn needs_drawing(self) -> bool {
        matches!(self, Self::BalloonedPdf)
    }

    /// File name extension without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::BalloonedPdf | Self::CheckSheetPdf | Self::InspectionReportPdf => "pdf",
            Self::Csv => "csv",
            Self::Xlsx | Self::CheckSheetXlsx | Self::InspectionReportXlsx => "xlsx",
        }
    }

    /// Name of the file type for a save dialog filter.
    pub const fn filter_name(self) -> &'static str {
        match self {
            Self::BalloonedPdf | Self::CheckSheetPdf | Self::InspectionReportPdf => "PDF",
            Self::Csv => "CSV",
            Self::Xlsx | Self::CheckSheetXlsx | Self::InspectionReportXlsx => "Excel",
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
}

/// How the ballooned PDF carries the balloons (D-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BalloonMode {
    /// Vector graphics in the page content.
    #[default]
    PageContent,
    /// One annotation per balloon, dated with the project's last change.
    Annotations,
}

/// How to render an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    /// What to export.
    pub format: ExportFormat,
    /// Language of the list headers and of the reports (D-32); ignored for the ballooned PDF.
    pub language: Language,
    /// Balloons as page content or annotations; only for the ballooned PDF.
    pub balloons: BalloonMode,
    /// Blank result columns of a check sheet for a project without samples, 1 to 10 (M3
    /// decision 12); ignored otherwise.
    pub blank_columns: u8,
}

/// What an export reads: the project as it prints, its last change time and, for the
/// ballooned PDF, the drawing file of the current revision.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    /// The project. For reports, its report info is the one of [`ExportPlan::report_info`].
    pub project: &'a Project,
    /// Time of the last change of the project, for dates written into exports (FR-EXP-11).
    pub modified: &'a Timestamp,
    /// The drawing file of the current revision; needed for the ballooned PDF only.
    pub drawing: Option<&'a [u8]>,
}

/// Why an export could not be rendered.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The report could not be built or rendered.
    #[error(transparent)]
    Report(#[from] ReportError),
    /// The bundled report template set does not load.
    #[error("report template {REPORT_TEMPLATE}: {0}")]
    Template(ReportError),
    /// The characteristic list could not be written.
    #[error(transparent)]
    List(#[from] dimo_io::export::ExportError),
    /// The PDF component could not write the ballooned PDF.
    #[error(transparent)]
    Pdf(#[from] PdfError),
    /// The ballooned PDF needs a PDF engine and none was given.
    #[error("no PDF engine")]
    NoPdfEngine,
    /// The ballooned PDF needs the drawing and the source has none.
    #[error("the export source has no drawing")]
    NoDrawing,
    /// A report record was asked for a format that is not a report; a bug.
    #[error("{0:?} is not a report")]
    NotAReport(ExportFormat),
    /// A report was rendered from a [`Prepared`] of another format; a bug.
    #[error("the report model was not built")]
    NotPrepared,
    /// The project time cannot be written as a PDF date.
    #[error("invalid project time {0}")]
    InvalidTime(String),
}

/// The bundled report template set.
pub fn report_template() -> Result<TemplateSet, ExportError> {
    TemplateSet::bundled(REPORT_TEMPLATE).map_err(ExportError::Template)
}

/// The reference of the bundled template set, as an issued report records it. Loaded once per
/// process, so callers can fetch it before they take a lock.
pub fn bundled_template_ref() -> Result<TemplateRef, ExportError> {
    static BUNDLED: std::sync::OnceLock<TemplateRef> = std::sync::OnceLock::new();
    if let Some(found) = BUNDLED.get() {
        return Ok(found.clone());
    }
    let found = template_ref(&report_template()?);
    Ok(BUNDLED.get_or_init(|| found).clone())
}

/// The reference of a template set, as an issued report records it.
pub fn template_ref(template: &TemplateSet) -> TemplateRef {
    TemplateRef {
        id: template.id().to_owned(),
        version: template.version(),
    }
}

/// What [`prepare`] built for [`Prepared::render`]: the slow first step of a report export,
/// split off so a job can report progress and stop between the steps (NFR-PERF-06).
#[derive(Debug)]
pub struct Prepared(Inner);

#[derive(Debug)]
enum Inner {
    /// Nothing to prepare: drawing and list exports.
    Direct,
    /// A report model and the template set to render it with.
    Report(Box<ReportModel>, TemplateSet),
}

/// Report options of `options` for `source`: the date falls back to the project's last change
/// (M3 decision 14), which [`plan`] never leaves to happen for app exports.
fn report_options(source: &Source<'_>, options: RenderOptions, kind: ReportKind) -> ReportOptions {
    ReportOptions {
        blank_columns: options.blank_columns,
        fallback_date: Some(source.modified.clone()),
        ..ReportOptions::new(kind, options.language)
    }
}

/// Step 1: builds the report model of a report export; nothing for other exports.
pub fn prepare(source: &Source<'_>, options: RenderOptions) -> Result<Prepared, ExportError> {
    let Some((kind, _)) = options.format.report() else {
        return Ok(Prepared(Inner::Direct));
    };
    let model = ReportModel::build(source.project, &report_options(source, options, kind))?;
    Ok(Prepared(Inner::Report(Box::new(model), report_template()?)))
}

impl Prepared {
    /// Step 2: the bytes of the file. `pdf` is needed for the ballooned PDF only.
    pub fn render(
        &self,
        source: &Source<'_>,
        options: RenderOptions,
        pdf: Option<&PdfEngine>,
    ) -> Result<Vec<u8>, ExportError> {
        if let Inner::Report(model, template) = &self.0 {
            return Ok(match options.format.report() {
                Some((_, ReportFormat::Xlsx)) => dimo_report::render_xlsx(template, model),
                _ => dimo_report::render_report(template, model),
            }?);
        }
        let list = ExportOptions {
            language: options.language,
        };
        let mut bytes = Vec::new();
        match options.format {
            ExportFormat::Csv => CsvListExporter.export(source.project, &list, &mut bytes)?,
            ExportFormat::Xlsx => XlsxListExporter.export(source.project, &list, &mut bytes)?,
            ExportFormat::BalloonedPdf => bytes = ballooned(source, options.balloons, pdf)?,
            ExportFormat::CheckSheetPdf
            | ExportFormat::CheckSheetXlsx
            | ExportFormat::InspectionReportPdf
            | ExportFormat::InspectionReportXlsx => {
                // `prepare` builds the model of every report format.
                return Err(ExportError::NotPrepared);
            }
        }
        Ok(bytes)
    }
}

fn ballooned(
    source: &Source<'_>,
    mode: BalloonMode,
    pdf: Option<&PdfEngine>,
) -> Result<Vec<u8>, ExportError> {
    let engine = pdf.ok_or(ExportError::NoPdfEngine)?;
    let drawing = source.drawing.ok_or(ExportError::NoDrawing)?;
    let output = match mode {
        BalloonMode::PageContent => BalloonOutput::PageContent,
        BalloonMode::Annotations => BalloonOutput::Annotations {
            date: PdfDate::from_timestamp(source.modified)
                .ok_or_else(|| ExportError::InvalidTime(source.modified.as_str().to_owned()))?,
        },
    };
    let overlay = project_overlay(source.project, output);
    Ok(engine.write_ballooned(drawing.to_vec(), overlay)?)
}

/// The bytes of an export of `source`: [`prepare`] then [`Prepared::render`]. Pure apart from
/// PDFium: same source and options, same bytes (FR-EXP-11). The desktop app and the command
/// line tool both render through here.
pub fn render(
    source: &Source<'_>,
    options: RenderOptions,
    pdf: Option<&PdfEngine>,
) -> Result<Vec<u8>, ExportError> {
    prepare(source, options)?.render(source, options, pdf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_follow_the_project() {
        let names: Vec<String> = ExportFormat::ALL
            .iter()
            .map(|f| f.file_name("part"))
            .collect();
        assert_eq!(
            names,
            [
                "part_ballooned.pdf",
                "part_characteristics.csv",
                "part_characteristics.xlsx",
                "part_check-sheet.pdf",
                "part_check-sheet.xlsx",
                "part_inspection-report.pdf",
                "part_inspection-report.xlsx",
            ]
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
        assert_eq!(ExportFormat::Csv.filter_name(), "CSV");
        assert!(ExportFormat::BalloonedPdf.needs_drawing());
        assert!(!ExportFormat::CheckSheetPdf.needs_drawing());
    }

    #[test]
    fn the_bundled_template_loads() {
        let template = report_template().unwrap();
        assert_eq!(template_ref(&template).id, REPORT_TEMPLATE);
    }
}
