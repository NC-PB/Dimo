//! Report model and the bundled dimo-default templates (T3.6, FR-EXP-04, FR-EXP-06, FR-EXP-11,
//! M3 decisions 10, 12, 14 and 15): data JSON snapshots, the published data schema, column
//! blocks, large reports and byte identical renders.
//!
//! The extracted PDF text snapshots need PDFium (`dimo-pdf`), which `dimo-report` may not use;
//! they live in `crates/dimo-cli/tests/report_text.rs`.
//!
//! Set `DIMO_REPORT_SAMPLES_DIR=<dir>` to also write the rendered sample PDFs there for a visual
//! check.

#![allow(clippy::expect_used, clippy::unwrap_used)] // Test crate; rust.md allows them in tests.

#[path = "common/fixture.rs"]
mod fixture;

use std::path::PathBuf;
use std::time::Instant;

use dimo_core::Language;
use dimo_core::{Project, ReportKind, Timestamp};
use dimo_report::model::{CellVerdict, GroupVerdict};
use dimo_report::{
    MAX_BLANK_COLUMNS, ReportDate, ReportError, ReportModel, ReportOptions, TemplateSet,
    data_schema_json, render_report,
};

fn bundled() -> TemplateSet {
    TemplateSet::bundled("dimo-default").expect("bundled template set")
}

fn model(project: &Project, kind: ReportKind, language: Language) -> ReportModel {
    ReportModel::build(project, &ReportOptions::new(kind, language)).expect("model")
}

fn pretty(model: &ReportModel) -> String {
    let mut text = serde_json::to_string_pretty(&model.to_json().expect("json")).expect("text");
    text.push('\n');
    text
}

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it")).join("../..")
}

/// Writes a rendered PDF to `DIMO_REPORT_SAMPLES_DIR` when it is set.
fn keep_sample(name: &str, pdf: &[u8]) {
    if let Some(dir) = std::env::var_os("DIMO_REPORT_SAMPLES_DIR") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("samples dir");
        std::fs::write(dir.join(name), pdf).expect("write sample");
    }
}

/// Number of pages of a PDF written by Typst: the `/Count` of its page tree.
fn page_count(pdf: &[u8]) -> usize {
    let text = String::from_utf8_lossy(pdf);
    text.split("/Count ")
        .skip(1)
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn bundled_set_has_both_documents_and_labels() {
    let set = bundled();
    assert_eq!(set.id(), "dimo-default");
    assert_eq!(set.version(), 1);
    let names: Vec<&str> = set.documents().iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["check-sheet", "inspection-report"]);
    assert_eq!(set.labels_file(), Some("labels.toml"));
}

/// Dotted keys of all leaves of a TOML value.
fn keys(value: &toml::Value, prefix: &str, out: &mut Vec<String>) {
    if let toml::Value::Table(table) = value {
        for (k, v) in table {
            keys(v, &format!("{prefix}{k}."), out);
        }
    } else {
        out.push(prefix.trim_end_matches('.').to_owned());
    }
}

/// D-32: the label file has the same keys in English and German, and a label for every code
/// the data can hold.
#[test]
fn labels_cover_both_languages_and_every_code() {
    let set = bundled();
    let text = std::str::from_utf8(&set.files()["labels.toml"]).expect("utf-8");
    let labels: toml::Table = toml::from_str(text).expect("labels parse");
    let (mut en, mut de) = (Vec::new(), Vec::new());
    keys(&labels["en"], "", &mut en);
    keys(&labels["de"], "", &mut de);
    assert_eq!(en, de);
    for code in ["pass", "fail", "open", "not_evaluated", "missing"] {
        assert!(en.contains(&format!("verdicts.{code}")), "{code}");
    }
    for code in [
        "explicit",
        "fit",
        "drawing_rule",
        "general",
        "decimal_rule",
        "custom_table",
        "no_tolerance_defined",
        "manual",
    ] {
        assert!(en.contains(&format!("rules.{code}")), "{code}");
    }
    for code in ["ok", "not_ok"] {
        assert!(en.contains(&format!("attributes.{code}")), "{code}");
    }
}

