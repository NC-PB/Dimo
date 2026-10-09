//! File format of tolerance tables (T2.2, D-43, FR-TOL-02, FR-TOL-04, FR-TOL-07).
//!
//! One TOML format covers general tolerance tables (ISO 2768-1), fit tables (ISO 286) and
//! custom tables written by users. A file has a `[table]` header and a list of `[[part]]`
//! sections. Each part is a range table: rows of size ranges with one value per column.
//! All values are decimal strings in the part's `value_unit`, never TOML numbers (rule 5).
//! The format is documented for users in `data/tolerances/README.md`; the JSON Schema
//! generated from these types is published in `docs/schema/tolerance-table.schema.json`.
//!
//! These types only describe the file. [`crate::table::Table`] is the validated form.

use std::fmt;

use dimo_core::decimal::{DECIMAL_PATTERN, decimal_schema, parse_decimal};
use rust_decimal::Decimal;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, de::Error as _};

/// Pattern of table ids: lowercase letters and digits in groups joined by `-`.
pub const TABLE_ID_PATTERN: &str = r"^[a-z0-9]+(-[a-z0-9]+)*$";

/// Pattern of part ids: lowercase letters, digits and `_`.
pub const PART_ID_PATTERN: &str = r"^[a-z0-9]+(_[a-z0-9]+)*$";

/// Pattern of the verification date, `YYYY-MM-DD`.
pub const DATE_PATTERN: &str = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$";

/// A whole tolerance table file.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schemars(title = "Dimo tolerance table")]
pub struct TableFile {
    /// Identification, source and verification status of the table.
    pub table: TableHeader,
    /// The range tables that make up this table, at least one.
    #[serde(rename = "part")]
    #[schemars(length(min = 1))]
    pub parts: Vec<Part>,
}

/// The `[table]` header.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TableHeader {
    /// Unique id across shipped and user tables, e.g. `iso-2768-1`. Projects reference
    /// shipped tables by id and version.
    #[schemars(pattern(TABLE_ID_PATTERN))]
    pub id: String,
    /// Short human readable name.
    #[schemars(length(min = 1))]
    pub title: String,
    /// Standard number, e.g. `ISO 2768-1`. Absent for company tables.
    #[serde(default)]
    pub standard: Option<String>,
    /// Edition of the standard, e.g. `1989`.
    #[serde(default)]
    pub edition: Option<String>,
    /// What the table is used for.
    pub kind: TableKind,
    /// Unit of all size ranges. Only millimetres for now.
    pub unit: SizeUnit,
    /// Version of this table file, starting at 1. Increased with every change of a value.
    #[schemars(range(min = 1))]
    pub version: u32,
    /// `draft` until the owner has checked every value against the printed source (D-43).
    pub status: Status,
    /// Who verified the table. Present exactly when `status` is `verified`.
    #[serde(default)]
    pub verified_by: Option<String>,
    /// When the table was verified, `YYYY-MM-DD`. Present exactly when `status` is `verified`.
    #[serde(default)]
    #[schemars(pattern(DATE_PATTERN))]
    pub verified_date: Option<String>,
    /// Where the values come from: standard, edition and table numbers, or the company
    /// document. Values and structure only, no text copied from the source.
    #[schemars(length(min = 1))]
    pub source: String,
}

/// Purpose of a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
    /// General tolerances by size range and class (FR-TOL-02), e.g. ISO 2768-1.
    General,
    /// ISO system of limits and fits (FR-TOL-04), e.g. ISO 286-1.
    Fit,
    /// A table written by a user or a company (FR-TOL-07), applied as a drawing rule.
    Custom,
}

/// Unit of the size ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SizeUnit {
    /// Millimetres.
    Mm,
}

/// Verification status (D-43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Drafted, not yet checked. Limits from draft tables carry a warning badge, and a
    /// release build is blocked while any shipped table is a draft.
    Draft,
    /// Every value checked by the owner against the printed source.
    Verified,
}

/// What the values of a part mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PartKind {
    /// A value `t` means the deviations `+t` and `-t`. Columns are tolerance classes.
    /// Used by general and custom tables.
    Symmetric,
    /// Standard tolerance grades (ISO 286). Columns are `IT01`, `IT0`, `IT1` to `IT18`.
    StandardTolerance,
    /// Fundamental deviations (ISO 286). Columns are deviation letters, see the README for
    /// the columns that depend on the grade.
    FundamentalDeviation,
    /// The delta values for holes (ISO 286). Columns are grades `IT3` to `IT8`.
    Delta,
    /// Fundamental deviations that replace the computed one for a single tolerance class in a
    /// size range (special cases of ISO 286). Columns are tolerance classes such as `M6`.
    Override,
}

/// Dimension type a symmetric part applies to (FR-TOL-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppliesTo {
    /// Linear dimensions and diameters, except broken edges.
    Linear,
    /// External radii and chamfer heights (broken edges).
    RadiusChamfer,
    /// Angular dimensions.
    Angular,
}

