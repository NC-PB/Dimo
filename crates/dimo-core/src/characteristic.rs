//! Shared vocabulary of characteristics (data model 07): kind, unit and tolerance rule.
//!
//! The full `Characteristic` entity follows in M1. These enums are defined now because the
//! corpus truth format (T0.10) uses the same names.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a characteristic describes (data model `Characteristic.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacteristicKind {
    /// Linear size or distance.
    Linear,
    /// Diameter (`Ø`, `⌀`, `DIA`).
    Diameter,
    /// Radius (`R`).
    Radius,
    /// Spherical radius (`SR`).
    SphericalRadius,
    /// Angle.
    Angle,
    /// Chamfer, e.g. `1x45°`.
    Chamfer,
    /// Thread, e.g. `M8x1.25-6H`.
    Thread,
    /// Surface texture requirement.
    SurfaceTexture,
    /// Geometric tolerance (feature control frame).
    Geometric,
    /// Text note with an inspectable requirement.
    Note,
    /// Flag note, linked from a flag symbol.
    FlagNote,
}

/// Unit of a nominal value and its limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Millimeter.
    Mm,
    /// Inch.
    In,
    /// Degree of angle (decimal degrees).
    Deg,
}

impl Unit {
    /// True for length units.
    pub fn is_length(self) -> bool {
        matches!(self, Self::Mm | Self::In)
    }
}

/// Which rule produced the limits of a characteristic (data model `ToleranceDerivation`,
/// precedence FR-TOL-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceRule {
    /// Explicit tolerance on the callout (deviations, `±`, limits). Wins over a fit table.
    Explicit,
    /// Fit designation expanded with the ISO 286 tables.
    Fit,
    /// Drawing specific rule such as a variable table or a local note.
    DrawingRule,
    /// General tolerance standard from the title block.
    General,
    /// Decimal place rule from the title block.
    DecimalRule,
    /// Custom tolerance table.
    CustomTable,
    /// The callout has no tolerance and the drawing declares no general tolerance.
    /// The characteristic is flagged for review and has no limits.
    NoToleranceDefined,
}

impl ToleranceRule {
    /// True if this rule produces upper and lower limits.
    pub fn yields_limits(self) -> bool {
        !matches!(self, Self::NoToleranceDefined)
    }
}
