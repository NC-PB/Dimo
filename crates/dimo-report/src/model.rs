//! The report model: everything a check sheet or an inspection report prints, already formatted
//! as text (M3 T3.6, decisions 10, 12, 14 and 15, FR-EXP-04, FR-RVC-04).
//!
//! [`ReportModel::build`] reads a project and its evaluation from `dimo-core` and produces the
//! data the template reads from `/data.json`. Every value a template prints is a string:
//! numbers keep the scale they were typed with and use the decimal separator of the report
//! language, dates are `YYYY-MM-DD`, verdicts, kinds and rules are stable codes the template
//! maps to its own labels (D-32). Templates never compute with numbers (rule 5) and never decide
//! pass or fail (rule 2). The only numbers in the data are the sample index ranges of the column
//! blocks, which a template uses to slice the sample and cell lists.
//!
//! The format is documented in `data/templates/README.md`; its JSON Schema is
//! `docs/schema/report-data.schema.json` ([`data_schema_json`]).

use std::collections::HashMap;

use dimo_core::{
    AttributeValue, CharId, Characteristic, CharacteristicKind, Classification, DerivationRule,
    Language, MeasurementResult, MeasurementState, OverallVerdict, Project, ReportKind, ResultType,
    ResultValue, RowState, SampleId, TableRef, Timestamp, Unit, Verdict, VerdictCounts,
    measured_characteristics, measurement_state,
};
use schemars::JsonSchema;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

use crate::{ReportDate, ReportError};

/// Version of the data format. Increased when a field is removed or changes meaning; templates
/// can check it (M7 custom templates).
pub const DATA_FORMAT_VERSION: u32 = 1;

/// Most sample columns in one column block (M3 decision 12: "about 6 columns"). More samples
/// repeat the table in blocks, with the characteristic columns repeated.
pub const SAMPLES_PER_BLOCK: usize = 6;

/// Blank result columns of a check sheet for a project without samples, by default.
pub const DEFAULT_BLANK_COLUMNS: u8 = 3;

/// Most blank result columns of a check sheet (decision 12: 1 to 10).
pub const MAX_BLANK_COLUMNS: u8 = 10;

/// Length of the short SHA-256 printed in the page header (decision 12).
const SHA256_SHORT_LEN: usize = 16;

/// What to build the model for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportOptions {
    /// Check sheet or inspection report.
    pub kind: ReportKind,
    /// Report language (D-32): decides the decimal separator.
    pub language: Language,
    /// Number of blank result columns of a check sheet when the project has no samples,
    /// 1 to [`MAX_BLANK_COLUMNS`]. Ignored otherwise.
    pub blank_columns: u8,
    /// Date used when the report info has no report date: the project modification date from
    /// the project file (decision 14). Never the clock. Its UTC calendar date is printed, so a
    /// file saved late in the evening in Europe may show the next day.
    pub fallback_date: Option<Timestamp>,
}

impl ReportOptions {
    /// Options for `kind` in `language` with the default blank columns and no fallback date.
    pub fn new(kind: ReportKind, language: Language) -> Self {
        Self {
            kind,
            language,
            blank_columns: DEFAULT_BLANK_COLUMNS,
            fallback_date: None,
        }
    }
}

// Schema of `ReportModel::language`, a `dimo_core::Language` in the data. It only describes
// the published data schema, which keeps this name and these descriptions; never built.
/// Report language as written into the data (`en`, `de`).
#[derive(JsonSchema)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum LanguageCode {
    /// English: decimal point.
    En,
    /// German: decimal comma.
    De,
}

/// Verdict of one cell (FR-MEA-03, decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CellVerdict {
    /// Within the limits, or attribute OK.
    Pass,
    /// Outside a limit, or attribute not OK.
    Fail,
    /// A value for a characteristic without limits.
    NotEvaluated,
    /// No result.
    Missing,
}

/// Verdict of a row, a sample or the whole report (decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupVerdict {
    /// Every cell passes.
    Pass,
    /// At least one cell fails.
    Fail,
    /// No cell fails, but some are missing or not evaluated, or there are no cells.
    Open,
}

