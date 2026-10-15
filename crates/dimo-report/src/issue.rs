//! Report issuing rules (D-23, T3.9, M3 decisions 15 to 17, MF F2): the report date, the
//! report info of new projects and of issued reports, the commands that record an issued
//! report, and the checks and warnings before issuing.
//!
//! The rules take the values that come from outside the project as [`IssueDefaults`]: the
//! company name of the app settings, the audit user and today's local date. The crate never
//! reads a clock (rule 11); the caller passes the date in.
//!
//! # The report date rule (T3.9)
//!
//! A report prints the stored report date, or today's date when the stored one is empty
//! ([`report_date`]). Preview, Save copy, Print and the issued export all print
//! [`printed_info`] or [`issued_info`], which apply this one rule. A normal report export of a
//! project without a report date stores today's date first ([`date_to_store`]), so later exports
//! print the same date and stay byte identical (FR-EXP-11); an issued export stores it with the
//! record in one undo step ([`issue_command`]).

use dimo_core::{
    CharId, Command, LockReason, Project, ReportFormat, ReportInfo, ReportInfoField, ReportKind,
    Sha256Hex, TemplateRef, measurement_state,
};
use serde::{Deserialize, Serialize};

use crate::{ReportError, ReportModel, ReportOptions};

/// Values from outside the project that issuing fills in (M3 decision 15, T3.9).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IssueDefaults {
    /// Company name of the app settings.
    pub company_name: String,
    /// The audit user (D-27), printed as inspector.
    pub inspector: String,
    /// Today's date in the time zone of the computer, `YYYY-MM-DD`. Passed in by the caller.
    pub today: String,
}

/// The report date a report prints today: the stored date, or `today` when the stored one is
/// empty or blank (T3.9). The one report date rule.
pub fn report_date<'a>(info: &'a ReportInfo, today: &'a str) -> &'a str {
    if info.report_date.trim().is_empty() {
        today
    } else {
        &info.report_date
    }
}

/// The value a normal report export stores before it renders: today's date when the stored
/// report date is empty (T3.9), `None` when a date is set.
pub fn date_to_store(info: &ReportInfo, today: &str) -> Option<ReportInfoField> {
    info.report_date
        .trim()
        .is_empty()
        .then(|| ReportInfoField::ReportDate(today.to_owned()))
}

/// The report info a report that is not issued prints today: `info` with the date of
/// [`report_date`]. Preview, Save copy and Print print this.
pub fn printed_info(info: &ReportInfo, today: &str) -> ReportInfo {
    ReportInfo {
        report_date: report_date(info, today).to_owned(),
        ..info.clone()
    }
}

/// The report info a report exported as issued prints (M3 decisions 15 and 17): an empty
/// company name filled from the settings, an empty inspector filled with the audit user, and the
/// date of [`report_date`]. Fields that are set stay as they are.
pub fn issued_info(info: &ReportInfo, defaults: &IssueDefaults) -> ReportInfo {
    let mut filled = printed_info(info, &defaults.today);
    if filled.company_name.trim().is_empty() {
        filled.company_name.clone_from(&defaults.company_name);
    }
    if filled.inspected_by.trim().is_empty() {
        filled.inspected_by.clone_from(&defaults.inspector);
    }
    filled
}

/// The report info a new project starts with (M3 decision 15): the company name of the
/// settings and the audit user as inspector. No date: it is set at the first report export.
pub fn initial_report_info(defaults: &IssueDefaults) -> ReportInfo {
    ReportInfo {
        company_name: defaults.company_name.clone(),
        inspected_by: defaults.inspector.clone(),
        ..ReportInfo::default()
    }
}

/// A report file written as issued, for its record (data model `IssuedReport`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedFile {
    /// Which document.
    pub kind: ReportKind,
    /// File format.
    pub format: ReportFormat,
    /// The template it was rendered with.
    pub template: TemplateRef,
    /// SHA-256 of the written file.
    pub output_sha256: Sha256Hex,
}

