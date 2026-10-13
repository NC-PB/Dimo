//! XLSX check sheet and inspection report (M3 T3.7, decisions 12, 13 and 14, FR-EXP-04,
//! FR-EXP-11).
//!
//! [`render_xlsx`] writes the same [`ReportModel`] as the PDF templates as a workbook with one
//! sheet, built in code with `rust_xlsxwriter` (spreadsheet templates are M7). The words come
//! from the label file of the template set, so PDF and spreadsheet use the same wording.
//!
//! Layout, top to bottom (columns A to H hold the characteristic, then one column per sample,
//! then the row verdict of an inspection report):
//!
//! 1. Title and header block (company, part, customer, order, drawing, revision, full SHA-256,
//!    report number, date).
//! 2. Inspection report only: summary (overall verdict and counts), as text with a fill.
//! 3. Table header rows: for an inspection report the date, inspector, gauges, comment,
//!    verdict and counts per verdict of each sample, then the column heads with the sample
//!    serials. These rows repeat on every printed page, as do columns A to H when the samples are printed in blocks of six.
//! 4. One row per characteristic. Values are number cells with the typed decimals (as the T1.4
//!    list); a number format adds the verdict mark (check, cross, hollow circle), so the verdict
//!    is readable in greyscale, and a cell fill adds the color. Attribute results and verdicts
//!    are text with the same mark and fill.
//! 5. Marks legend, notes, remarks, signature fields.
//!
//! Page header and footer carry the company, title, part, report number, date, drawing and page
//! numbers. The document properties are constant and the ZIP entries carry the fixed DOS
//! epoch time (1980-01-01), so equal models give identical bytes (rule 11).

use std::fmt::Write as _;

use dimo_core::ReportKind;
use rust_decimal::Decimal;
use rust_xlsxwriter::{
    DocProperties, ExcelDateTime, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError,
};

use crate::model::{
    Cell, CellType, CellVerdict, Counts, GroupVerdict, Row, SAMPLES_PER_BLOCK, SampleColumn,
};
use crate::{Language, ReportError, ReportModel, TemplateSet};

/// Selects a text field of a sample column.
type SampleField = fn(&SampleColumn) -> &str;

/// Selects one count of a sample column.
type CountField = fn(&Counts) -> &str;

/// Number of characteristic columns A to H.
const FIXED_COLUMNS: u16 = 8;

/// Most significant digits a decimal can have to be written as a number cell (as the T1.4 list).
const MAX_NUMBER_DIGITS: usize = 15;

/// Sheet names are limited to 31 characters.
const MAX_SHEET_NAME: usize = 31;

/// Longest text of one part of the page header or footer (Excel allows 255 characters in all).
const MAX_HEADER_PART: usize = 36;

/// Print scale when the samples are printed in column blocks.
const BLOCK_PRINT_SCALE: u16 = 60;

const MARK_PASS: &str = "\u{2713}";
const MARK_FAIL: &str = "\u{2717}";
const MARK_NOT_EVALUATED: &str = "\u{25CB}";
const MARK_OPEN: &str = "-";

const FILL_PASS: u32 = 0x00C6_EFCE;
const FILL_FAIL: u32 = 0x00FF_C7CE;
const FILL_NOT_EVALUATED: u32 = 0x00FF_EB9C;
const FILL_OPEN: u32 = 0x00D9_D9D9;
const FILL_HEAD: u32 = 0x00F2_F2F2;

/// Label texts of one language from the label file of a template set.
struct Labels {
    template: String,
    language: &'static str,
    table: toml::Table,
}

impl Labels {
    fn load(template: &TemplateSet, language: Language) -> Result<Self, ReportError> {
        let invalid = |message: String| ReportError::InvalidTemplate(message);
        let path = template.labels_file().ok_or_else(|| {
            invalid("the template set has no label file, needed for XLSX reports".into())
        })?;
        let bytes = template
            .files()
            .get(path)
            .ok_or_else(|| invalid(format!("labels names the missing file {path}")))?;
        let text =
            std::str::from_utf8(bytes).map_err(|_| invalid(format!("{path} is not UTF-8")))?;
        let mut all: toml::Table =
            toml::from_str(text).map_err(|err| invalid(format!("{path}: {err}")))?;
        let language_code = language.code();
        let Some(toml::Value::Table(table)) = all.remove(language_code) else {
            return Err(invalid(format!("{path} has no [{language_code}] table")));
        };
        Ok(Self {
            template: template.id().to_owned(),
            language: language_code,
            table,
        })
    }

