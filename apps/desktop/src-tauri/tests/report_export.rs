//! Report exports of an open project (T3.8, FR-EXP-04, FR-EXP-11, M3 decisions 14, 15, 17):
//! check sheet and inspection report as PDF and XLSX from a session snapshot, byte identical
//! when exported twice; the job steps write nothing when cancelled; an issued report locks the
//! numbering, fills company and inspector and records the report in one undo step.
//!
//! Needs PDFium for the project (drawing import). Skips without it, fails instead with
//! `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use dimo_core::Language;
use dimo_core::{
    Command, FieldValue, Point, ReportFormat, ReportInfoField, ReportKind, ResultEntry, Sha256Hex,
    TemplateRef,
};
use dimo_desktop::export::{
    ExportFormat, ExportRequest, JobStop, REPORT_TEMPLATE, render, report_template,
    report_warnings, run_steps, take_snapshot,
};
use dimo_desktop::session::AppSession;
use dimo_desktop::settings::PdfBalloons;
use dimo_pdf::tiles::{TileConfig, TileService};
use dimo_pdf::{PdfEngine, PdfError};
use rust_decimal::Decimal;

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn service() -> Option<TileService> {
    match PdfEngine::start() {
        Ok(engine) => Some(TileService::new(engine, TileConfig::default())),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn drawing() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../../corpus/drawings/test_drawing_1.pdf")
}

fn request(format: ExportFormat, language: Language) -> ExportRequest {
    ExportRequest {
        format,
        language,
        pdf_balloons: PdfBalloons::PageContent,
        blank_columns: 4,
        issued: false,
        open_after: false,
    }
}

const REPORTS: [ExportFormat; 4] = [
    ExportFormat::CheckSheetPdf,
    ExportFormat::CheckSheetXlsx,
    ExportFormat::InspectionReportPdf,
    ExportFormat::InspectionReportXlsx,
];

/// A project with two characteristics with limits, two samples and three of four results.
fn session(tiles: &TileService) -> AppSession {
    let mut session = AppSession::new(None);
    session.set_user_name("Anna");
    session.set_company_name("Muster GmbH");
    session
        .create_from_drawing(tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    for (x, text, nominal) in [(150.0, "Ø8 ±0.1", 80), (270.0, "30 ±0.1", 300)] {
        session
            .execute(Command::AddCharacteristic {
                sheet,
                position: Point { x, y: 120.0 },
                anchor: Point { x, y: 150.0 },
                region: None,
                values: vec![
                    FieldValue::RequirementText(text.into()),
                    FieldValue::Nominal(Some(Decimal::new(nominal, 1)).into()),
                    FieldValue::UpperDev(Some(Decimal::new(1, 1)).into()),
                    FieldValue::LowerDev(Some(Decimal::new(-1, 1)).into()),
                ],
                insert_after: None,
            })
            .unwrap();
    }
    session.execute(Command::AddSamples { count: 2 }).unwrap();
    let project = session.project().unwrap();
    let (a, b) = (project.samples[0].id, project.samples[1].id);
    let (c1, c2) = (project.characteristics[0].id, project.characteristics[1].id);
    let entry = |sample, characteristic, text: &str| ResultEntry {
        sample,
        characteristic,
        text: text.to_owned(),
        comment: None,
    };
    session
        .execute(Command::SetResults {
            results: vec![
                entry(a, c1, "8.05"),
                entry(a, c2, "30.05"),
                entry(b, c1, "7,95"),
            ],
        })
        .unwrap();
    session
}

#[test]
fn reports_are_byte_identical_when_exported_twice() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let snapshot = session.export_snapshot(false).unwrap();
    for format in REPORTS {
        for language in [Language::En, Language::De] {
            let request = request(format, language);
            let first = render(&snapshot, &request, None).unwrap();
            // A new snapshot of the unchanged project, as a second click on export takes.
            let again = render(&session.export_snapshot(false).unwrap(), &request, None).unwrap();
            assert_eq!(first, again, "{format:?} {language:?}");
            let magic: &[u8] = if format.extension() == "pdf" {
                b"%PDF"
            } else {
                b"PK"
            };
            assert!(first.starts_with(magic), "{format:?}");
        }
    }
    // The language changes the file.
    let en = render(
        &snapshot,
        &request(ExportFormat::InspectionReportXlsx, Language::En),
        None,
    )
    .unwrap();
    let de = render(
        &snapshot,
        &request(ExportFormat::InspectionReportXlsx, Language::De),
        None,
    )
    .unwrap();
    assert_ne!(en, de);
}

#[test]
fn job_steps_write_the_file_or_nothing_when_cancelled() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let snapshot = session.export_snapshot(false).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let request = request(ExportFormat::InspectionReportPdf, Language::En);

    let path = dir.path().join("report.pdf");
    let mut steps = Vec::new();
    let sha = run_steps(
        &snapshot,
        &request,
        None,
        &path,
        &AtomicBool::new(false),
        &mut |fraction, message| steps.push((fraction, message.to_owned())),
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(sha, dimo_io::project::sha256(&bytes));
    assert_eq!(
        steps.iter().map(|(_, m)| m.as_str()).collect::<Vec<_>>(),
        ["building", "rendering", "writing"]
    );

    // Cancelled while building or rendering: the next step does not run, no file and no
    // temporary file. Once writing has started the file is completed.
    for cancel_at in 0..2 {
        let path = dir.path().join(format!("cancelled-{cancel_at}.pdf"));
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let result = run_steps(&snapshot, &request, None, &path, &cancel, &mut |_, _| {
            if seen == cancel_at {
                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            seen += 1;
        });
        assert_eq!(
            result,
            Err(JobStop::Cancelled),
            "cancel at step {cancel_at}"
        );
    }
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["report.pdf"], "cancelled jobs leave nothing behind");
}

#[test]
fn warnings_name_open_results_and_never_apply_to_blank_check_sheets() {
    let Some(tiles) = service() else { return };
    let session = session(&tiles);
    let project = session.project().unwrap();
    let report = report_warnings(project, ReportKind::InspectionReport, false).unwrap();
    assert!(report.open_results, "one result is missing");
    assert!(!report.draft_limits, "limits typed by hand");
    assert!(report.any());
    let sheet = report_warnings(project, ReportKind::CheckSheet, false).unwrap();
    assert!(!sheet.any(), "{sheet:?}");
    assert!(
        report_warnings(project, ReportKind::CheckSheet, true)
            .unwrap()
            .limits_changed
    );
}

#[test]
fn snapshots_lock_drawing_exports_at_once_and_fill_issued_reports_only() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    session
        .execute(Command::UpdateReportInfo {
            values: vec![
                ReportInfoField::CompanyName(String::new()),
                ReportInfoField::InspectedBy(String::new()),
            ],
        })
        .unwrap();
    let issued = |format| ExportRequest {
        issued: true,
        ..request(format, Language::En)
    };

    // A report exported as issued changes nothing yet; its snapshot prints the filled info.
    let start = take_snapshot(&mut session, issued(ExportFormat::InspectionReportPdf)).unwrap();
    assert!(start.lock.is_none());
    assert!(!session.project().unwrap().is_numbering_locked());
    let info = start.issued.unwrap();
    assert_eq!(info.company_name, "Muster GmbH");
    assert_eq!(info.inspected_by, "Anna");
    assert_eq!(start.snapshot.project.report_info, info);
    assert_eq!(session.project().unwrap().report_info.company_name, "");

    // Not issued: nothing filled.
    let plain = take_snapshot(
        &mut session,
        request(ExportFormat::CheckSheetPdf, Language::En),
    )
    .unwrap();
    assert!(plain.issued.is_none() && plain.lock.is_none());
    assert_eq!(plain.snapshot.project.report_info.company_name, "");

    // An issued list export locks at once, as in T1.9.
    let list = take_snapshot(&mut session, issued(ExportFormat::Csv)).unwrap();
    assert!(list.lock.is_some() && list.issued.is_none());
    assert!(list.snapshot.project.is_numbering_locked());
}