// The codes are the `DerivationRule::id`s (`rule_code`). Plain comment: the doc comment below
// is the description in the published data schema.
/// Which rule produced the limits (FR-TOL-08), as in `project.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleCode {
    /// Tolerance written on the callout.
    Explicit,
    /// Fit designation and fit table.
    Fit,
    /// Drawing specific rule.
    DrawingRule,
    /// General tolerance standard.
    General,
    /// Decimal place rule.
    DecimalRule,
    /// Custom tolerance table.
    CustomTable,
    /// No tolerance and no general rule: no limits.
    NoToleranceDefined,
    /// Limits typed by hand.
    Manual,
}

/// How a row is measured: the effective result type (decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeasureType {
    /// A measured value compared with the limits.
    Variable,
    /// OK or not OK.
    Attribute,
}

/// What a result cell holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CellType {
    /// No result (and every cell of a check sheet).
    None,
    /// A measured value; `value` is the formatted number.
    Variable,
    /// An attribute result; `value` is `ok` or `not_ok`.
    Attribute,
}

/// Number of cells per verdict, as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Counts {
    /// Cells that pass.
    pub pass: String,
    /// Cells that fail.
    pub fail: String,
    /// Cells with a value but no limits.
    pub not_evaluated: String,
    /// Cells without a result.
    pub missing: String,
}

/// Header printed on every page (decision 12, FR-RVC-04, FR-DOC-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Header {
    /// Issuing company (report info).
    pub company: String,
    /// Part number.
    pub part_number: String,
    /// Part name.
    pub part_name: String,
    /// Customer.
    pub customer: String,
    /// Order or purchase reference.
    pub order_reference: String,
    /// File name of the current drawing revision.
    pub drawing_file: String,
    /// Revision label of the current drawing, empty if unknown.
    pub revision: String,
    /// Full SHA-256 of the drawing file (64 hex digits), printed on the first page.
    pub sha256: String,
    /// First 16 hex digits of the SHA-256, printed in the page header.
    pub sha256_short: String,
    /// Report number.
    pub report_number: String,
    /// Report date `YYYY-MM-DD`: the report info date, else the project modification date,
    /// else empty (decision 14).
    pub report_date: String,
}

/// Overall result of the report (inspection report summary block).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Summary {
    /// Overall verdict.
    pub verdict: GroupVerdict,
    /// Cells per verdict over all rows and samples.
    pub counts: Counts,
    /// Number of measured characteristics (rows).
    pub characteristics: String,
    /// Number of samples (real samples, not blank check sheet columns).
    pub samples: String,
}

/// One result column: a sample, or a blank column of a check sheet without samples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SampleColumn {
    /// Position from 1, as text.
    pub index: String,
    /// Serial number; empty for a blank column.
    pub serial: String,
    /// Measuring date `YYYY-MM-DD` or empty.
    pub date: String,
    /// Who measured the part.
    pub inspector: String,
    /// Gauges and instruments used.
    pub gauges: String,
    /// Comment.
    pub comment: String,
    /// True for a blank check sheet column (the project has no samples).
    pub blank: bool,
    /// Verdict of this part over all rows.
    pub verdict: GroupVerdict,
    /// Cells per verdict of this part.
    pub counts: Counts,
}

/// A range of sample columns printed together (decision 12). `from` and `to` index the sample
/// list and each row's cells (`to` exclusive); `first` and `last` are the printed positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ColumnBlock {
    /// Index of the first sample of the block, from 0.
    pub from: u32,
    /// Index after the last sample of the block.
    pub to: u32,
    /// Position of the first sample, from 1, as text; empty if the block has no samples.
    pub first: String,
    /// Position of the last sample, as text; empty if the block has no samples.
    pub last: String,
}

/// One result cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Cell {
    /// What the cell holds.
    #[serde(rename = "type")]
    pub cell_type: CellType,
    /// Formatted number (variable), `ok` or `not_ok` (attribute), or empty.
    pub value: String,
    /// Verdict of the cell.
    pub verdict: CellVerdict,
    /// Comment of the result.
    pub comment: String,
}

