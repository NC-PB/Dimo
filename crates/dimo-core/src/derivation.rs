//! Where the limits of a characteristic come from (data model `ToleranceDerivation`, FR-TOL-01,
//! FR-TOL-08, D-43, M2 decision 3).
//!
//! The project stores the rule and its parameters, never free text. The explanation shown in the
//! UI is built from this structure in the UI language, and exports write the rule in a stable
//! English form.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::characteristic::{ToleranceRule, Unit};
use crate::decimal::{decimal_option_schema, decimal_schema};

/// A tolerance table by id and version (`data/tolerances/`, D-43). Shipped tables are referenced
/// this way; custom tables are also copied into the project (M2 decision 4).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct TableRef {
    /// Table id, e.g. `iso-2768-1`.
    pub id: String,
    /// Table version.
    pub version: u32,
}

/// One end of a size range, as in the table row (exact decimals, rule 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct RangeBound {
    /// The bound in the table's size unit.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub value: Decimal,
    /// Whether the bound belongs to the range.
    pub inclusive: bool,
}

/// The size range of the table row a value came from, e.g. "over 30 up to and including 120".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SizeRange {
    /// Lower bound; `null` means the range starts above zero.
    pub min: Option<RangeBound>,
    /// Upper bound; `null` means the range is open ended.
    pub max: Option<RangeBound>,
}

/// A value read from a range table: which table, part, column and row (FR-TOL-02, FR-TOL-07).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct TableLookup {
    /// The table.
    pub table: TableRef,
    /// Part id in the table, e.g. `linear`, `radius_chamfer` or `angular`.
    pub part: String,
    /// Column, e.g. the tolerance class `m`.
    pub class: String,
    /// Range of the row that covers the size.
    pub range: SizeRange,
}

/// The rule that produced the limits, with its parameters (FR-TOL-01 precedence, FR-TOL-08).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum DerivationRule {
    /// Tolerance written on the callout: deviations, `±`, limits, `MIN` or `MAX`.
    Explicit,
    /// Fit designation expanded with a fit table (FR-TOL-04).
    Fit {
        /// The fit table, e.g. `iso-286`.
        table: TableRef,
        /// The designation as written, e.g. `H7` or `H7/g6`.
        fit: String,
        /// Size step of the table the nominal lies in.
        range: Option<SizeRange>,
    },
    /// Drawing specific rule, such as a custom table assigned to the project as drawing rule.
    DrawingRule {
        /// Where the value came from.
        lookup: TableLookup,
    },
    /// General tolerance standard by size range, e.g. ISO 2768-m (FR-TOL-02).
    General {
        /// Where the value came from.
        lookup: TableLookup,
    },
    /// Decimal place rule, e.g. `X.XX ±0.05` (FR-TOL-06).
    DecimalRule {
        /// Decimal places of the nominal as written.
        places: u32,
        /// Symmetric tolerance of the rule.
        #[serde(with = "crate::decimal::serde_str")]
        #[schemars(schema_with = "decimal_schema")]
        #[cfg_attr(feature = "specta", specta(type = String))]
        tolerance: Decimal,
    },
    /// General tolerance from a custom table of the user (FR-TOL-07).
    CustomTable {
        /// Where the value came from.
        lookup: TableLookup,
    },
    /// No tolerance on the callout and no general rule: no limits, flagged for review.
    NoToleranceDefined,
    /// Limits typed or edited by hand. Never re-interpreted automatically.
    Manual,
}

impl DerivationRule {
    /// The rule without parameters, as in the corpus truth format.
    pub fn kind(&self) -> Option<ToleranceRule> {
        Some(match self {
            Self::Explicit => ToleranceRule::Explicit,
            Self::Fit { .. } => ToleranceRule::Fit,
            Self::DrawingRule { .. } => ToleranceRule::DrawingRule,
            Self::General { .. } => ToleranceRule::General,
            Self::DecimalRule { .. } => ToleranceRule::DecimalRule,
            Self::CustomTable { .. } => ToleranceRule::CustomTable,
            Self::NoToleranceDefined => ToleranceRule::NoToleranceDefined,
            Self::Manual => return None,
        })
    }
}