    fn missing(&self, key: &str) -> ReportError {
        ReportError::MissingLabel {
            template: self.template.clone(),
            language: self.language,
            key: key.to_owned(),
        }
    }

    fn get(&self, key: &str) -> Result<&str, ReportError> {
        self.table
            .get(key)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| self.missing(key))
    }

    /// A label of a nested table such as `verdicts.pass`.
    fn nested(&self, table: &str, key: &str) -> Result<&str, ReportError> {
        self.table
            .get(table)
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| self.missing(&format!("{table}.{key}")))
    }

    fn verdict(&self, key: &str) -> Result<&str, ReportError> {
        self.nested("verdicts", key)
    }

    /// A verdict label with its mark, such as `\u{2713} Pass`.
    fn marked(&self, mark: &str, key: &str) -> Result<String, ReportError> {
        Ok(format!("{mark} {}", self.verdict(key)?))
    }
}

/// Writes `model` as an XLSX workbook (one sheet) with the words of `template`'s label file.
///
/// The template set must name a label file (`labels` in `template.toml`). Same model and
/// template give byte identical output (FR-EXP-11).
pub fn render_xlsx(template: &TemplateSet, model: &ReportModel) -> Result<Vec<u8>, ReportError> {
    let labels = Labels::load(template, model.language())?;
    Ok(Builder::new(model, &labels).build()?)
}

/// Failure inside the builder: a missing label or a spreadsheet error.
enum BuildError {
    Report(ReportError),
    Xlsx(XlsxError),
}

impl From<ReportError> for BuildError {
    fn from(err: ReportError) -> Self {
        Self::Report(err)
    }
}

impl From<XlsxError> for BuildError {
    fn from(err: XlsxError) -> Self {
        Self::Xlsx(err)
    }
}

impl From<BuildError> for ReportError {
    fn from(err: BuildError) -> Self {
        match err {
            BuildError::Report(err) => err,
            BuildError::Xlsx(err) => Self::Xlsx(err),
        }
    }
}

struct Builder<'a> {
    model: &'a ReportModel,
    labels: &'a Labels,
}

impl<'a> Builder<'a> {
    fn new(model: &'a ReportModel, labels: &'a Labels) -> Self {
        Self { model, labels }
    }

    fn inspection(&self) -> bool {
        self.model.document == ReportKind::InspectionReport
    }

