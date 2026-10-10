//! Drawing revisions and their sheets (data model 07, FR-DOC-05, FR-DOC-07).

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

use crate::characteristic::Unit;
use crate::env::Timestamp;
use crate::geometry::{Rect, Size};
use crate::id::{RevisionId, SheetId};

/// How the text of a sheet is stored, which decides the recognition path (FR-DOC-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SheetKind {
    /// PDF text objects cover the dimension text.
    VectorText,
    /// Text was exported as curves.
    VectorOutlined,
    /// Scanned or image only.
    Raster,
}

/// Clockwise rotation used to view a sheet, in 90 degree steps (FR-DOC-05).
///
/// Only the view rotates. Stored geometry stays in unrotated sheet space (rule 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    /// Not rotated.
    #[default]
    Deg0,
    /// 90 degrees clockwise.
    Deg90,
    /// 180 degrees.
    Deg180,
    /// 270 degrees clockwise (90 counterclockwise).
    Deg270,
}

impl Rotation {
    /// The angle in degrees, clockwise.
    pub fn degrees(self) -> u32 {
        match self {
            Self::Deg0 => 0,
            Self::Deg90 => 90,
            Self::Deg180 => 180,
            Self::Deg270 => 270,
        }
    }
}

/// Drawing scale as printed in the title block, e.g. `1:2` (FR-DOC-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Scale {
    /// Length on the drawing. At least 1.
    pub drawing: u32,
    /// Corresponding length on the part. At least 1.
    pub actual: u32,
}

impl Scale {
    /// Full size, `1:1`.
    pub const FULL: Self = Self {
        drawing: 1,
        actual: 1,
    };

    /// True if both parts are at least 1.
    pub fn is_valid(self) -> bool {
        self.drawing > 0 && self.actual > 0
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self::FULL
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.drawing, self.actual)
    }
}

/// One sheet (PDF page) of a drawing revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Sheet {
    /// Stable ID.
    pub id: SheetId,
    /// Zero based page index in the PDF.
    pub index: u32,
    /// Size in PDF user units, unrotated.
    pub size: Size,
    /// View rotation.
    pub rotation: Rotation,
    /// How the text is stored.
    pub kind: SheetKind,
    /// Resolution of raster sheets.
    pub raster_dpi: Option<u32>,
    /// Length unit of the dimensions on this sheet, `mm` or `in`. Default `mm` (D-20: never
    /// inferred from the sheet size).
    pub unit: Unit,
    /// Drawing scale.
    pub scale: Scale,
    /// Zone grid of the drawing frame, defined by hand in M2 (M2 decision 1, D-21).
    pub zone_grid: Option<ZoneGrid>,
    /// Views drawn by hand, in the order the user drew them (M2 decision 1).
    pub views: Vec<SheetView>,
}

impl Sheet {
    /// A sheet with default view settings: not rotated, unit mm, scale 1:1, no zones, no views.
    pub fn new(id: SheetId, index: u32, size: Size, kind: SheetKind) -> Self {
        Self {
            id,
            index,
            size,
            rotation: Rotation::Deg0,
            kind,
            raster_dpi: None,
            unit: Unit::Mm,
            scale: Scale::FULL,
            zone_grid: None,
            views: Vec::new(),
        }
    }
}

/// The zone grid of a drawing frame: equal columns and rows inside the frame rectangle, with
/// the labels printed on the frame (D-21 "sheet, then zone").
///
/// Labels are listed left to right and top to bottom as they appear on the sheet, so grids
/// numbered from the right or lettered from the bottom are stored as printed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ZoneGrid {
    /// Inner frame rectangle the zones divide, in sheet space.
    pub frame: Rect,
    /// Column labels, left to right, e.g. `1` to `8`. One per column, at least one.
    pub column_labels: Vec<String>,
    /// Row labels, top to bottom, e.g. `A` to `F`. One per row, at least one.
    pub row_labels: Vec<String>,
}

impl ZoneGrid {
    /// Most columns or rows a grid may have.
    pub const MAX_DIVISIONS: usize = 100;