/// One measured characteristic (decision 5), in number order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Row {
    /// Display number (balloon number).
    pub number: String,
    /// Kind of characteristic.
    pub kind: CharacteristicKind,
    /// Requirement text as on the drawing.
    pub requirement: String,
    /// Number of features (`4X`), as text.
    pub quantity: String,
    /// Nominal value, formatted, or empty.
    pub nominal: String,
    /// Upper deviation with sign (`+0.1`), or empty.
    pub upper_deviation: String,
    /// Lower deviation with sign (`-0.2`), or empty.
    pub lower_deviation: String,
    /// Lower limit, formatted, or empty.
    pub lower_limit: String,
    /// Upper limit, formatted, or empty.
    pub upper_limit: String,
    /// Unit symbol: `mm`, `in`, `°`, or empty.
    pub unit: String,
    /// Fit designation such as `H7`, or empty.
    pub fit: String,
    /// Which rule produced the limits; `null` if none was recorded.
    pub rule: Option<RuleCode>,
    /// Source of the rule without words, such as `ISO 2768-1 m`, `ISO 286 H7` or `±0.05`;
    /// empty for rules without a table or parameter.
    pub rule_detail: String,
    /// True if the limits came from a draft tolerance table (D-43).
    pub draft: bool,
    /// Inspection method.
    pub method: String,
    /// Gauge or instrument.
    pub gauge: String,
    /// Classification; `none` if not classified.
    pub classification: Classification,
    /// Effective result type.
    pub result_type: MeasureType,
    /// Verdict of the row over all samples.
    pub verdict: GroupVerdict,
    /// Cells per verdict of this row.
    pub counts: Counts,
    /// One cell per entry of `samples`, in the same order.
    pub cells: Vec<Cell>,
}

/// One line of the signature block (decision 15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Signature {
    /// Name from the report info, or empty.
    pub name: String,
    /// Date `YYYY-MM-DD`, or empty for a date written by hand.
    pub date: String,
}

/// Signature block: who inspected and who approved (decision 15). No digital signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Signatures {
    /// Inspected by: name and the report date.
    pub inspected_by: Signature,
    /// Approved by: name; the date stays empty, the approver writes it with the signature.
    pub approved_by: Signature,
}

/// Facts the template shows as notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Flags {
    /// At least one row has limits from a draft table (D-43): print the footnote.
    pub draft_limits: bool,
    /// Inspection report whose overall verdict is open: results missing or not evaluated, or
    /// no samples.
    pub open_results: bool,
    /// The result columns are blank check sheet columns (no samples in the project).
    pub blank_samples: bool,
}

/// The data of one report, serialized as the template's `/data.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(
    title = "Dimo report data",
    description = "Data of a check sheet or inspection \
    report as passed to report templates (/data.json). Every printed value is text."
)]
pub struct ReportModel {
    /// Data format version, [`DATA_FORMAT_VERSION`].
    pub format_version: u32,
    /// Which document.
    pub document: ReportKind,
    /// Report language; also passed to the template as `sys.inputs.lang`.
    #[schemars(with = "LanguageCode")]
    pub language: Language,
    /// Page header.
    pub header: Header,
    /// Overall result.
    pub summary: Summary,
    /// Result columns in order.
    pub samples: Vec<SampleColumn>,
    /// Column blocks, at least one.
    pub column_blocks: Vec<ColumnBlock>,
    /// Measured characteristics in number order.
    pub rows: Vec<Row>,
    /// Remarks from the report info, line breaks kept.
    pub remarks: String,
    /// Signature block.
    pub signatures: Signatures,
    /// Notes to print.
    pub flags: Flags,
}

