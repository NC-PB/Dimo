//! XLSX check sheet and inspection report (T3.7, FR-EXP-04, FR-EXP-11, M3 decisions 12 to 14):
//! snapshots of the unzipped XML parts, byte identical output, number cells, verdict marks,
//! print setup and a large report.

#![allow(clippy::expect_used, clippy::unwrap_used)] // Test crate; rust.md allows them in tests.

#[path = "common/fixture.rs"]
mod fixture;

use std::fmt::Write as _;
use std::io::Read;
use std::time::Instant;

use dimo_core::{Project, ReportKind};
use dimo_report::{Language, ReportError, ReportModel, ReportOptions, TemplateSet, render_xlsx};

fn bundled() -> TemplateSet {
    TemplateSet::bundled("dimo-default").expect("bundled template set")
}

fn xlsx(project: &Project, kind: ReportKind, language: Language) -> Vec<u8> {
    let model = ReportModel::build(project, &ReportOptions::new(kind, language)).expect("model");
    render_xlsx(&bundled(), &model).expect("xlsx")
}

fn archive(bytes: &[u8]) -> zip::ZipArchive<std::io::Cursor<&[u8]>> {
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip")
}

fn read_entry(bytes: &[u8], name: &str) -> String {
    let mut text = String::new();
    archive(bytes)
        .by_name(name)
        .unwrap_or_else(|_| panic!("entry {name}"))
        .read_to_string(&mut text)
        .expect("utf-8 entry");
    text
}

/// The XML parts that carry content, as one text for a snapshot.
fn parts(bytes: &[u8]) -> String {
    let mut out = String::new();
    for name in [
        "docProps/core.xml",
        "xl/workbook.xml",
        "xl/styles.xml",
        "xl/sharedStrings.xml",
        "xl/worksheets/sheet1.xml",
    ] {
        let _ = writeln!(out, "=== {name}\n{}", read_entry(bytes, name));
    }
    out
}

#[test]
fn inspection_report_english() {
    let bytes = xlsx(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::En,
    );
    insta::assert_snapshot!(parts(&bytes));
}

#[test]
fn inspection_report_german() {
    let bytes = xlsx(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::De,
    );
    insta::assert_snapshot!(parts(&bytes));
}

#[test]
fn check_sheet_with_blank_columns_english() {
    let bytes = xlsx(
        &fixture::without_samples(),
        ReportKind::CheckSheet,
        Language::En,
    );
    insta::assert_snapshot!(parts(&bytes));
}

#[test]
fn check_sheet_with_samples_german() {
    let bytes = xlsx(&fixture::project(), ReportKind::CheckSheet, Language::De);
    insta::assert_snapshot!(parts(&bytes));
}