    fn title(&self) -> Result<&'a str, ReportError> {
        self.labels.get(match self.model.document {
            ReportKind::CheckSheet => "check_sheet",
            ReportKind::InspectionReport => "inspection_report",
        })
    }

    fn build(&self) -> Result<Vec<u8>, BuildError> {
        let model = self.model;
        let title = self.title()?;
        let mut workbook = Workbook::new();
        // Constant properties: no clock, no user name (rule 11).
        let doc_title = if model.header.part_number.is_empty() {
            title.to_owned()
        } else {
            format!("{title} {}", model.header.part_number)
        };
        let properties = DocProperties::new()
            .set_title(doc_title)
            .set_author("Dimo")
            .set_creation_datetime(&ExcelDateTime::from_ymd(2000, 1, 1)?);
        workbook.set_properties(&properties);

        let sheet = workbook.add_worksheet();
        sheet.set_name(sheet_name(title))?;
        let result_col = result_column(model.samples.len());

        // Column widths.
        for (col, width) in [
            (0, 6.0),
            (1, 36.0),
            (2, 11.0),
            (3, 11.0),
            (4, 11.0),
            (5, 6.0),
        ]
        .into_iter()
        .chain([(6, 16.0), (7, 20.0)])
        {
            sheet.set_column_width(col, width)?;
        }
        for col in FIXED_COLUMNS..result_col {
            sheet.set_column_width(col, 12.0)?;
        }
        if self.inspection() {
            sheet.set_column_width(result_col, 14.0)?;
        }

        let mut row = 0u32;
        row = self.header_block(sheet, row, title)?;
        if self.inspection() {
            row = self.summary_block(sheet, row)?;
        }
        let (first_head, data_start) = self.table_header(sheet, row)?;
        let after_table = self.table_rows(sheet, data_start, result_col)?;
        self.footer_block(sheet, after_table + 1)?;

        self.print_setup(sheet, title, first_head, data_start - 1)?;
        sheet.set_freeze_panes(0, 2)?;
        Ok(workbook.save_to_buffer()?)
    }

    /// Title and header block; returns the next free row.
    fn header_block(
        &self,
        sheet: &mut Worksheet,
        mut row: u32,
        title: &str,
    ) -> Result<u32, BuildError> {
        let labels = self.labels;
        let h = &self.model.header;
        let big = Format::new().set_bold().set_font_size(14.0);
        let bold = Format::new().set_bold();
        sheet.write_string_with_format(row, 0, title, &big)?;
        row += 1;
        if !h.company.is_empty() {
            sheet.write_string_with_format(row, 0, &h.company, &bold)?;
        }
        row += 2;
        let part = join(" ", [&h.part_number, &h.part_name]);
        let revision_label = labels.get("revision")?;
        let pairs: [(&str, &str); 8] = [
            (labels.get("part")?, &part),
            (labels.get("customer")?, &h.customer),
            (labels.get("order")?, &h.order_reference),
            (labels.get("drawing")?, &h.drawing_file),
            (revision_label, &h.revision),
            (labels.get("sha256_full")?, &h.sha256),
            (labels.get("report_number")?, &h.report_number),
            (labels.get("report_date")?, &h.report_date),
        ];
        for (label, value) in &pairs {
            // An empty value prints no label either; the row stays so the layout is stable.
            if !value.is_empty() {
                sheet.write_string_with_format(row, 1, *label, &bold)?;
                sheet.write_string(row, 2, *value)?;
            }
            row += 1;
        }
        Ok(row + 1)
    }

    /// Summary block of an inspection report; returns the next free row.
    fn summary_block(&self, sheet: &mut Worksheet, mut row: u32) -> Result<u32, BuildError> {
        let labels = self.labels;
        let s = &self.model.summary;
        let bold = Format::new().set_bold();
        sheet.write_string_with_format(row, 1, labels.get("summary")?, &bold)?;
        row += 1;
        sheet.write_string_with_format(row, 1, labels.get("overall")?, &bold)?;
        let (text, format) = self.group_verdict_cell(s.verdict)?;
        sheet.write_string_with_format(row, 2, &text, &format)?;
        row += 1;
        for (label, value) in [
            (labels.get("characteristics")?, s.characteristics.as_str()),
            (labels.get("samples")?, s.samples.as_str()),
        ] {
            sheet.write_string_with_format(row, 1, label, &bold)?;
            write_count(sheet, row, 2, value)?;
            row += 1;
        }
        for (mark, key, value, fill) in [
            (MARK_PASS, "pass", &s.counts.pass, Some(FILL_PASS)),
            (MARK_FAIL, "fail", &s.counts.fail, Some(FILL_FAIL)),
            (
                MARK_NOT_EVALUATED,
                "not_evaluated",
                &s.counts.not_evaluated,
                Some(FILL_NOT_EVALUATED),
            ),
            ("", "missing", &s.counts.missing, None),
        ] {
            let label = if mark.is_empty() {
                labels.verdict(key)?.to_owned()
            } else {
                labels.marked(mark, key)?
            };
            let mut format = bordered();
            if let Some(fill) = fill {
                format = format.set_background_color(fill);
            }
            sheet.write_string_with_format(row, 1, &label, &format)?;
            write_count(sheet, row, 2, value)?;
            row += 1;
        }
        Ok(row + 1)
    }

    /// Writes the table header rows; returns the first header row and the first data row.
    fn table_header(&self, sheet: &mut Worksheet, mut row: u32) -> Result<(u32, u32), BuildError> {
        let labels = self.labels;
        let first = row;
        let head = Format::new()
            .set_bold()
            .set_text_wrap()
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_border(FormatBorder::Thin)
            .set_background_color(FILL_HEAD);
        let side = Format::new().set_bold().set_align(FormatAlign::Right);
        let samples = &self.model.samples;
        let result_col = result_column(samples.len());
        if self.inspection() && !samples.is_empty() {
            let rows: [(&str, SampleField); 4] = [
                (labels.get("date")?, |s| &s.date),
                (labels.get("inspector")?, |s| &s.inspector),
                (labels.get("gauges")?, |s| &s.gauges),
                (labels.get("comment")?, |s| &s.comment),
            ];
            for (label, field) in rows {
                sheet.write_string_with_format(row, FIXED_COLUMNS - 1, label, &side)?;
                for (col, sample) in (FIXED_COLUMNS..).zip(samples) {
                    sheet.write_string_with_format(row, col, field(sample), &wrapped())?;
                }
                row += 1;
            }
            sheet.write_string_with_format(
                row,
                FIXED_COLUMNS - 1,
                labels.get("overall")?,
                &side,
            )?;
            for (col, sample) in (FIXED_COLUMNS..).zip(samples) {
                let (text, format) = self.group_verdict_cell(sample.verdict)?;
                sheet.write_string_with_format(row, col, &text, &format)?;
            }
            row += 1;
            // Counts per sample, as the sample block of the PDF prints them.
            let counts: [(String, CountField); 4] = [
                (labels.marked(MARK_PASS, "pass")?, |c| &c.pass),
                (labels.marked(MARK_FAIL, "fail")?, |c| &c.fail),
                (labels.marked(MARK_NOT_EVALUATED, "not_evaluated")?, |c| {
                    &c.not_evaluated
                }),
                (labels.verdict("missing")?.to_owned(), |c| &c.missing),
            ];
            for (label, field) in counts {
                sheet.write_string_with_format(row, FIXED_COLUMNS - 1, &label, &side)?;
                for (col, sample) in (FIXED_COLUMNS..).zip(samples) {
                    write_count(sheet, row, col, field(&sample.counts))?;
                }
                row += 1;
            }
        }
        let fixed = [
            "number",
            "requirement",
            "nominal",
            "lower_limit",
            "upper_limit",
            "unit",
            "rule",
            "method",
        ];
        for (col, key) in (0u16..).zip(fixed) {
            sheet.write_string_with_format(row, col, labels.get(key)?, &head)?;
        }
        for (col, sample) in (FIXED_COLUMNS..).zip(samples) {
            // Blank columns of a check sheet get an empty head to write the serial into.
            sheet.write_string_with_format(row, col, &sample.serial, &head)?;
        }
        if self.inspection() {
            sheet.write_string_with_format(row, result_col, labels.get("result")?, &head)?;
        }
        sheet.set_row_height(row, 32.0)?;
        Ok((first, row + 1))
    }

    /// Writes the characteristic rows; returns the next free row.
    fn table_rows(
        &self,
        sheet: &mut Worksheet,
        start: u32,
        result_col: u16,
    ) -> Result<u32, BuildError> {
        let mut row = start;
        for r in &self.model.rows {
            self.write_row(sheet, row, r, result_col)?;
            row += 1;
        }
        Ok(row)
    }

    fn write_row(
        &self,
        sheet: &mut Worksheet,
        row: u32,
        r: &Row,
        result_col: u16,
    ) -> Result<(), BuildError> {
        let labels = self.labels;
        let text = bordered().set_text_wrap();
        sheet.write_string_with_format(row, 0, &r.number, &text)?;
        sheet.write_string_with_format(row, 1, &r.requirement, &text)?;
        for (col, value) in [(2, &r.nominal), (3, &r.lower_limit), (4, &r.upper_limit)] {
            write_decimal(
                sheet,
                row,
                col,
                value,
                &bordered(),
                None,
                self.model.language(),
            )?;
        }
        sheet.write_string_with_format(row, 5, &r.unit, &text)?;
        let mut rule = match r.rule {
            Some(code) => {
                let label = labels.nested("rules", rule_key(code))?;
                join(" ", [label, &r.rule_detail])
            }
            None => String::new(),
        };
        if r.draft {
            rule.push('*');
        }
        sheet.write_string_with_format(row, 6, &rule, &text)?;
        sheet.write_string_with_format(row, 7, join(" / ", [&r.method, &r.gauge]), &text)?;
        for (col, cell) in (FIXED_COLUMNS..).zip(&r.cells) {
            self.write_result(sheet, row, col, cell)?;
        }
        if self.inspection() {
            let (text, format) = self.group_verdict_cell(r.verdict)?;
            sheet.write_string_with_format(row, result_col, &text, &format)?;
        }
        Ok(())
    }

    fn write_result(
        &self,
        sheet: &mut Worksheet,
        row: u32,
        col: u16,
        cell: &Cell,
    ) -> Result<(), BuildError> {
        let language = self.model.language();
        let (mark, fill, is_fail) = match cell.verdict {
            CellVerdict::Pass => (MARK_PASS, Some(FILL_PASS), false),
            CellVerdict::Fail => (MARK_FAIL, Some(FILL_FAIL), true),
            CellVerdict::NotEvaluated => (MARK_NOT_EVALUATED, Some(FILL_NOT_EVALUATED), false),
            CellVerdict::Missing => ("", None, false),
        };
        let mut format = bordered().set_align(FormatAlign::Right);
        if let Some(fill) = fill {
            format = format.set_background_color(fill);
        }
        if is_fail {
            format = format.set_bold().set_border(FormatBorder::Medium);
        }
        match cell.cell_type {
            CellType::None => {
                sheet.write_blank(row, col, &format)?;
            }
            CellType::Variable => {
                write_decimal(sheet, row, col, &cell.value, &format, Some(mark), language)?;
            }
            CellType::Attribute => {
                let key = if cell.value == "ok" { "ok" } else { "not_ok" };
                let label = self.labels.nested("attributes", key)?;
                let text = if mark.is_empty() {
                    label.to_owned()
                } else {
                    format!("{mark} {label}")
                };
                sheet.write_string_with_format(row, col, &text, &format)?;
            }
        }
        Ok(())
    }

    /// Text and format of a group verdict: mark, label and fill.
    fn group_verdict_cell(&self, verdict: GroupVerdict) -> Result<(String, Format), ReportError> {
        let (mark, key, fill) = match verdict {
            GroupVerdict::Pass => (MARK_PASS, "pass", FILL_PASS),
            GroupVerdict::Fail => (MARK_FAIL, "fail", FILL_FAIL),
            GroupVerdict::Open => (MARK_OPEN, "open", FILL_OPEN),
        };
        let mut format = bordered()
            .set_align(FormatAlign::Center)
            .set_background_color(fill)
            .set_bold();
        if verdict == GroupVerdict::Fail {
            format = format.set_border(FormatBorder::Medium);
        }
        Ok((self.labels.marked(mark, key)?, format))
    }

    /// Legend, notes, remarks and signature fields; returns the next free row.
    fn footer_block(&self, sheet: &mut Worksheet, mut row: u32) -> Result<u32, BuildError> {
        let labels = self.labels;
        let model = self.model;
        let bold = Format::new().set_bold();
        let wrap = Format::new().set_text_wrap().set_align(FormatAlign::Top);

        if model.rows.is_empty() {
            sheet.write_string(row, 1, labels.get("no_rows")?)?;
            row += 1;
        }
        if self.inspection() {
            // Marks legend.
            sheet.write_string_with_format(row, 1, labels.get("marks")?, &bold)?;
            let mut legend = String::new();
            for (mark, key) in [
                (MARK_PASS, "pass"),
                (MARK_FAIL, "fail"),
                (MARK_NOT_EVALUATED, "not_evaluated"),
                (MARK_OPEN, "open"),
            ] {
                let _ = write!(legend, "{mark} {}   ", labels.verdict(key)?);
            }
            legend.push_str(labels.get("missing_mark")?);
            sheet.write_string(row, 2, &legend)?;
            row += 1;
        }
        let flags = &model.flags;
        for (show, key) in [
            (flags.draft_limits, "draft_note"),
            (flags.open_results, "open_note"),
            (flags.blank_samples, "blank_note"),
        ] {
            if show {
                let note = labels.get(key)?;
                let note = if key == "draft_note" {
                    format!("* {note}")
                } else {
                    note.to_owned()
                };
                sheet.write_string(row, 1, &note)?;
                row += 1;
            }
        }
        row += 1;

        if self.inspection() && !model.remarks.is_empty() {
            sheet.write_string_with_format(row, 1, labels.get("remarks")?, &bold)?;
            let lines = u32::try_from(model.remarks.lines().count().max(1)).unwrap_or(1);
            sheet.merge_range(row, 2, row, 7, &model.remarks, &wrap)?;
            sheet.set_row_height(row, 15.0 * f64::from(lines))?;
            row += 2;
        }

        // Signature fields: name, date and an empty field for a handwritten signature.
        let head = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_border(FormatBorder::Thin)
            .set_background_color(FILL_HEAD);
        sheet.write_string_with_format(row, 1, labels.get("approval")?, &bold)?;
        sheet.merge_range(row, 2, row, 4, labels.get("name")?, &head)?;
        sheet.merge_range(row, 5, row, 6, labels.get("date")?, &head)?;
        sheet.write_string_with_format(row, 7, labels.get("signature")?, &head)?;
        row += 1;
        let sigs = &model.signatures;
        let mut lines = vec![("inspected_by", &sigs.inspected_by)];
        if self.inspection() {
            lines.push(("approved_by", &sigs.approved_by));
        }
        let boxed = bordered().set_align(FormatAlign::VerticalCenter);
        for (key, sig) in lines {
            sheet.write_string_with_format(row, 1, labels.get(key)?, &bold)?;
            sheet.merge_range(row, 2, row, 4, &sig.name, &boxed)?;
            sheet.merge_range(row, 5, row, 6, &sig.date, &boxed)?;
            sheet.write_blank(row, 7, &boxed)?;
            sheet.set_row_height(row, 32.0)?;
            row += 1;
        }
        Ok(row)
    }

    fn print_setup(
        &self,
        sheet: &mut Worksheet,
        title: &str,
        first_head: u32,
        last_head: u32,
    ) -> Result<(), BuildError> {
        let labels = self.labels;
        let h = &self.model.header;
        sheet.set_landscape();
        sheet.set_paper_size(9); // A4
        sheet.set_repeat_rows(first_head, last_head)?;
        sheet.set_repeat_columns(0, FIXED_COLUMNS - 1)?;
        let samples = self.model.samples.len();
        if samples <= SAMPLES_PER_BLOCK {
            sheet.set_print_fit_to_pages(1, 0);
        } else {
            // Blocks of six samples with the characteristic columns repeated (decision 12).
            sheet.set_print_scale(BLOCK_PRINT_SCALE);
            let breaks: Vec<u32> = (u32::from(FIXED_COLUMNS)
                ..u32::from(FIXED_COLUMNS) + u32::try_from(samples).unwrap_or(0))
                .step_by(SAMPLES_PER_BLOCK)
                .skip(1)
                .collect();
            sheet.set_vertical_page_breaks(&breaks)?;
        }
        let page = format!(
            "{} &P {} &N",
            escape(labels.get("page")?),
            escape(labels.get("of")?)
        );
        let left = join(" ", [&h.company, &h.part_number, &h.part_name]);
        let right = join(" ", [&h.report_number, &h.report_date]);
        sheet.set_header(format!(
            "&L&B{}&B\n{}&C&B{}&B&R{}\n{page}",
            cut(&left),
            cut(&join(
                " ",
                [
                    labels.get("customer")?,
                    &h.customer,
                    labels.get("order")?,
                    &h.order_reference
                ]
            )),
            cut(title),
            cut(&right),
        ));
        let drawing = join(
            " ",
            [
                h.drawing_file.as_str(),
                labels.get("revision")?,
                h.revision.as_str(),
                labels.get("sha256")?,
                h.sha256_short.as_str(),
            ],
        );
        sheet.set_footer(format!("&L{}", cut(&drawing)));
        Ok(())
    }
}