/// The one undo step that records `file` as issued (D-23, M3 decision 17): locks the numbering
/// if it is not locked yet, stores `printed` (the report info as printed, see [`issued_info`])
/// and adds the [`dimo_core::IssuedReport`].
pub fn issue_command(project: &Project, printed: &ReportInfo, file: IssuedFile) -> Command {
    let mut commands = Vec::new();
    if !project.is_numbering_locked() {
        commands.push(Command::LockNumbering {
            reason: LockReason::IssuedReport,
        });
    }
    let values = project.report_info.changes_to(printed);
    if !values.is_empty() {
        commands.push(Command::UpdateReportInfo { values });
    }
    commands.push(Command::RecordIssuedReport {
        kind: file.kind,
        template: file.template,
        format: file.format,
        output_sha256: file.output_sha256,
    });
    Command::Batch { commands }
}

/// Warnings before a report is exported as issued (M3 decision 17). They never block the
/// export; the user confirms them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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

/// The warnings of report `kind` of `project`; `limits_changed` from the audit log (M3
/// decision 16).
pub fn report_warnings(
    project: &Project,
    kind: ReportKind,
    limits_changed: bool,
) -> Result<ReportWarnings, ReportError> {
    let model = ReportModel::build(project, &ReportOptions::new(kind, dimo_core::Language::En))?;
    Ok(ReportWarnings {
        draft_limits: model.flags.draft_limits,
        open_results: model.flags.open_results,
        limits_changed,
    })
}

/// A report header field the Report tab asks for before issuing (T3.19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
/// the last issued report; the header fields are checked as [`issued_info`] would print them.
pub fn report_checks(
    project: &Project,
    kind: ReportKind,
    changed: &[CharId],
    defaults: &IssueDefaults,
) -> Result<ReportChecks, ReportError> {
    let filled = issued_info(&project.report_info, defaults);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> IssueDefaults {
        IssueDefaults {
            company_name: "Muster GmbH".into(),
            inspector: "Anna".into(),
            today: "2026-10-13".into(),
        }
    }

    #[test]
    fn the_report_date_is_the_stored_one_or_today() {
        let mut info = ReportInfo::default();
        assert_eq!(report_date(&info, "2026-10-13"), "2026-10-13");
        assert_eq!(
            date_to_store(&info, "2026-10-13"),
            Some(ReportInfoField::ReportDate("2026-10-13".into()))
        );
        info.report_date = "  ".into();
        assert_eq!(report_date(&info, "2026-10-13"), "2026-10-13");
        info.report_date = "2026-01-05".into();
        assert_eq!(report_date(&info, "2026-10-13"), "2026-01-05");
        assert_eq!(date_to_store(&info, "2026-10-13"), None);
        assert_eq!(printed_info(&info, "2026-10-13"), info);
    }

    #[test]
    fn issued_info_fills_only_empty_fields() {
        let info = ReportInfo {
            report_number: "IR-1".into(),
            inspected_by: "  ".into(),
            ..ReportInfo::default()
        };
        let filled = issued_info(&info, &defaults());
        assert_eq!(filled.company_name, "Muster GmbH");
        assert_eq!(filled.inspected_by, "Anna");
        assert_eq!(filled.report_date, "2026-10-13");
        assert_eq!(filled.report_number, "IR-1");
        let set = ReportInfo {
            company_name: "Other AG".into(),
            inspected_by: "Ben".into(),
            report_date: "2026-01-05".into(),
            ..ReportInfo::default()
        };
        assert_eq!(issued_info(&set, &defaults()), set);
        // Not issued: only the date rule applies.
        assert_eq!(printed_info(&info, "2026-10-13").company_name, "");
    }

    #[test]
    fn new_projects_start_with_company_and_inspector_but_no_date() {
        let info = initial_report_info(&defaults());
        assert_eq!(info.company_name, "Muster GmbH");
        assert_eq!(info.inspected_by, "Anna");
        assert_eq!(info.report_date, "");
    }
}