#[test]
fn inspection_report_data_english() {
    let model = model(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::En,
    );
    insta::assert_snapshot!(pretty(&model));
}

#[test]
fn inspection_report_data_german() {
    let model = model(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::De,
    );
    insta::assert_snapshot!(pretty(&model));
}

#[test]
fn check_sheet_data_with_blank_columns() {
    let mut options = ReportOptions::new(ReportKind::CheckSheet, Language::De);
    options.blank_columns = 4;
    let model = ReportModel::build(&fixture::without_samples(), &options).expect("model");
    insta::assert_snapshot!(pretty(&model));
}

#[test]
fn check_sheet_with_samples_prints_serials_and_no_results() {
    let model = model(&fixture::project(), ReportKind::CheckSheet, Language::En);
    let serials: Vec<&str> = model.samples.iter().map(|s| s.serial.as_str()).collect();
    assert_eq!(serials, ["S-001", "S-002", "S-003"]);
    assert!(!model.flags.blank_samples && !model.flags.open_results);
    // The fixture has failing results; a check sheet reports none of them.
    assert_eq!(model.summary.verdict, GroupVerdict::Open);
    assert_eq!(model.summary.counts.missing, "24");
    assert_eq!(model.summary.counts.fail, "0");
    assert!(
        model
            .samples
            .iter()
            .all(|s| s.verdict == GroupVerdict::Open)
    );
    assert!(
        model
            .rows
            .iter()
            .all(|r| r.verdict == GroupVerdict::Open && r.counts.missing == "3")
    );
    for row in &model.rows {
        assert_eq!(row.cells.len(), 3);
        assert!(
            row.cells
                .iter()
                .all(|c| c.value.is_empty() && c.verdict == CellVerdict::Missing)
        );
    }
}

/// Decision 5: only inspected, not rejected characteristics, in number order.
#[test]
fn rows_are_the_measured_characteristics_in_number_order() {
    let model = model(
        &fixture::project(),
        ReportKind::InspectionReport,
        Language::En,
    );
    let numbers: Vec<&str> = model.rows.iter().map(|r| r.number.as_str()).collect();
    assert_eq!(numbers, ["1", "2", "3", "4", "5", "6", "8", "10"]);
    assert_eq!(model.summary.verdict, GroupVerdict::Fail);
    assert!(model.flags.draft_limits);
}

#[test]
fn blank_columns_are_checked() {
    for blank in [0, MAX_BLANK_COLUMNS + 1] {
        let mut options = ReportOptions::new(ReportKind::CheckSheet, Language::En);
        options.blank_columns = blank;
        assert!(matches!(
            ReportModel::build(&fixture::without_samples(), &options),
            Err(ReportError::InvalidOptions(_))
        ));
    }
}

/// Decision 14: the report date, else the project modification date, never the clock.
#[test]
fn report_date_comes_from_the_project() {
    let mut project = fixture::project();
    let built = model(&project, ReportKind::InspectionReport, Language::En);
    assert_eq!(built.header.report_date, "2026-10-09");
    assert_eq!(
        built.date(),
        Some(ReportDate {
            year: 2026,
            month: 10,
            day: 9
        })
    );

    project.report_info.report_date.clear();
    let mut options = ReportOptions::new(ReportKind::InspectionReport, Language::En);
    assert_eq!(
        ReportModel::build(&project, &options)
            .expect("model")
            .header
            .report_date,
        ""
    );
    options.fallback_date = Some(Timestamp::parse("2026-10-11T17:45:00Z").expect("time"));
    let built = ReportModel::build(&project, &options).expect("model");
    assert_eq!(built.header.report_date, "2026-10-11");
    assert_eq!(built.signatures.inspected_by.date, "2026-10-11");
    assert_eq!(built.signatures.approved_by.date, "");
}

