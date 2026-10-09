//! Data exports of a project (FR-EXP-09, FR-EXP-11, architecture 05 `Exporter`).
//!
//! An [`Exporter`] turns a [`Project`] into the bytes of one file. Exports are pure: the output
//! depends only on the project and the [`ExportOptions`], never on the clock, the machine or a
//! random source, so the same input gives byte identical files (rule 11).
//!
//! - [`CsvListExporter`]: the characteristic list as CSV for CMM programming.
//! - [`XlsxListExporter`]: the same list as an Excel workbook.
//!
//! Both write the columns of [`Column`] in the same order, see `docs/user/en/exports.md`.

mod columns;
mod csv_list;
mod xlsx_list;

use std::io::Write;

use dimo_core::Project;

pub use columns::{COLUMNS, Column};
pub use csv_list::CsvListExporter;
pub use xlsx_list::XlsxListExporter;

/// Language of column headers and other fixed texts, chosen per export independent of the UI
/// language (D-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// English.
    #[default]
    English,
    /// German.
    German,
}

/// Options shared by all exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExportOptions {
    /// Language of headers and fixed texts.
    pub language: Language,
}

/// Why an export failed.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// Writing to the output failed.
    #[error("cannot write the export: {0}")]
    Io(#[from] std::io::Error),
    /// The CSV writer failed.
    #[error("cannot write CSV: {0}")]
    Csv(#[from] csv::Error),
    /// The XLSX writer failed.
    #[error("cannot write XLSX: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
}

/// A data output of a project (architecture 05).
///
/// Differences from the sketch in the architecture document: `export` returns [`ExportError`],
/// and the trait adds [`file_extension`](Exporter::file_extension) so callers can propose a
/// file name.
pub trait Exporter {
    /// File name extension without the dot, e.g. `csv`.
    fn file_extension(&self) -> &'static str;

    /// Writes the export of `project` to `out`.
    fn export(
        &self,
        project: &Project,
        opts: &ExportOptions,
        out: &mut dyn Write,
    ) -> Result<(), ExportError>;
}
