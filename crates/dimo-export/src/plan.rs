//! The plan of an export (D-23, T3.9, M3 decisions 15 and 17, MF F2): what to execute before
//! the snapshot is taken, which report info a report prints, and what to record after a report
//! exported as issued was written. The rules themselves live in [`dimo_report::issue`].
//!
//! - Ballooned PDF and characteristic list exported as issued: the numbering is locked before
//!   the snapshot, so the issued numbers are exactly the exported ones. The lock stays when the
//!   export fails afterwards.
//! - Report, not issued (Save copy, Print, the preview): prints the stored report date or today
//!   ([`dimo_report::issue::report_date`]). An export stores today's date first when none is
//!   stored, as one undo step, so later exports print the same date (FR-EXP-11). The preview
//!   prints the same info without storing anything.
//! - Report exported as issued: prints [`dimo_report::issue::issued_info`] (company, inspector
//!   and date filled) and changes nothing before the write. After the write,
//!   [`record_command`] locks, stores the printed info and records the file in one undo step.

use dimo_core::{Command, LockReason, Project, ReportInfo, Sha256Hex};
use dimo_report::issue::{
    IssueDefaults, IssuedFile, date_to_store, issue_command, issued_info, printed_info,
};

use crate::{ExportError, ExportFormat, bundled_template_ref};

/// What an export does around its render (see the module docs).
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPlan {
    /// A command to execute as its own undo step before the snapshot: the numbering lock of an
    /// issued drawing or list export, or the report date a normal report export stores.
    pub before: Option<Command>,
    /// For a report: the report info it prints, to put into the snapshot. `None` for drawing
    /// and list exports.
    pub report_info: Option<ReportInfo>,
    /// True for a report exported as issued: record it with [`record_command`] once the file is
    /// written.
    pub issued: bool,
}

impl ExportPlan {
    /// `project` as the export prints it: with the report info of the plan.
    pub fn apply(&self, project: &mut Project) {
        if let Some(info) = &self.report_info {
            project.report_info.clone_from(info);
        }
    }
}

/// The plan of exporting `project` as `format`, issued or not, with `defaults` from the app
/// settings and today's date.
pub fn plan(
    project: &Project,
    format: ExportFormat,
    issued: bool,
    defaults: &IssueDefaults,
) -> ExportPlan {
    let info = &project.report_info;
    match (format.report().is_some(), issued) {
        (false, issued) => ExportPlan {
            before: issued.then_some(Command::LockNumbering {
                reason: LockReason::IssuedReport,
            }),
            report_info: None,
            issued: false,
        },
        (true, false) => ExportPlan {
            before: date_to_store(info, &defaults.today)
                .map(|date| Command::UpdateReportInfo { values: vec![date] }),
            report_info: Some(printed_info(info, &defaults.today)),
            issued: false,
        },
        (true, true) => ExportPlan {
            before: None,
            report_info: Some(issued_info(info, defaults)),
            issued: true,
        },
    }
}

/// The one undo step that records a report written as issued (M3 decision 17): `project` is
/// the project now, `printed` the report info the file shows, `output_sha256` the hash of the
/// written file. Refused for a format that is not a report.
pub fn record_command(
    project: &Project,
    printed: &ReportInfo,
    format: ExportFormat,
    output_sha256: Sha256Hex,
) -> Result<Command, ExportError> {
    let (kind, file_format) = format.report().ok_or(ExportError::NotAReport(format))?;
    let file = IssuedFile {
        kind,
        format: file_format,
        template: bundled_template_ref()?,
        output_sha256,
    };
    Ok(issue_command(project, printed, file))
}
