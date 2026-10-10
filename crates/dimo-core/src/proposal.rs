//! Proposals: what automation suggests before the user accepts it (data model `Proposal`,
//! ADR 0006, FR-REC-02).
//!
//! Recognition (box select in M2, auto detection in M4) never changes the project. It returns
//! [`Proposal`]s, and only [`Command::AcceptProposals`](crate::command::Command::AcceptProposals)
//! turns them into characteristics, all in one undo step.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::characteristic::{CharacteristicKind, Origin, SourceRegion, Unit};
use crate::decimal::decimal_option_schema;
use crate::derivation::ToleranceDerivation;
use crate::geometry::Point;

/// Where the balloon of an accepted proposal goes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonPlacement {
    /// Balloon center in sheet space.
    pub position: Point,
    /// Leader anchor on the feature in sheet space.
    pub anchor: Point,
}

/// A callout text that could not be parsed (spec 08 stage 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ParseIssue {
    /// Byte position in the raw text where parsing stopped.
    pub position: u32,
    /// What the parser expected there, in English.
    pub expected: String,
}

/// Something the parser assumed that the user should check (spec 08 stage 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ParseHint {
    /// The unit inch was read from the notation (fraction or no leading zero), not from an
    /// explicit unit marker (D-20).
    InchFromNotation,
    /// Tolerance lines stacked above each other were joined.
    StackedLinesJoined,
    /// A decimal comma was read as decimal point.
    DecimalComma,
}

/// A suggested characteristic (data model `Proposal`, ADR 0006): characteristic fields plus
/// source region, origin, balloon placement and parse hints.
///
/// The frontend may change the fields before accepting, like any typed value. Limits are stored
/// as proposed, they are not derived again from nominal and deviations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Proposal {
    /// What the characteristic describes.
    pub kind: CharacteristicKind,
    /// Text as read from the drawing.
    pub requirement_text: String,
    /// Nominal value.
    #[serde(with = "crate::decimal::serde_str_nullable")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub nominal: Option<Decimal>,
    /// Unit of nominal, deviations and limits.
    pub unit: Option<Unit>,
    /// Upper deviation.
    #[serde(with = "crate::decimal::serde_str_nullable")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub upper_dev: Option<Decimal>,
    /// Lower deviation.
    #[serde(with = "crate::decimal::serde_str_nullable")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub lower_dev: Option<Decimal>,
    /// Upper limit.
    #[serde(with = "crate::decimal::serde_str_nullable")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub upper_limit: Option<Decimal>,
    /// Lower limit.
    #[serde(with = "crate::decimal::serde_str_nullable")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub lower_limit: Option<Decimal>,
    /// Fit designation such as `H7`.
    pub fit: Option<String>,
    /// Which rule produced the limits (FR-TOL-08).
    pub derivation: Option<ToleranceDerivation>,
    /// Number of features, from `4X` or `2 PL`. At least 1.
    pub quantity: u32,
    /// False for reference and basic dimensions (D-25, FR-CHR-08).
    pub inspect: bool,
    /// Region and text the proposal was read from (FR-CHR-09).
    pub source: SourceRegion,
    /// How the proposal was made, e.g. `box_select`.
    pub origin: Origin,
    /// Where the balloon goes when accepted.
    pub placement: BalloonPlacement,
    /// Why the text could not be parsed, if it could not. Such a proposal has no limits.
    pub parse_error: Option<ParseIssue>,
    /// Assumptions of the parser to check.
    pub parse_hints: Vec<ParseHint>,
}
