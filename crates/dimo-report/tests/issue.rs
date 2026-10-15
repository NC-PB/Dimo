//! Issuing rules (MF F2, D-23, T3.9, M3 decisions 15 to 17): the record command and the checks
//! before issuing, on the shared report fixture.

#![allow(clippy::expect_used, clippy::unwrap_used)] // Test crate; rust.md allows it in tests.

#[path = "common/fixture.rs"]
mod fixture;

use dimo_core::{
    Command, FixedEnvironment, ReportFormat, ReportInfo, ReportKind, Sha256Hex, TemplateRef,
};
use dimo_report::issue::{
    IssueDefaults, IssuedFile, ReportField, issue_command, issued_info, report_checks,
    report_warnings,
};

fn defaults() -> IssueDefaults {
    IssueDefaults {
        company_name: "Muster GmbH".into(),
        inspector: "Anna".into(),
        today: "2026-10-13".into(),
    }
}

fn file() -> IssuedFile {
    IssuedFile {
        kind: ReportKind::InspectionReport,
        format: ReportFormat::Pdf,
        template: TemplateRef {
            id: "dimo-default".into(),
            version: 1,
        },
        output_sha256: Sha256Hex::of(b"file"),
    }
}

#[test]
fn issuing_locks_stores_and_records_in_one_undo_step() {
    let mut project = fixture::project();
    project.report_info = ReportInfo::default();
    let before = project.clone();
    let printed = issued_info(&project.report_info, &defaults());
    let command = issue_command(&project, &printed, file());
    let Command::Batch { commands } = &command else {
        panic!("not a batch: {command:?}");
    };
    assert!(matches!(commands[0], Command::LockNumbering { .. }));
    assert!(matches!(&commands[1], Command::UpdateReportInfo { values } if values.len() == 3));
    assert!(matches!(commands[2], Command::RecordIssuedReport { .. }));

    let mut document = dimo_core::Document::new(project);
    let mut env = FixedEnvironment::new();
    document.execute(command, &mut env).unwrap();
    let project = document.project();
    assert!(project.is_numbering_locked());
    assert_eq!(project.report_info, printed);
    assert_eq!(project.issued_reports.len(), 1);
    document.undo(&mut env).unwrap();
    assert_eq!(document.project(), &before);

    // A second issue of the locked project with the printed info stored records only.
    let mut issued = before;
    issued.report_info = printed.clone();
    issued.numbering.lock = document_lock();
    let Command::Batch { commands } = issue_command(&issued, &printed, file()) else {
        panic!("not a batch");
    };
    assert_eq!(commands.len(), 1);
}

/// A numbering lock as an issued export leaves it.
fn document_lock() -> Option<dimo_core::NumberingLock> {
    let mut document = dimo_core::Document::new(fixture::project());
    let mut env = FixedEnvironment::new();
    document
        .execute(
            Command::LockNumbering {
                reason: dimo_core::LockReason::IssuedReport,
            },
            &mut env,
        )
        .unwrap();
    document.project().numbering.lock.clone()
}

#[test]
fn checks_name_empty_header_fields_after_the_fill() {
    let mut project = fixture::project();
    project.report_info = ReportInfo::default();
    project.info.part_number = String::new();
    let checks = report_checks(&project, ReportKind::InspectionReport, &[], &defaults()).unwrap();
    assert_eq!(
        checks.missing_fields,
        [ReportField::PartNumber, ReportField::ReportNumber],
        "company and inspector are filled when issuing"
    );
    assert_eq!(checks.samples, 3);
    assert!(checks.missing_values > 0);
    let empty = IssueDefaults::default();
    let checks = report_checks(&project, ReportKind::CheckSheet, &[], &empty).unwrap();
    assert_eq!(checks.missing_fields.len(), 4);
    assert_eq!((checks.missing_values, checks.not_evaluated), (0, 0));
    assert_eq!(
        checks.warnings,
        report_warnings(&project, ReportKind::CheckSheet, false).unwrap()
    );
}