impl ReportModel {
    /// Builds the model of `project` with its current evaluation.
    ///
    /// Fails if `blank_columns` is outside 1 to [`MAX_BLANK_COLUMNS`].
    pub fn build(project: &Project, options: &ReportOptions) -> Result<Self, ReportError> {
        if !(1..=MAX_BLANK_COLUMNS).contains(&options.blank_columns) {
            return Err(ReportError::InvalidOptions(format!(
                "blank columns must be 1 to {MAX_BLANK_COLUMNS}, got {}",
                options.blank_columns
            )));
        }
        let fmt = Formatter {
            language: options.language,
        };
        let check_sheet = options.kind == ReportKind::CheckSheet;
        let state = if check_sheet {
            // Nothing is printed as measured on a check sheet: every cell counts as missing.
            unmeasured(measurement_state(project))
        } else {
            measurement_state(project)
        };
        let header = header(project, options);

        let blank_samples = check_sheet && project.samples.is_empty();
        let samples = if blank_samples {
            blank_columns(options.blank_columns)
        } else {
            sample_columns(project, &state)
        };

        let results: HashMap<(SampleId, CharId), &MeasurementResult> = project
            .results
            .iter()
            .map(|r| ((r.sample, r.characteristic), r))
            .collect();
        // `measurement_state` lists exactly the measured characteristics in this order.
        let rows: Vec<Row> = measured_characteristics(project)
            .into_iter()
            .zip(&state.rows)
            .map(|(c, row_state)| {
                let cells = if blank_samples {
                    samples.iter().map(|_| Cell::empty()).collect()
                } else {
                    project
                        .samples
                        .iter()
                        .zip(&row_state.cells)
                        .map(|(sample, verdict)| {
                            if check_sheet {
                                Cell::empty()
                            } else {
                                fmt.cell(results.get(&(sample.id, c.id)).copied(), *verdict)
                            }
                        })
                        .collect()
                };
                fmt.row(c, row_state, cells)
            })
            .collect();

        let column_blocks = column_blocks(samples.len());
        let info = &project.report_info;
        Ok(Self {
            format_version: DATA_FORMAT_VERSION,
            document: options.kind,
            language: options.language,
            summary: Summary {
                verdict: group_verdict(state.verdict),
                counts: counts(state.counts),
                characteristics: rows.len().to_string(),
                samples: project.samples.len().to_string(),
            },
            flags: Flags {
                draft_limits: rows.iter().any(|r| r.draft),
                open_results: !check_sheet && state.verdict == OverallVerdict::Open,
                blank_samples,
            },
            signatures: Signatures {
                inspected_by: Signature {
                    name: info.inspected_by.clone(),
                    date: header.report_date.clone(),
                },
                approved_by: Signature {
                    name: info.approved_by.clone(),
                    date: String::new(),
                },
            },
            remarks: info.remarks.clone(),
            header,
            samples,
            column_blocks,
            rows,
        })
    }

    /// The model as the JSON value passed to the template.
    pub fn to_json(&self) -> Result<serde_json::Value, ReportError> {
        Ok(serde_json::to_value(self)?)
    }

    /// The report date as a calendar date for the PDF metadata, if it is a valid
    /// `YYYY-MM-DD` date (decision 14).
    pub fn date(&self) -> Option<ReportDate> {
        parse_date(&self.header.report_date)
    }

    /// The report language.
    pub fn language(&self) -> Language {
        self.language
    }
}

/// Name of the template document that renders `kind` (`template.toml`).
pub fn document_name(kind: ReportKind) -> &'static str {
    match kind {
        ReportKind::CheckSheet => "check-sheet",
        ReportKind::InspectionReport => "inspection-report",
    }
}

/// The JSON Schema of the report data ([`ReportModel`]), keys sorted, pretty printed with a
/// final newline, as published in `docs/schema/report-data.schema.json`.
pub fn data_schema_json() -> String {
    let schema = sort_keys(schemars::schema_for!(ReportModel).to_value());
    // Serializing a `Value` cannot fail: all keys are strings.
    let mut text = serde_json::to_string_pretty(&schema).unwrap_or_default();
    text.push('\n');
    text
}

/// Object keys sorted recursively, so the schema text does not depend on map ordering.
fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, sort_keys(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_keys).collect()),
        other => other,
    }
}

impl Cell {
    fn empty() -> Self {
        Self {
            cell_type: CellType::None,
            value: String::new(),
            verdict: CellVerdict::Missing,
            comment: String::new(),
        }
    }
}

fn header(project: &Project, options: &ReportOptions) -> Header {
    let info = &project.report_info;
    let revision = project.current_revision();
    let sha256 = revision.map_or_else(String::new, |r| r.sha256.as_str().to_owned());
    let report_date = if info.report_date.is_empty() {
        options
            .fallback_date
            .as_ref()
            .and_then(|t| t.as_str().get(..10))
            .unwrap_or_default()
            .to_owned()
    } else {
        info.report_date.clone()
    };
    Header {
        company: info.company_name.clone(),
        part_number: project.info.part_number.clone(),
        part_name: project.info.part_name.clone(),
        customer: project.info.customer.clone(),
        order_reference: project.info.order_reference.clone(),
        drawing_file: revision.map_or_else(String::new, |r| r.file_name.clone()),
        revision: revision.map_or_else(String::new, |r| r.label.clone()),
        sha256_short: sha256.chars().take(SHA256_SHORT_LEN).collect(),
        sha256,
        report_number: info.report_number.clone(),
        report_date,
    }
}