fn result_column(samples: usize) -> u16 {
    FIXED_COLUMNS + u16::try_from(samples).unwrap_or(u16::MAX - FIXED_COLUMNS)
}

fn bordered() -> Format {
    Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::VerticalCenter)
}

fn wrapped() -> Format {
    bordered().set_text_wrap().set_align(FormatAlign::Center)
}

/// A count as a whole number cell (the model holds counts as text).
fn write_count(sheet: &mut Worksheet, row: u32, col: u16, value: &str) -> Result<(), XlsxError> {
    let format = Format::new().set_align(FormatAlign::Left);
    match value.parse::<u32>() {
        Ok(n) => sheet.write_number_with_format(row, col, f64::from(n), &format),
        Err(_) => sheet.write_string_with_format(row, col, value, &format),
    }
    .map(|_| ())
}

/// Writes a formatted decimal (`12,10` in German) as a number cell that keeps its decimals.
///
/// The number format shows the stored decimal places and, for a result, the verdict `mark` as a
/// literal suffix. Decimals a double cannot hold exactly, and text that is not a decimal, stay
/// text. Empty text writes an empty bordered cell.
fn write_decimal(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    text: &str,
    format: &Format,
    mark: Option<&str>,
    language: Language,
) -> Result<(), XlsxError> {
    if text.is_empty() {
        sheet.write_blank(row, col, format)?;
        return Ok(());
    }
    let normalized = match language {
        Language::En => text.to_owned(),
        Language::De => text.replace(',', "."),
    };
    let decimal = dimo_core::decimal::parse_decimal(&normalized);
    let number = decimal.and_then(number_value);
    if let (Some(d), Some(number)) = (decimal, number) {
        let mut code = if d.scale() == 0 {
            "0".to_owned()
        } else {
            format!("0.{}", "0".repeat(d.scale() as usize))
        };
        if let Some(mark) = mark.filter(|m| !m.is_empty()) {
            let _ = write!(code, " \"{mark}\"");
        }
        let format = format.clone().set_num_format(code);
        sheet.write_number_with_format(row, col, number, &format)?;
        return Ok(());
    }
    let text = match mark.filter(|m| !m.is_empty()) {
        Some(mark) => format!("{text} {mark}"),
        None => text.to_owned(),
    };
    sheet.write_string_with_format(row, col, &text, format)?;
    Ok(())
}