/// Which size selects the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RangeOf {
    /// The nominal size of the dimension.
    #[default]
    Nominal,
    /// The length of the shorter leg of an angle.
    ShorterLeg,
}

/// Shaft or hole, for fundamental deviations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// External feature, lowercase letters.
    Shaft,
    /// Internal feature, uppercase letters.
    Hole,
}

/// Which deviation a fundamental deviation part holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Deviation {
    /// Upper deviation (es for shafts, ES for holes).
    Upper,
    /// Lower deviation (ei for shafts, EI for holes).
    Lower,
}

/// Unit of the values in a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValueUnit {
    /// Millimetres.
    Mm,
    /// Minutes of arc (1 degree is 60). Exact for the angular general tolerances.
    Arcmin,
}

impl fmt::Display for ValueUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mm => "mm",
            Self::Arcmin => "arcmin",
        })
    }
}

/// One `[[part]]`: a range table.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Part {
    /// Unique within the table, e.g. `linear` or `shaft_upper`.
    #[schemars(pattern(PART_ID_PATTERN))]
    pub id: String,
    /// Short human readable name.
    #[serde(default)]
    pub title: Option<String>,
    /// Table number or section in the source, e.g. `table 1`.
    #[schemars(length(min = 1))]
    pub source: String,
    /// What the values mean.
    pub kind: PartKind,
    /// Dimension type, required for `symmetric` parts and absent otherwise.
    #[serde(default)]
    pub applies_to: Option<AppliesTo>,
    /// Which size selects the row. Default `nominal`.
    #[serde(default)]
    pub range_of: RangeOf,
    /// Shaft or hole, required for `fundamental_deviation` parts and absent otherwise.
    #[serde(default)]
    pub feature: Option<Feature>,
    /// Upper or lower, required for `fundamental_deviation` parts and absent otherwise.
    #[serde(default)]
    pub deviation: Option<Deviation>,
    /// Unit of the values.
    pub value_unit: ValueUnit,
    /// Column names, unique within the part.
    #[schemars(length(min = 1))]
    pub columns: Vec<String>,
    /// Rows in ascending order without gaps. Each row has one value per column.
    #[schemars(length(min = 1))]
    pub rows: Vec<Row>,
}

/// One row of a part: a size range and one value per column.
///
/// A range has at most one lower bound (`min_inclusive` or `min_exclusive`) and at most one
/// upper bound (`max_inclusive` or `max_exclusive`). "Over 30 up to and including 120" is
/// `min_exclusive = "30"`, `max_inclusive = "120"`. Without a lower bound the row starts above
/// zero; only the last row may omit the upper bound.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Row {
    /// Lower bound, the bound itself belongs to the row.
    #[serde(default, with = "dimo_core::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_schema")]
    pub min_inclusive: Option<Decimal>,
    /// Lower bound, the bound itself does not belong to the row.
    #[serde(default, with = "dimo_core::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_schema")]
    pub min_exclusive: Option<Decimal>,
    /// Upper bound, the bound itself belongs to the row.
    #[serde(default, with = "dimo_core::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_schema")]
    pub max_inclusive: Option<Decimal>,
    /// Upper bound, the bound itself does not belong to the row.
    #[serde(default, with = "dimo_core::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_schema")]
    pub max_exclusive: Option<Decimal>,
    /// One value per column, in column order.
    pub values: Vec<Cell>,
}

/// A table cell: a decimal string, or `"-"` where the source defines no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell(pub Option<Decimal>);

/// Text of an empty cell.
pub const EMPTY_CELL: &str = "-";

impl<'de> Deserialize<'de> for Cell {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        if text == EMPTY_CELL {
            return Ok(Self(None));
        }
        parse_decimal(&text)
            .map(|value| Self(Some(value)))
            .ok_or_else(|| {
                D::Error::custom(format!(
                    "invalid cell {text:?}, expected a decimal string such as \"0.021\" or \"-\""
                ))
            })
    }
}

impl JsonSchema for Cell {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Cell".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        let either = format!("{DECIMAL_PATTERN}|^-$");
        json_schema!({
            "type": "string",
            "pattern": either,
            "description": "Exact decimal number as a string, e.g. \"0.021\", or \"-\" where the source defines no value."
        })
    }
}

/// The JSON Schema of a table file with sorted keys.
pub fn table_schema() -> serde_json::Value {
    sort_keys(schemars::schema_for!(TableFile).to_value())
}

/// [`table_schema`] as pretty printed JSON with a trailing newline, the exact content of
/// `docs/schema/tolerance-table.schema.json`.
pub fn table_schema_json() -> String {
    // Serializing a `Value` cannot fail: all keys are strings.
    let mut text = serde_json::to_string_pretty(&table_schema()).unwrap_or_default();
    text.push('\n');
    text
}

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