/// The evaluation with every cell missing and every group open, for a check sheet: its data
/// never reports a verdict, whatever results the project holds.
fn unmeasured(mut state: MeasurementState) -> MeasurementState {
    let missing = |n: usize| VerdictCounts {
        missing: u32::try_from(n).unwrap_or(u32::MAX),
        ..VerdictCounts::default()
    };
    let (rows, samples) = (state.rows.len(), state.samples.len());
    for row in &mut state.rows {
        row.cells.iter_mut().for_each(|c| *c = Verdict::Missing);
        row.positions.iter_mut().for_each(|p| *p = None);
        row.counts = missing(samples);
        row.verdict = OverallVerdict::Open;
    }
    for sample in &mut state.samples {
        sample.counts = missing(rows);
        sample.verdict = OverallVerdict::Open;
    }
    state.counts = missing(rows.saturating_mul(samples));
    state.verdict = OverallVerdict::Open;
    state.complete_samples = 0;
    state
}

/// `count` blank check sheet columns for a project without samples.
fn blank_columns(count: u8) -> Vec<SampleColumn> {
    (1..=count)
        .map(|i| SampleColumn {
            index: i.to_string(),
            serial: String::new(),
            date: String::new(),
            inspector: String::new(),
            gauges: String::new(),
            comment: String::new(),
            blank: true,
            verdict: GroupVerdict::Open,
            counts: counts(VerdictCounts::default()),
        })
        .collect()
}

/// The samples of the project with their verdicts.
fn sample_columns(project: &Project, state: &MeasurementState) -> Vec<SampleColumn> {
    project
        .samples
        .iter()
        .zip(&state.samples)
        .enumerate()
        .map(|(i, (sample, sample_state))| SampleColumn {
            index: (i + 1).to_string(),
            serial: sample.serial.clone(),
            date: sample.date.clone(),
            inspector: sample.inspector.clone(),
            gauges: sample.gauges.clone(),
            comment: sample.comment.clone(),
            blank: false,
            verdict: group_verdict(sample_state.verdict),
            counts: counts(sample_state.counts),
        })
        .collect()
}

/// Splits `count` sample columns into blocks of at most [`SAMPLES_PER_BLOCK`]. Always at least
/// one block, so a report without samples still prints its characteristic table.
fn column_blocks(count: usize) -> Vec<ColumnBlock> {
    let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    if count == 0 {
        return vec![ColumnBlock {
            from: 0,
            to: 0,
            first: String::new(),
            last: String::new(),
        }];
    }
    (0..count)
        .step_by(SAMPLES_PER_BLOCK)
        .map(|from| {
            let to = (from + SAMPLES_PER_BLOCK).min(count);
            ColumnBlock {
                from: to_u32(from),
                to: to_u32(to),
                first: (from + 1).to_string(),
                last: to.to_string(),
            }
        })
        .collect()
}

fn counts(counts: VerdictCounts) -> Counts {
    Counts {
        pass: counts.pass.to_string(),
        fail: counts.fail.to_string(),
        not_evaluated: counts.not_evaluated.to_string(),
        missing: counts.missing.to_string(),
    }
}

fn group_verdict(verdict: OverallVerdict) -> GroupVerdict {
    match verdict {
        OverallVerdict::Pass => GroupVerdict::Pass,
        OverallVerdict::Fail => GroupVerdict::Fail,
        OverallVerdict::Open => GroupVerdict::Open,
    }
}

fn cell_verdict(verdict: Verdict) -> CellVerdict {
    match verdict {
        Verdict::Pass => CellVerdict::Pass,
        Verdict::Fail => CellVerdict::Fail,
        Verdict::NotEvaluated => CellVerdict::NotEvaluated,
        Verdict::Missing => CellVerdict::Missing,
    }
}

fn rule_code(rule: &DerivationRule) -> Option<RuleCode> {
    let id = rule.id();
    let code = RuleCode::deserialize(id.as_str().into_deserializer());
    code.map_err(|_: serde::de::value::Error| tracing::warn!(%id, "rule without a report code"))
        .ok()
}