/// The value as a double if that is exact (see [`MAX_NUMBER_DIGITS`]).
fn number_value(d: Decimal) -> Option<f64> {
    let digits = d.mantissa().unsigned_abs().to_string().len();
    if digits > MAX_NUMBER_DIGITS {
        return None;
    }
    d.to_string().parse().ok()
}

fn rule_key(code: crate::model::RuleCode) -> &'static str {
    use crate::model::RuleCode::{
        CustomTable, DecimalRule, DrawingRule, Explicit, Fit, General, Manual, NoToleranceDefined,
    };
    match code {
        Explicit => "explicit",
        Fit => "fit",
        DrawingRule => "drawing_rule",
        General => "general",
        DecimalRule => "decimal_rule",
        CustomTable => "custom_table",
        NoToleranceDefined => "no_tolerance_defined",
        Manual => "manual",
    }
}

/// Joins the non empty parts with `separator`.
fn join<S: AsRef<str>>(separator: &str, parts: impl IntoIterator<Item = S>) -> String {
    parts
        .into_iter()
        .filter(|p| !p.as_ref().is_empty())
        .map(|p| p.as_ref().to_owned())
        .collect::<Vec<_>>()
        .join(separator)
}

/// A sheet name valid for a spreadsheet: at most 31 characters, none of `[]:*?/\`.
fn sheet_name(title: &str) -> String {
    title
        .chars()
        .filter(|c| !"[]:*?/\\".contains(*c))
        .take(MAX_SHEET_NAME)
        .collect()
}

/// Escapes `&` for the control codes of page headers and footers.
fn escape(text: &str) -> String {
    text.replace('&', "&&")
}

/// Escaped text of at most [`MAX_HEADER_PART`] characters (after escaping, so the whole header
/// stays below the 255 characters Excel allows).
fn cut(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let add = if c == '&' { 2 } else { 1 };
        if out.chars().count() + add > MAX_HEADER_PART {
            break;
        }
        if c == '&' {
            out.push('&');
        }
        out.push(c);
    }
    out
}
