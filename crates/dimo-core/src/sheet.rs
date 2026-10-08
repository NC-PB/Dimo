//! Sheet vocabulary (data model 07, recognition stage 1).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How the text of a sheet is stored, which decides the recognition path (FR-DOC-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SheetKind {
    /// PDF text objects cover the dimension text.
    VectorText,
    /// Text was exported as curves.
    VectorOutlined,
    /// Scanned or image only.
    Raster,
}