/// T3.9: the first report export of a project without a report date stores today's date as one
/// undo step; later exports reuse it, even on another day, and print identical files.
#[test]
fn first_report_export_stores_the_report_date_once() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    session.set_fixed_local_date(Some("2026-10-13"));
    assert_eq!(session.project().unwrap().report_info.report_date, "");
    let before = session.project().unwrap().clone();

    for format in REPORTS {
        let request = request(format, Language::En);
        let first = take_snapshot(&mut session, request).unwrap();
        // The first export stores the date; the later ones find it.
        assert_eq!(
            first.date.is_some(),
            format == ExportFormat::CheckSheetPdf,
            "{format:?}"
        );
        assert_eq!(
            session.project().unwrap().report_info.report_date,
            "2026-10-13"
        );
        assert_eq!(first.snapshot.project.report_info.report_date, "2026-10-13");
    }

    // Another day: the stored date stays, so a re-export is byte identical.
    let inspection = request(ExportFormat::InspectionReportPdf, Language::En);
    let one = render(&session.export_snapshot(false).unwrap(), &inspection, None).unwrap();
    session.set_fixed_local_date(Some("2026-11-02"));
    let again = take_snapshot(&mut session, inspection).unwrap();
    assert!(again.date.is_none());
    let two = render(&again.snapshot, &inspection, None).unwrap();
    assert_eq!(one, two);
    assert_eq!(
        session.project().unwrap().report_info.report_date,
        "2026-10-13"
    );

    // One undo step took the date; it is gone again.
    session.undo().unwrap();
    assert_eq!(session.project().unwrap(), &before);

    // A date the user entered is never replaced, and list exports do not store one.
    session
        .execute(Command::UpdateReportInfo {
            values: vec![ReportInfoField::ReportDate("2026-01-05".into())],
        })
        .unwrap();
    let kept = take_snapshot(
        &mut session,
        request(ExportFormat::CheckSheetXlsx, Language::En),
    )
    .unwrap();
    assert!(kept.date.is_none());
    assert_eq!(kept.snapshot.project.report_info.report_date, "2026-01-05");
    session.undo().unwrap();
    let list = take_snapshot(&mut session, request(ExportFormat::Csv, Language::En)).unwrap();
    assert!(list.date.is_none());
    assert_eq!(session.project().unwrap().report_info.report_date, "");
}