impl RuleCode {
    /// The code as written into the data, also the key of its label in the template
    /// (`rules.<code>`).
    pub fn code(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|json| json.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

/// Printed name of a tolerance table: shipped ISO tables as `ISO 2768-1`, other ids as stored.
fn table_name(table: &TableRef) -> String {
    match table.id.strip_prefix("iso-") {
        Some(rest) => format!("ISO {rest}"),
        None => table.id.clone(),
    }
}

fn unit_symbol(unit: Unit) -> &'static str {
    match unit {
        Unit::Mm => "mm",
        Unit::In => "in",
        Unit::Deg => "°",
    }
}

/// A `YYYY-MM-DD` text as a calendar date, `None` if it is not one.
fn parse_date(text: &str) -> Option<ReportDate> {
    let bytes = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        let part = text.get(range)?;
        part.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| part.parse::<u32>().ok())
            .flatten()
    };
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > days {
        return None;
    }
    Some(ReportDate {
        year: i32::try_from(year).ok()?,
        month: u8::try_from(month).ok()?,
        day: u8::try_from(day).ok()?,
    })
}

/// Formats values in the report language.
struct Formatter {
    language: Language,
}

impl Formatter {
    /// A decimal with the scale as stored (`12.10` stays `12.10`) and the decimal separator of
    /// the language: point in English, comma in German.
    fn decimal(&self, value: rust_decimal::Decimal) -> String {
        let text = value.to_string();
        match self.language {
            Language::En => text,
            Language::De => text.replace('.', ","),
        }
    }

    fn optional(&self, value: Option<rust_decimal::Decimal>) -> String {
        value.map_or_else(String::new, |v| self.decimal(v))
    }

    /// A deviation with an explicit sign: `+0.1`, `-0.2`, `0`.
    fn deviation(&self, value: Option<rust_decimal::Decimal>) -> String {
        value.map_or_else(String::new, |v| {
            // A zero deviation prints without sign, also when stored as -0.
            let v = if v.is_zero() { v.abs() } else { v };
            if v.is_sign_positive() && !v.is_zero() {
                format!("+{}", self.decimal(v))
            } else {
                self.decimal(v)
            }
        })
    }

    fn rule_detail(&self, rule: &DerivationRule) -> String {
        match rule {
            DerivationRule::Fit { table, fit, .. } => format!("{} {fit}", table_name(table)),
            DerivationRule::DrawingRule { lookup }
            | DerivationRule::General { lookup }
            | DerivationRule::CustomTable { lookup } => {
                format!("{} {}", table_name(&lookup.table), lookup.class)
            }
            DerivationRule::DecimalRule { tolerance, .. } => {
                format!("±{}", self.decimal(*tolerance))
            }
            DerivationRule::Explicit
            | DerivationRule::NoToleranceDefined
            | DerivationRule::Manual => String::new(),
        }
    }

    fn cell(&self, result: Option<&MeasurementResult>, verdict: Verdict) -> Cell {
        let Some(result) = result else {
            return Cell::empty();
        };
        let (cell_type, value) = match result.value {
            ResultValue::Variable { value } => (CellType::Variable, self.decimal(value)),
            ResultValue::Attribute { value } => (
                CellType::Attribute,
                match value {
                    AttributeValue::Ok => "ok",
                    AttributeValue::NotOk => "not_ok",
                }
                .to_owned(),
            ),
        };
        Cell {
            cell_type,
            value,
            verdict: cell_verdict(verdict),
            comment: result.comment.clone(),
        }
    }

