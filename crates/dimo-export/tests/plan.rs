//! The export plan and the registry render (MF F2, D-23, T3.9, M3 decisions 15 and 17,
//! FR-EXP-11), on the report fixture of `dimo-report`.

#![allow(clippy::expect_used, clippy::unwrap_used)] // Test crate; rust.md allows it in tests.

#[path = "../../dimo-report/tests/common/fixture.rs"]
mod fixture;

use dimo_core::{Command, Language, ReportInfo, ReportInfoField, Sha256Hex, Timestamp};
use dimo_export::{BalloonMode, ExportError, ExportFormat, RenderOptions, Source, plan, render};
use dimo_report::issue::IssueDefaults;

fn defaults() -> IssueDefaults {
    IssueDefaults {
        company_name: "Muster GmbH".into(),
        inspector: "Anna".into(),
        today: "2026-10-13".into(),
    }
}

fn undated() -> dimo_core::Project {
    let mut project = fixture::project();
    project.report_info = ReportInfo {
        report_number: "IR-7".into(),
        ..ReportInfo::default()
    };
    project
}

#[test]
fn drawing_and_list_exports_lock_only_when_issued() {
    let project = undated();
    for format in [
        ExportFormat::BalloonedPdf,
        ExportFormat::Csv,
        ExportFormat::Xlsx,
    ] {
        let plain = plan(&project, format, false, &defaults());
        assert_eq!(plain.before, None);
        assert_eq!(plain.report_info, None);
        let issued = plan(&project, format, true, &defaults());
        assert!(matches!(issued.before, Some(Command::LockNumbering { .. })));
        assert!(!issued.issued && issued.report_info.is_none());
    }
}

#[test]
fn reports_print_one_date_and_store_it_only_when_not_issued() {
    let project = undated();
    for kind in [
        ExportFormat::CheckSheetPdf,
        ExportFormat::InspectionReportXlsx,
    ] {
        let plain = plan(&project, kind, false, &defaults());
        assert_eq!(
            plain.before,
            Some(Command::UpdateReportInfo {
                values: vec![ReportInfoField::ReportDate("2026-10-13".into())]
            })
        );
        let printed = plain.report_info.unwrap();
        assert_eq!(printed.report_date, "2026-10-13");
        assert_eq!(printed.company_name, "", "only issuing fills the company");
        assert!(!plain.issued);

        let issued = plan(&project, kind, true, &defaults());
        assert_eq!(issued.before, None, "nothing changes before the write");
        assert!(issued.issued);
        let info = issued.report_info.unwrap();
        assert_eq!(info.report_date, printed.report_date, "the same date rule");
        assert_eq!(
            (info.company_name.as_str(), info.inspected_by.as_str()),
            ("Muster GmbH", "Anna")
        );
        assert_eq!(info.report_number, "IR-7");
    }
    // A stored date is printed and nothing is stored.
    let mut dated = project;
    dated.report_info.report_date = "2026-01-05".into();
    let plain = plan(&dated, ExportFormat::CheckSheetPdf, false, &defaults());
    assert_eq!(plain.before, None);
    assert_eq!(plain.report_info.unwrap().report_date, "2026-01-05");
    let mut applied = dated.clone();
    plan(&dated, ExportFormat::CheckSheetPdf, true, &defaults()).apply(&mut applied);
    assert_eq!(applied.report_info.report_date, "2026-01-05");
    assert_eq!(applied.report_info.company_name, "Muster GmbH");
}

#[test]
fn records_are_for_reports_only() {
    let project = undated();
    let sha = Sha256Hex::of(b"file");
    assert!(matches!(
        dimo_export::record_command(&project, &project.report_info, ExportFormat::Csv, sha),
        Err(ExportError::NotAReport(ExportFormat::Csv))
    ));
    let command = dimo_export::record_command(
        &project,
        &project.report_info,
        ExportFormat::InspectionReportPdf,
        sha,
    )
    .unwrap();
    let Command::Batch { commands } = command else {
        panic!("not a batch");
    };
    assert!(matches!(
        commands.last(),
        Some(Command::RecordIssuedReport { template, .. })
            if template.id == dimo_export::REPORT_TEMPLATE
    ));
}

#[test]
fn reports_and_lists_render_the_same_bytes_twice() {
    let mut project = fixture::project();
    plan(
        &project.clone(),
        ExportFormat::CheckSheetPdf,
        false,
        &defaults(),
    )
    .apply(&mut project);
    let modified = Timestamp::parse("2026-10-13T08:00:00Z").unwrap();
    let source = Source {
        project: &project,
        modified: &modified,
        drawing: None,
    };
    for format in ExportFormat::ALL {
        let options = RenderOptions {
            format,
            language: Language::De,
            balloons: BalloonMode::PageContent,
            blank_columns: 4,
        };
        if format.needs_drawing() {
            assert!(matches!(
                render(&source, options, None),
                Err(ExportError::NoPdfEngine)
            ));
            continue;
        }
        let first = render(&source, options, None).unwrap();
        assert_eq!(first, render(&source, options, None).unwrap(), "{format:?}");
        assert!(first.len() > 100, "{format:?}");
    }
}
