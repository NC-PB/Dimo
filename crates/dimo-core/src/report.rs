//! Report data stored in the project (data model 07 `IssuedReport`, FR-RVC-04, D-23,
//! M3 decisions 1 and 17).
//!
//! - [`ReportInfo`]: the fields printed in the report header and signature block.
//! - [`IssuedReport`]: the record of a report exported as issued. It keeps the hash of the
//!   written file, so what was sent stays provable after limits change (M3 decision 2).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::env::Timestamp;
use crate::id::RevisionId;
use crate::sheet::Sha256Hex;

/// Report header and approval data (M3 decision 1). All text; empty means not set.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ReportInfo {
    /// Report number.
    pub report_number: String,
    /// Report date as `YYYY-MM-DD`, or empty.
    pub report_date: String,
    /// Name of the issuing company (the shop).
    pub company_name: String,
    /// Who inspected the parts.
    pub inspected_by: String,
    /// Who approved the report.
    pub approved_by: String,
    /// Remarks printed on the report.
    pub remarks: String,
}

/// Which document a report is (FR-EXP-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    /// Characteristic list with empty result columns for printing.
    CheckSheet,
    /// Results, verdicts and approval.
    InspectionReport,
}

/// File format of a written report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    /// PDF.
    Pdf,
    /// Spreadsheet (XLSX).
    Xlsx,
}

/// A report template, by id and version (FR-EXP-06, M3 decision 10).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct TemplateRef {
    /// Template id, e.g. `dimo-default`.
    pub id: String,
    /// Template version.
    pub version: u32,
}

/// Record of a report exported as issued (data model `IssuedReport`, D-23, M3 decision 17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct IssuedReport {
    /// Which document.
    pub kind: ReportKind,
    /// The template it was rendered with.
    pub template: TemplateRef,
    /// The drawing revision that was current.
    pub revision: RevisionId,
    /// SHA-256 of that revision's drawing file (FR-DOC-07).
    pub drawing_sha256: Sha256Hex,
    /// When it was exported, from the environment clock. Project data only; never written into
    /// the report file (FR-EXP-11).
    pub exported_at: Timestamp,
    /// SHA-256 of the written file.
    pub output_sha256: Sha256Hex,
    /// File format.
    pub format: ReportFormat,
}