#[test]
fn sheet_names_come_from_the_label_file() {
    let project = fixture::project();
    let en = xlsx(&project, ReportKind::InspectionReport, Language::En);
    let de = xlsx(&project, ReportKind::InspectionReport, Language::De);
    assert!(read_entry(&en, "xl/workbook.xml").contains(r#"name="Inspection report""#));
    assert!(read_entry(&de, "xl/workbook.xml").contains(r#"name="Prüfbericht""#));
    let check = xlsx(&project, ReportKind::CheckSheet, Language::De);
    assert!(read_entry(&check, "xl/workbook.xml").contains(r#"name="Prüfblatt""#));
}

#[test]
fn exports_are_byte_identical() {
    for project in [fixture::project(), fixture::without_samples()] {
        for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
            for language in [Language::En, Language::De] {
                let first = xlsx(&project, kind, language);
                let second = xlsx(&project, kind, language);
                assert_eq!(first, second, "{kind:?} {language:?}");
            }
        }
    }
}

#[test]
fn zip_entries_have_a_fixed_time() {
    let bytes = xlsx(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::En,
    );
    let mut zip = archive(&bytes);
    for i in 0..zip.len() {
        let file = zip.by_index(i).expect("entry");
        let time = file.last_modified().expect("time");
        assert_eq!((time.hour(), time.minute(), time.second()), (0, 0, 0));
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (1980, 1, 1),
            "{}",
            file.name()
        );
    }
}

/// Values keep their typed decimals as number cells; the number format carries the scale and
/// the verdict mark, so the cells read in greyscale. German values are stored with a point.
#[test]
fn result_cells_are_numbers_with_a_verdict_mark_in_the_number_format() {
    for language in [Language::En, Language::De] {
        let bytes = xlsx(&fixture::project(), ReportKind::InspectionReport, language);
        let styles = read_entry(&bytes, "xl/styles.xml");
        assert!(styles.contains("&quot;\u{2713}&quot;"), "{styles}");
        assert!(styles.contains("&quot;\u{2717}&quot;"), "{styles}");
        assert!(styles.contains("&quot;\u{25CB}&quot;"), "{styles}");
        let sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
        // 30.010 is stored as the number 30.01 (shown with three decimals by its format).
        assert!(sheet.contains("<v>30.01</v>"), "{language:?}");
        assert!(!sheet.contains("30,01"), "{language:?}");
    }
}

#[test]
fn print_setup_is_landscape_a4_with_repeated_header_rows() {
    let bytes = xlsx(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::En,
    );
    let sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains(r#"orientation="landscape""#));
    assert!(sheet.contains(r#"paperSize="9""#));
    assert!(sheet.contains(r#"fitToPage="1""#) && sheet.contains(r#"fitToHeight="0""#));
    let workbook = read_entry(&bytes, "xl/workbook.xml");
    assert!(workbook.contains("_xlnm.Print_Titles"));
    assert!(workbook.contains("$A:$H"), "{workbook}");
}

#[test]
fn twelve_samples_print_in_two_column_blocks() {
    let bytes = xlsx(
        &fixture::with_samples(12),
        ReportKind::InspectionReport,
        Language::En,
    );
    let sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
    // A break before column O (index 14): samples 1 to 6 in columns I to N, the rest follow.
    assert!(sheet.contains(r#"<colBreaks count="1""#), "{sheet}");
    assert!(sheet.contains(r#"<brk id="14""#));
}

#[test]
fn a_template_without_labels_is_refused() {
    let manifest =
        "id = \"bare\"\nversion = 1\n[[documents]]\nname = \"check-sheet\"\nmain = \"a.typ\"\n";
    let set = TemplateSet::from_files([
        ("template.toml".to_string(), manifest.as_bytes().to_vec()),
        ("a.typ".to_string(), Vec::new()),
    ])
    .expect("set");
    let model = ReportModel::build(
        &fixture::project(),
        &ReportOptions::new(ReportKind::CheckSheet, Language::En),
    )
    .expect("model");
    assert!(matches!(
        render_xlsx(&set, &model),
        Err(ReportError::InvalidTemplate(_))
    ));
}

#[test]
fn large_report_300_rows_by_10_samples() {
    let project = fixture::large(300, 10);
    let start = Instant::now();
    let first = xlsx(&project, ReportKind::InspectionReport, Language::De);
    let elapsed = start.elapsed();
    assert_eq!(
        first,
        xlsx(&project, ReportKind::InspectionReport, Language::De)
    );
    let sheet = read_entry(&first, "xl/worksheets/sheet1.xml");
    // 300 data rows plus the header block, summary, table head and signature rows.
    let rows = sheet.matches("<row ").count();
    assert!((300..360).contains(&rows), "{rows} rows");
    // Ten samples: blocks of six, so one break before the seventh sample column.
    assert!(sheet.contains(r#"<colBreaks count="1""#));
    // 300 rows x 10 samples are number cells.
    assert!(sheet.matches("<v>").count() >= 3000);
    assert!(elapsed.as_secs() < 5, "took {elapsed:?}");
    let check = xlsx(&project, ReportKind::CheckSheet, Language::En);
    assert!(read_entry(&check, "xl/worksheets/sheet1.xml").contains(r#"<row r="1""#));
}