    /// Number of columns.
    pub fn columns(&self) -> usize {
        self.column_labels.len()
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.row_labels.len()
    }

    /// Checks the frame and the labels: 1 to 100 columns and rows, labels not empty and unique
    /// per axis.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.frame.is_valid() {
            return Err("zone frame needs a finite position and a positive size");
        }
        for labels in [&self.column_labels, &self.row_labels] {
            if labels.is_empty() || labels.len() > Self::MAX_DIVISIONS {
                return Err("a zone grid has 1 to 100 columns and rows");
            }
            if labels.iter().any(|l| l.trim().is_empty() || l.len() > 16) {
                return Err("zone labels must not be empty or longer than 16 bytes");
            }
            let mut sorted: Vec<&String> = labels.iter().collect();
            sorted.sort();
            sorted.dedup();
            if sorted.len() != labels.len() {
                return Err("zone labels must be unique per axis");
            }
        }
        Ok(())
    }
}

/// A view of the drawing, drawn by hand as a rectangle (M2 decision 1, FR-BAL-04 "per view").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SheetView {
    /// Name as printed, e.g. `A-A` or `Detail B`. Empty if none.
    pub label: String,
    /// Rectangle around the view, in sheet space.
    pub rect: Rect,
}

impl SheetView {
    /// Checks the rectangle.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.rect.is_valid() {
            return Err("view rectangle needs a finite position and a positive size");
        }
        if self.label.len() > 256 {
            return Err("view labels are at most 256 bytes");
        }
        Ok(())
    }
}

/// SHA-256 of a file as 64 lowercase hex digits (FR-DOC-07).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct Sha256Hex(String);

/// Error for text that is not 64 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid SHA-256 {0:?}, expected 64 lowercase hex digits")]
pub struct Sha256Error(String);

impl Sha256Hex {
    /// Checks 64 lowercase hex digits.
    pub fn parse(text: &str) -> Result<Self, Sha256Error> {
        let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
        if text.len() == 64 && text.bytes().all(hex) {
            Ok(Self(text.to_owned()))
        } else {
            Err(Sha256Error(text.to_owned()))
        }
    }

    /// Hex digits from 32 hash bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        use fmt::Write;
        let mut text = String::with_capacity(64);
        for b in bytes {
            // Writing to a String cannot fail.
            let _ = write!(text, "{b:02x}");
        }
        Self(text)
    }

    /// The hex digits.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = Sha256Error;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<Sha256Hex> for String {
    fn from(hash: Sha256Hex) -> Self {
        hash.0
    }
}

impl JsonSchema for Sha256Hex {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Sha256Hex".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string", "pattern": "^[0-9a-f]{64}$" })
    }
}

/// One imported drawing file (data model `DrawingRevision`, FR-DOC-07).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct DrawingRevision {
    /// Stable ID.
    pub id: RevisionId,
    /// Revision label from the title block or the user, e.g. `B`. Empty if unknown.
    pub label: String,
    /// File name at import, without directories.
    pub file_name: String,
    /// SHA-256 of the imported file. The file is stored as `drawings/<sha256>.pdf`.
    pub sha256: Sha256Hex,
    /// When the file was imported.
    pub imported_at: Timestamp,
    /// Sheets in page order.
    pub sheets: Vec<Sheet>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_accepts_only_lowercase_hex() {
        let zeros = "0".repeat(64);
        assert!(Sha256Hex::parse(&zeros).is_ok());
        assert!(Sha256Hex::parse(&"A".repeat(64)).is_err());
        assert!(Sha256Hex::parse("abc").is_err());
        assert_eq!(Sha256Hex::from_bytes(&[0xab; 32]).as_str(), "ab".repeat(32));
        assert!(serde_json::from_str::<Sha256Hex>("\"xyz\"").is_err());
    }

    #[test]
    fn rotation_and_scale_serialize_readably() {
        assert_eq!(
            serde_json::to_string(&Rotation::Deg90).unwrap(),
            "\"deg90\""
        );
        assert_eq!(
            Scale {
                drawing: 1,
                actual: 2
            }
            .to_string(),
            "1:2"
        );
    }
}