/// A remark on a derivation that the user should see, without changing the limits (spec 08
/// stage 7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "hint", rename_all = "snake_case")]
pub enum DerivationHint {
    /// The printed deviations differ from the fit table. The printed ones were kept.
    FitDeviationsDiffer {
        /// The fit as written.
        fit: String,
        /// Upper deviation of the fit table.
        #[serde(with = "crate::decimal::serde_str_nullable")]
        #[schemars(schema_with = "decimal_option_schema")]
        #[cfg_attr(feature = "specta", specta(type = Option<String>))]
        table_upper_dev: Option<Decimal>,
        /// Lower deviation of the fit table.
        #[serde(with = "crate::decimal::serde_str_nullable")]
        #[schemars(schema_with = "decimal_option_schema")]
        #[cfg_attr(feature = "specta", specta(type = Option<String>))]
        table_lower_dev: Option<Decimal>,
    },
    /// The fit is not in the fit table; no limits from it.
    UnknownFit {
        /// The fit as written.
        fit: String,
    },
    /// The size is outside the ranges of the table that would apply.
    SizeOutsideTable {
        /// The table.
        table: TableRef,
    },
    /// Reference dimension, not toleranced and not inspected by default (D-25, FR-CHR-08).
    ReferenceDimension,
    /// Basic (theoretically exact) dimension, not inspected by default (D-25, FR-CHR-08).
    BasicDimension,
}

/// A unit conversion applied to nominal and limits (FR-TOL-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct UnitConversion {
    /// Unit on the drawing.
    pub from: Unit,
    /// Unit stored in the characteristic.
    pub to: Unit,
    /// Decimal places the converted values were rounded to.
    pub places: u32,
}

/// Which rule produced the limits of a characteristic and what the user should know about it
/// (data model `ToleranceDerivation`, FR-TOL-08). No free text: explanations are rendered from
/// this (M2 decision 3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ToleranceDerivation {
    /// The rule and its parameters.
    pub rule: DerivationRule,
    /// True if a value came from a table with `status = "draft"`; the UI shows a badge (D-43).
    pub draft: bool,
    /// Remarks for the user.
    pub hints: Vec<DerivationHint>,
    /// Unit conversion applied, if any.
    pub conversion: Option<UnitConversion>,
}

impl ToleranceDerivation {
    /// A derivation by `rule` from no table, without hints or conversion.
    pub fn new(rule: DerivationRule) -> Self {
        Self {
            rule,
            draft: false,
            hints: Vec::new(),
            conversion: None,
        }
    }

    /// Limits typed or edited by hand.
    pub fn manual() -> Self {
        Self::new(DerivationRule::Manual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_rule_with_parameters_flat() {
        let derivation = ToleranceDerivation {
            rule: DerivationRule::General {
                lookup: TableLookup {
                    table: TableRef {
                        id: "iso-2768-1".into(),
                        version: 1,
                    },
                    part: "linear".into(),
                    class: "m".into(),
                    range: SizeRange {
                        min: Some(RangeBound {
                            value: Decimal::new(30, 0),
                            inclusive: false,
                        }),
                        max: Some(RangeBound {
                            value: Decimal::new(120, 0),
                            inclusive: true,
                        }),
                    },
                },
            },
            draft: true,
            hints: vec![DerivationHint::ReferenceDimension],
            conversion: None,
        };
        let json = serde_json::to_value(&derivation).unwrap();
        assert_eq!(json["rule"]["rule"], "general");
        assert_eq!(json["rule"]["lookup"]["range"]["min"]["value"], "30");
        assert_eq!(json["hints"][0]["hint"], "reference_dimension");
        let back: ToleranceDerivation = serde_json::from_value(json).unwrap();
        assert_eq!(back, derivation);

        let rule = DerivationRule::DecimalRule {
            places: 2,
            tolerance: Decimal::new(5, 2),
        };
        let json = serde_json::to_value(&rule).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "rule": "decimal_rule", "places": 2, "tolerance": "0.05" })
        );
        assert_eq!(rule.kind(), Some(ToleranceRule::DecimalRule));
        assert_eq!(DerivationRule::Manual.kind(), None);
    }
}
