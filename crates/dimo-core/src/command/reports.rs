//! Report data: header and approval data, revision labels, issued reports (M3 decisions 1
//! and 17, FR-RVC-04, D-23).

use super::{CommandError, Tx, index_u32};
use crate::env::Environment;
use crate::id::RevisionId;
use crate::measurement::is_date_or_empty;
use crate::patch::Change;
use crate::report::{IssuedReport, ReportFormat, ReportInfo, ReportKind, TemplateRef};
use crate::sheet::Sha256Hex;

pub(super) fn update_report_info(tx: &mut Tx<'_>, info: &ReportInfo) -> Result<(), CommandError> {
    if !is_date_or_empty(&info.report_date) {
        return Err(CommandError::InvalidDate(info.report_date.clone()));
    }
    if tx.project.report_info != *info {
        tx.push(Change::ReportInfoChanged {
            before: tx.project.report_info.clone(),
            after: info.clone(),
        })?;
    }
    Ok(())
}

pub(super) fn update_revision_label(
    tx: &mut Tx<'_>,
    revision: RevisionId,
    label: &str,
) -> Result<(), CommandError> {
    let current = tx
        .project
        .revisions
        .iter()
        .find(|r| r.id == revision)
        .ok_or(CommandError::UnknownRevision(revision))?;
    let label = label.trim();
    if current.label != label {
        tx.push(Change::RevisionLabelChanged {
            revision,
            before: current.label.clone(),
            after: label.to_owned(),
        })?;
    }
    Ok(())
}

pub(super) fn record_issued_report(
    tx: &mut Tx<'_>,
    kind: ReportKind,
    template: &TemplateRef,
    format: ReportFormat,
    output_sha256: &Sha256Hex,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    let revision = tx
        .project
        .current_revision()
        .ok_or(CommandError::UnknownRevision(tx.project.current_revision))?;
    let report = IssuedReport {
        kind,
        template: template.clone(),
        revision: revision.id,
        drawing_sha256: revision.sha256,
        exported_at: env.now(),
        output_sha256: *output_sha256,
        format,
    };
    tx.push(Change::IssuedReportInserted {
        index: index_u32(tx.project.issued_reports.len()),
        report,
    })
}
