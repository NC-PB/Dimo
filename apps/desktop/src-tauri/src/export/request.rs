//! What the frontend asks for and what it hears back: the [`ExportRequest`], its render options
//! and save dialog filter, and the outcome events of an export job.

use dimo_core::Language;
use dimo_export::{BalloonMode, ExportFormat, RenderOptions};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::dialogs::Filter;
use crate::ipc::{CommandError, JobId};
use crate::settings::PdfBalloons;

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

/// File type filter of the save dialog for `format`.
pub const fn filter(format: ExportFormat) -> Filter {
    let extensions: &'static [&'static str] = match format.extension().as_bytes() {
        b"pdf" => &["pdf"],
        b"csv" => &["csv"],
        _ => &["xlsx"],
    };
    Filter {
        name: format.filter_name(),
        extensions,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_filters_follow_the_format() {
        for format in ExportFormat::ALL {
            assert_eq!(filter(format).extensions, [format.extension()]);
            assert_eq!(filter(format).name, format.filter_name());
        }
        assert_eq!(filter(ExportFormat::InspectionReportXlsx).name, "Excel");
    }

    #[test]
    fn balloon_modes_map() {
        let request = ExportRequest {
            format: ExportFormat::BalloonedPdf,
            language: Language::En,
            pdf_balloons: PdfBalloons::Annotations,
            blank_columns: 4,
            issued: false,
            open_after: false,
        };
        assert_eq!(request.render_options().balloons, BalloonMode::Annotations);
    }
}