#[test]
fn committed_schema_is_current() {
    let path = repo_root().join("docs/schema/report-data.schema.json");
    let generated = data_schema_json();
    if std::env::var_os("DIMO_UPDATE_SCHEMA").is_some() {
        std::fs::write(&path, &generated).expect("write schema");
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == generated,
        "docs/schema/report-data.schema.json is stale. Regenerate with ./scripts/regen.sh or \
         DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-report --test report, update \
         data/templates/README.md, and increase DATA_FORMAT_VERSION if a field was removed or \
         changed meaning"
    );
}

#[test]
fn data_matches_the_published_schema() {
    let schema: serde_json::Value = serde_json::from_str(&data_schema_json()).expect("schema");
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:report-data.schema.json", schema)
        .expect("resource");
    let index = compiler
        .compile("urn:dimo:report-data.schema.json", &mut schemas)
        .expect("compile");
    for (project, kind) in [
        (fixture::project(), ReportKind::InspectionReport),
        (fixture::project(), ReportKind::CheckSheet),
        (fixture::without_samples(), ReportKind::CheckSheet),
        (fixture::without_samples(), ReportKind::InspectionReport),
    ] {
        for language in [Language::En, Language::De] {
            let data = model(&project, kind, language).to_json().expect("json");
            if let Err(error) = schemas.validate(&data, index) {
                panic!("report data does not match the schema: {error:#}");
            }
        }
    }
}

/// Both documents in both languages render, twice byte identical (FR-EXP-11).
#[test]
fn both_documents_render_byte_identical_in_both_languages() {
    let set = bundled();
    for (project, name) in [
        (fixture::project(), "fixture"),
        (fixture::without_samples(), "blank"),
    ] {
        for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
            for language in [Language::En, Language::De] {
                let model = model(&project, kind, language);
                let first = render_report(&set, &model).expect("render");
                typst::comemo::evict(0);
                let second = render_report(&set, &model).expect("render");
                assert!(first == second, "{kind:?} {language:?} differs");
                assert!(first.starts_with(b"%PDF-"));
                keep_sample(
                    &format!(
                        "{name}_{}_{}.pdf",
                        dimo_report::document_name(kind),
                        language.code()
                    ),
                    &first,
                );
            }
        }
    }
}

/// Decision 12: more than six samples repeat the table in blocks of sample columns.
#[test]
fn twelve_samples_render_in_two_column_blocks() {
    let project = fixture::with_samples(12);
    let set = bundled();
    for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
        let model = model(&project, kind, Language::En);
        let blocks: Vec<(u32, u32)> = model.column_blocks.iter().map(|b| (b.from, b.to)).collect();
        assert_eq!(blocks, [(0, 6), (6, 12)]);
        let pdf = render_report(&set, &model).expect("render");
        // Each block starts on its own page.
        assert!(
            page_count(&pdf) >= 2,
            "{kind:?}: {} pages",
            page_count(&pdf)
        );
        keep_sample(
            &format!("twelve_{}_en.pdf", dimo_report::document_name(kind)),
            &pdf,
        );
    }
}

/// 300 rows by 10 samples render on several pages (header rows repeat, two column blocks).
#[test]
fn large_report_renders_on_several_pages() {
    let project = fixture::large(300, 10);
    let set = bundled();
    for kind in [ReportKind::CheckSheet, ReportKind::InspectionReport] {
        let model = model(&project, kind, Language::De);
        assert_eq!(model.rows.len(), 300);
        assert_eq!(model.column_blocks.len(), 2);
        let start = Instant::now();
        let pdf = render_report(&set, &model).expect("render");
        let elapsed = start.elapsed();
        let pages = page_count(&pdf);
        assert!(pages >= 10, "{kind:?}: {pages} pages");
        tracing::info!("{kind:?} 300 x 10: {pages} pages in {elapsed:?}");
        keep_sample(
            &format!("large_{}_de.pdf", dimo_report::document_name(kind)),
            &pdf,
        );
    }
}
