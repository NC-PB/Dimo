//! XLSX characteristic list.

use std::collections::BTreeMap;
use std::io::Write;

use dimo_core::Project;
use rust_decimal::Decimal;
use rust_xlsxwriter::{DocProperties, ExcelDateTime, Format, Workbook, Worksheet};

use super::columns::{COLUMNS, Cell, rows};
use super::{ExportError, ExportOptions, Exporter, Language};

/// Most significant digits a decimal can have to be written as a number cell. Spreadsheet
/// numbers are IEEE doubles, which hold every decimal of up to 15 significant digits exactly as
/// typed.
const MAX_NUMBER_DIGITS: usize = 15;

/// Writes the characteristic list as an XLSX workbook with one sheet.
///
/// Decimals with at most 15 significant digits are number cells with a number format that shows
/// the stored number of decimal places, so `90.0` is shown as `90.0`. Longer decimals, which a
/// double cannot hold, are text cells with the exact digits. Display numbers, quantities and
/// sheet numbers are whole number cells, everything else is text. The workbook properties are
/// constant (fixed creation date, no clock), so identical input gives identical bytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct XlsxListExporter;

impl Exporter for XlsxListExporter {
    fn file_extension(&self) -> &'static str {
        "xlsx"
    }

    fn export(
        &self,
        project: &Project,
        opts: &ExportOptions,
        out: &mut dyn Write,
    ) -> Result<(), ExportError> {
        let bytes = build(project, *opts)?;
        out.write_all(&bytes)?;
        Ok(())
    }
}

fn build(project: &Project, opts: ExportOptions) -> Result<Vec<u8>, ExportError> {
    let mut workbook = Workbook::new();
    // Constant properties: no clock, no user name (rule 11). The ZIP entries take their time
    // from the creation date as well.
    let title = if project.info.part_number.is_empty() {
        "Dimo"
    } else {
        &project.info.part_number
    };
    let properties = DocProperties::new()
        .set_title(title)
        .set_author("Dimo")
        .set_creation_datetime(&ExcelDateTime::from_ymd(2000, 1, 1)?);
    workbook.set_properties(&properties);

    let header_format = Format::new().set_bold();
    let mut decimal_formats: BTreeMap<u32, Format> = BTreeMap::new();

    let sheet = workbook.add_worksheet();
    sheet.set_name(match opts.language {
        Language::English => "Characteristics",
        Language::German => "Merkmale",
    })?;
    for (col, column) in (0u16..).zip(COLUMNS) {
        sheet.write_string_with_format(0, col, column.header(opts.language), &header_format)?;
        sheet.set_column_width(col, column.width())?;
    }
    let rows = rows(project);
    for (row, characteristic) in (1u32..).zip(&rows) {
        for (col, column) in (0u16..).zip(COLUMNS) {
            write_cell(
                sheet,
                &mut decimal_formats,
                row,
                col,
                column.cell(project, characteristic),
            )?;
        }
    }
    sheet.set_freeze_panes(1, 0)?;
    let last_row = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let last_col = u16::try_from(COLUMNS.len() - 1).unwrap_or(u16::MAX);
    sheet.autofilter(0, 0, last_row, last_col)?;
    Ok(workbook.save_to_buffer()?)
}

fn write_cell(
    sheet: &mut Worksheet,
    formats: &mut BTreeMap<u32, Format>,
    row: u32,
    col: u16,
    cell: Cell,
) -> Result<(), ExportError> {
    match cell {
        Cell::Empty => {}
        Cell::Text(text) => {
            sheet.write_string(row, col, text)?;
        }
        Cell::Integer(n) => {
            sheet.write_number(row, col, f64::from(n))?;
        }
        Cell::Decimal(d) => match number_value(d) {
            Some(number) => {
                let format = formats.entry(d.scale()).or_insert_with(|| {
                    let places = d.scale() as usize;
                    let code = if places == 0 {
                        "0".to_owned()
                    } else {
                        format!("0.{}", "0".repeat(places))
                    };
                    Format::new().set_num_format(code)
                });
                sheet.write_number_with_format(row, col, number, format)?;
            }
            None => {
                sheet.write_string(row, col, d.to_string())?;
            }
        },
    }
    Ok(())
}

/// The value as a double if that is exact (see [`MAX_NUMBER_DIGITS`]).
fn number_value(d: Decimal) -> Option<f64> {
    let digits = d.mantissa().unsigned_abs().to_string().len();
    if digits > MAX_NUMBER_DIGITS {
        return None;
    }
    // Parsing the printed digits gives the double nearest to the decimal.
    d.to_string().parse().ok()
}