/// T3.9: an issued report stores the date in the same undo step as the lock and the record.
#[test]
fn issued_report_stores_the_report_date_in_its_one_undo_step() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    session.set_fixed_local_date(Some("2026-10-13"));
    let before = session.project().unwrap().clone();
    let issued = ExportRequest {
        issued: true,
        ..request(ExportFormat::CheckSheetPdf, Language::En)
    };
    let start = take_snapshot(&mut session, issued).unwrap();
    assert!(start.date.is_none(), "nothing is stored before the write");
    assert_eq!(session.project().unwrap(), &before);
    let info = start.issued.unwrap();
    assert_eq!(info.report_date, "2026-10-13");
    assert_eq!(start.snapshot.project.report_info.report_date, "2026-10-13");
    let template = report_template().unwrap();
    session
        .record_issued_report(
            start.snapshot.state,
            &info,
            Command::RecordIssuedReport {
                kind: ReportKind::CheckSheet,
                template: TemplateRef {
                    id: template.id().to_owned(),
                    version: template.version(),
                },
                format: ReportFormat::Pdf,
                output_sha256: Sha256Hex::parse(&"c".repeat(64)).unwrap(),
            },
        )
        .unwrap()
        .expect("recorded");
    assert_eq!(
        session.project().unwrap().report_info.report_date,
        "2026-10-13"
    );
    session.undo().unwrap();
    assert_eq!(session.project().unwrap(), &before);
}

