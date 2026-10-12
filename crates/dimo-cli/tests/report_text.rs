//! Text of the rendered check sheet and inspection report, extracted with PDFium (T3.6,
//! FR-EXP-04, M3 decision 14). `dimo-report` may not depend on `dimo-pdf`, so this test lives
//! with the CLI, which may use every crate (rust.md).
//!
//! Each snapshot lists the text lines of every page, top to bottom, runs left to right. Verdict
//! marks are shapes, not text, so only values and words appear.
//!
//! Skips without PDFium, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap and expect are fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

#[path = "../../dimo-report/tests/common/fixture.rs"]
mod fixture;

use std::fmt::Write as _;

use dimo_core::{Project, ReportKind};
use dimo_pdf::{PdfEngine, PdfError, TextRun};
use dimo_report::{Language, ReportModel, ReportOptions, TemplateSet, render_report};

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// Text runs grouped into lines: runs whose vertical centers are within 2 units form a line.
fn lines(runs: &[TextRun]) -> Vec<String> {
    let mut runs: Vec<&TextRun> = runs.iter().filter(|r| !r.text.trim().is_empty()).collect();
    let center = |r: &TextRun| r.bbox.y + r.bbox.height / 2.0;
    runs.sort_by(|a, b| center(a).total_cmp(&center(b)));
    let mut groups: Vec<Vec<&TextRun>> = Vec::new();
    for run in runs {
        match groups.last_mut() {
            Some(group) if (center(run) - center(group[0])).abs() < 2.0 => group.push(run),
            _ => groups.push(vec![run]),
        }
    }
    groups
        .into_iter()
        .map(|mut group| {
            group.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
            group
                .iter()
                .map(|r| r.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn pdf_text(engine: &PdfEngine, pdf: Vec<u8>) -> String {
    let doc = engine.open(pdf).unwrap();
    let mut text = String::new();
    for sheet in 0..doc.sheet_count() {
        let _ = writeln!(text, "--- page {} ---", sheet + 1);
        for line in lines(&doc.text_runs(sheet).unwrap()) {
            let _ = writeln!(text, "{line}");
        }
    }
    text
}

fn render(project: &Project, kind: ReportKind, language: Language) -> Vec<u8> {
    let set = TemplateSet::bundled("dimo-default").unwrap();
    let model = ReportModel::build(project, &ReportOptions::new(kind, language)).unwrap();
    render_report(&set, &model).unwrap()
}

#[test]
fn report_text_snapshots() {
    let Some(engine) = engine() else { return };
    let project = fixture::project();
    for (kind, name) in [
        (ReportKind::CheckSheet, "check_sheet"),
        (ReportKind::InspectionReport, "inspection_report"),
    ] {
        for language in [Language::En, Language::De] {
            let text = pdf_text(&engine, render(&project, kind, language));
            insta::assert_snapshot!(format!("{name}_{}", language.code()), text);
        }
    }
}

#[test]
fn check_sheet_without_samples_has_blank_columns() {
    let Some(engine) = engine() else { return };
    let text = pdf_text(
        &engine,
        render(
            &fixture::without_samples(),
            ReportKind::CheckSheet,
            Language::En,
        ),
    );
    assert!(text.contains("Part 1 Part 2 Part 3"), "{text}");
    assert!(!text.contains("Part 4"), "{text}");
    assert!(text.contains("Write the serial number"), "{text}");
}