    fn row(&self, c: &Characteristic, state: &RowState, cells: Vec<Cell>) -> Row {
        Row {
            number: c.number.to_string(),
            kind: c.kind,
            requirement: c.requirement_text.clone(),
            quantity: c.quantity.to_string(),
            nominal: self.optional(c.nominal),
            upper_deviation: self.deviation(c.upper_dev),
            lower_deviation: self.deviation(c.lower_dev),
            lower_limit: self.optional(c.lower_limit),
            upper_limit: self.optional(c.upper_limit),
            unit: c.unit.map_or("", unit_symbol).to_owned(),
            fit: c.fit.clone().unwrap_or_default(),
            rule: c.derivation.as_ref().and_then(|d| rule_code(&d.rule)),
            rule_detail: c
                .derivation
                .as_ref()
                .map_or_else(String::new, |d| self.rule_detail(&d.rule)),
            draft: state.draft,
            method: c.inspection.method.clone(),
            gauge: c.inspection.gauge.clone(),
            classification: c.classification,
            // The evaluation holds the effective type, never auto.
            result_type: match state.result_type {
                ResultType::Attribute => MeasureType::Attribute,
                ResultType::Variable | ResultType::Auto => MeasureType::Variable,
            },
            verdict: group_verdict(state.verdict),
            counts: counts(state.counts),
            cells,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MF F9: every rule has a report code, and code and label key are the rule id of
    /// `project.json`, as the hand written tables were.
    #[test]
    fn rule_codes_are_the_rule_ids() {
        use dimo_core::{SizeRange, TableLookup};
        let table = TableRef {
            id: "iso-2768-1".into(),
            version: 1,
        };
        let lookup = TableLookup {
            table: table.clone(),
            part: "linear".into(),
            class: "m".into(),
            range: SizeRange {
                min: None,
                max: None,
            },
        };
        let rules = [
            DerivationRule::Explicit,
            DerivationRule::Fit {
                table,
                fit: "H7".into(),
                range: None,
            },
            DerivationRule::DrawingRule {
                lookup: lookup.clone(),
            },
            DerivationRule::General {
                lookup: lookup.clone(),
            },
            DerivationRule::DecimalRule {
                places: 2,
                tolerance: rust_decimal::Decimal::new(5, 2),
            },
            DerivationRule::CustomTable { lookup },
            DerivationRule::NoToleranceDefined,
            DerivationRule::Manual,
        ];
        // A new variant is a compile error here and needs an entry in `rules`.
        for rule in &rules {
            match rule {
                DerivationRule::Explicit
                | DerivationRule::Fit { .. }
                | DerivationRule::DrawingRule { .. }
                | DerivationRule::General { .. }
                | DerivationRule::DecimalRule { .. }
                | DerivationRule::CustomTable { .. }
                | DerivationRule::NoToleranceDefined
                | DerivationRule::Manual => {}
            }
        }
        let codes: Vec<String> = rules
            .iter()
            .map(|rule| rule_code(rule).expect("report code").code())
            .collect();
        assert_eq!(
            codes,
            [
                "explicit",
                "fit",
                "drawing_rule",
                "general",
                "decimal_rule",
                "custom_table",
                "no_tolerance_defined",
                "manual",
            ]
        );
        for rule in &rules {
            assert_eq!(rule_code(rule).map(RuleCode::code), Some(rule.id()));
        }
    }

    #[test]
    fn dates_are_real_calendar_dates() {
        assert_eq!(
            parse_date("2026-10-12"),
            Some(ReportDate {
                year: 2026,
                month: 10,
                day: 12
            })
        );
        assert!(parse_date("2024-02-29").is_some());
        for bad in [
            "",
            "2026-1-12",
            "2026-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-04-31",
            "+026-01-01",
            "2026/10/12",
        ] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn decimals_keep_their_scale_and_use_the_language_separator() {
        let en = Formatter {
            language: Language::En,
        };
        let de = Formatter {
            language: Language::De,
        };
        let d = |s: &str| dimo_core::decimal::parse_decimal(s).expect("decimal");
        assert_eq!(en.decimal(d("12.10")), "12.10");
        assert_eq!(de.decimal(d("12.10")), "12,10");
        assert_eq!(de.decimal(d("-0.020")), "-0,020");
        assert_eq!(en.decimal(d("30")), "30");
        assert_eq!(en.deviation(Some(d("0.1"))), "+0.1");
        assert_eq!(de.deviation(Some(d("-0.2"))), "-0,2");
        assert_eq!(en.deviation(Some(d("0"))), "0");
        assert_eq!(en.deviation(None), "");
        assert_eq!(en.deviation(Some(-d("0.00"))), "0.00");
    }

    #[test]
    fn blocks_hold_at_most_six_samples() {
        let blocks = column_blocks(13);
        let ranges: Vec<(u32, u32, &str, &str)> = blocks
            .iter()
            .map(|b| (b.from, b.to, b.first.as_str(), b.last.as_str()))
            .collect();
        assert_eq!(
            ranges,
            [(0, 6, "1", "6"), (6, 12, "7", "12"), (12, 13, "13", "13")]
        );
        assert_eq!(column_blocks(0).len(), 1);
        assert_eq!(column_blocks(6).len(), 1);
    }

    #[test]
    fn table_names() {
        let t = |id: &str| TableRef {
            id: id.into(),
            version: 1,
        };
        assert_eq!(table_name(&t("iso-2768-1")), "ISO 2768-1");
        assert_eq!(table_name(&t("iso-286")), "ISO 286");
        assert_eq!(table_name(&t("shop-a")), "shop-a");
    }
}