#[test]
fn issued_report_locks_fills_and_records_in_one_undo_step() {
    let Some(tiles) = service() else { return };
    let mut session = session(&tiles);
    // The user cleared company and inspector; the issued export fills them.
    session
        .execute(Command::UpdateReportInfo {
            values: vec![
                ReportInfoField::CompanyName(String::new()),
                ReportInfoField::InspectedBy("  ".into()),
                ReportInfoField::ReportNumber("IR-1".into()),
            ],
        })
        .unwrap();
    let before = session.project().unwrap().clone();
    assert!(!before.is_numbering_locked());

    let snapshot = session.export_snapshot(false).unwrap();
    let filled = session.issued_report_info(&snapshot.project.report_info);
    assert_eq!(filled.company_name, "Muster GmbH");
    assert_eq!(filled.inspected_by, "Anna");
    assert_eq!(filled.report_number, "IR-1", "set fields stay");

    let template = report_template().unwrap();
    assert_eq!(template.id(), REPORT_TEMPLATE);
    let record = |sha: &str| Command::RecordIssuedReport {
        kind: ReportKind::InspectionReport,
        template: TemplateRef {
            id: template.id().to_owned(),
            version: template.version(),
        },
        format: ReportFormat::Pdf,
        output_sha256: Sha256Hex::parse(&sha.repeat(64)).unwrap(),
    };
    let patched = session
        .record_issued_report(snapshot.state, &filled, record("a"))
        .unwrap()
        .expect("unchanged project is recorded");
    assert!(!patched.patch.is_empty());
    let project = session.project().unwrap();
    assert!(project.is_numbering_locked());
    assert_eq!(project.report_info, filled);
    assert_eq!(project.issued_reports.len(), 1);
    let issued = &project.issued_reports[0];
    assert_eq!(issued.kind, ReportKind::InspectionReport);
    assert_eq!(issued.format, ReportFormat::Pdf);
    assert_eq!(issued.template.id, REPORT_TEMPLATE);
    assert_eq!(issued.output_sha256.to_hex(), "a".repeat(64));
    assert_eq!(issued.revision, project.current_revision);

    // One undo removes lock, filled info and record together.
    session.undo().unwrap();
    assert_eq!(session.project().unwrap(), &before);

    // A project that changed after the snapshot is not recorded.
    let stale = session.export_snapshot(false).unwrap().state;
    session
        .execute(Command::UpdateReportInfo {
            values: vec![ReportInfoField::Remarks("changed".into())],
        })
        .unwrap();
    let changed = session.project().unwrap().clone();
    assert!(
        session
            .record_issued_report(stale, &filled, record("b"))
            .unwrap()
            .is_none()
    );
    assert_eq!(session.project().unwrap(), &changed);
}

/// Render time of the inspection report for 300 characteristics by 10 samples (T3.8). Not a
/// pass/fail check: run with
/// `cargo test --release -p dimo-desktop --test report_export -- --ignored --nocapture`.
#[test]
#[ignore = "timing measurement, run by hand in release mode"]
fn render_time_300_by_10() {
    let Some(tiles) = service() else { return };
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(&tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let commands = (0..300_u32)
        .map(|i| Command::AddCharacteristic {
            sheet,
            position: Point {
                x: f64::from(20 + (i % 30) * 25),
                y: f64::from(20 + (i / 30) * 40),
            },
            anchor: Point { x: 10.0, y: 10.0 },
            region: None,
            values: vec![
                FieldValue::RequirementText(format!("{i} ±0.1")),
                FieldValue::Nominal(Some(Decimal::from(i + 1)).into()),
                FieldValue::UpperDev(Some(Decimal::new(1, 1)).into()),
                FieldValue::LowerDev(Some(Decimal::new(-1, 1)).into()),
            ],
            insert_after: None,
        })
        .collect();
    session.execute(Command::Batch { commands }).unwrap();
    session.execute(Command::AddSamples { count: 10 }).unwrap();
    let project = session.project().unwrap();
    let results = project
        .samples
        .iter()
        .flat_map(|s| {
            project.characteristics.iter().map(|c| ResultEntry {
                sample: s.id,
                characteristic: c.id,
                text: c.nominal.unwrap_or_default().to_string(),
                comment: None,
            })
        })
        .collect();
    session.execute(Command::SetResults { results }).unwrap();
    let snapshot = session.export_snapshot(false).unwrap();
    for format in [
        ExportFormat::InspectionReportPdf,
        ExportFormat::InspectionReportXlsx,
    ] {
        let request = request(format, Language::En);
        let start = std::time::Instant::now();
        let bytes = render(&snapshot, &request, None).unwrap();
        eprintln!(
            "{format:?}: {} ms, {} bytes",
            start.elapsed().as_millis(),
            bytes.len()
        );
    }
}
