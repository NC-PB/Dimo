//! CSV characteristic list for CMM programming (FR-EXP-09).

use std::io::Write;

use dimo_core::Project;

use super::columns::{COLUMNS, Cell, rows};
use super::{ExportError, ExportOptions, Exporter};

/// Writes the characteristic list as CSV.
///
/// UTF-8 without byte order mark, comma separated, fields quoted only where needed (RFC 4180),
/// `\n` line ends, a header row. Decimals are written with the stored digits and a `.`, e.g.
/// `30.0203` and `90.0`. Enumerations are written as stable lowercase identifiers independent of
/// the header language. Empty values are empty fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct CsvListExporter;

impl Exporter for CsvListExporter {
    fn file_extension(&self) -> &'static str {
        "csv"
    }

    fn export(
        &self,
        project: &Project,
        opts: &ExportOptions,
        out: &mut dyn Write,
    ) -> Result<(), ExportError> {
        let mut writer = csv::WriterBuilder::new()
            .terminator(csv::Terminator::Any(b'\n'))
            .from_writer(out);
        writer.write_record(COLUMNS.iter().map(|c| c.header(opts.language)))?;
        for characteristic in rows(project) {
            writer.write_record(COLUMNS.iter().map(|column| {
                match column.cell(project, characteristic) {
                    Cell::Empty => String::new(),
                    Cell::Text(text) => text,
                    Cell::Integer(n) => n.to_string(),
                    Cell::Decimal(d) => d.to_string(),
                }
            }))?;
        }
        writer.flush()?;
        Ok(())
    }
}
