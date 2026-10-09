//! Characteristics (data model 07, FR-CHR-01, FR-CHR-02, FR-CHR-09).
//!
//! The enums [`CharacteristicKind`], [`Unit`] and [`ToleranceRule`] are shared with the corpus
//! truth format (T0.10). [`Characteristic`] is the entity stored in `project.json`.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::decimal::{OptionalDecimal, decimal_option_schema};
use crate::geometry::OrientedBox;
use crate::id::{CharId, SheetId};

/// What a characteristic describes (data model `Characteristic.kind`, FR-CHR-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
    /// Counterbore.
    Counterbore,
    /// Countersink.
    Countersink,
    /// Depth of a hole or pocket.
    Depth,
    /// Surface texture requirement.
    SurfaceTexture,
    /// Geometric tolerance (feature control frame).
    Geometric,
    /// Text note with an inspectable requirement.
    Note,
    /// Flag note, linked from a flag symbol.
    FlagNote,
    /// Material or process requirement, e.g. hardness or coating.
    MaterialProcess,
    /// Anything else. Default for a characteristic placed by hand before its kind is known.
    Other,
}

/// Unit of a nominal value and its limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
#[cfg_attr(feature = "specta", derive(specta::Type))]
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

/// Importance class of a characteristic (FR-CHR-02). Default `none` (D-26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// Critical characteristic.
    Critical,
    /// Major characteristic.
    Major,
    /// Minor characteristic.
    Minor,
    /// Key characteristic.
    Key,
    /// Not classified.
    #[default]
    None,
}

/// Review state of a characteristic (data model `status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CharacteristicStatus {
    /// Suggested by automation, not yet reviewed.
    Proposed,
    /// Accepted by the user. Characteristics placed by hand start here.
    Accepted,
    /// Checked by a second look.
    Verified,
    /// Kept for traceability but not inspected.
    Rejected,
}

/// How a characteristic came into the project (data model `origin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Placed by hand.
    Manual,
    /// Read from a box the user drew (M2).
    BoxSelect,
    /// Accepted from automatic detection (M4).
    AutoDetect,
    /// Carried over from an earlier drawing revision (M6).
    CarriedOver,
}

/// How a characteristic is inspected (FR-CHR-02). Empty text means not set.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Inspection {
    /// Inspection method, e.g. `CMM` or `visual`.
    pub method: String,
    /// Gauge or measuring instrument.
    pub gauge: String,
    /// Sampling plan, e.g. `100%` or `5 per lot`.
    pub sampling: String,
    /// Inspection frequency.
    pub frequency: String,
}

/// Where the text of a source region came from (data model `SourceRegion`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextSource {
    /// Region drawn by the user; no text was read.
    Manual,
    /// Text objects of the PDF.
    PdfText,
    /// Optical character recognition.
    Ocr {
        /// OCR engine name.
        engine: String,
        /// Model version of the engine.
        model_version: String,
    },
}

/// The region of a sheet a characteristic was read from (FR-CHR-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SourceRegion {
    /// Sheet that holds the region.
    pub sheet: SheetId,
    /// Oriented box in sheet space.
    pub region: OrientedBox,
    /// Where the text came from.
    pub text_source: TextSource,
    /// Raw recognized text, if any text was read.
    pub raw_text: Option<String>,
}

/// One inspectable characteristic (data model 07, FR-CHR-01, FR-CHR-02).
///
/// Optional values are written as `null` in `project.json`, so every key is always present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Characteristic {
    /// Stable ID, never changes (FR-BAL-09).
    pub id: CharId,
    /// Display number, starting at 1. Follows the placement order while numbering is unlocked
    /// (D-21); kept when numbering is locked (D-23).
    pub number: u32,
    /// What the characteristic describes.
    pub kind: CharacteristicKind,
    /// Text as it appears on the drawing.
    pub requirement_text: String,
    /// Nominal value.
    #[serde(with = "crate::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub nominal: Option<Decimal>,
    /// Unit of nominal, deviations and limits.
    pub unit: Option<Unit>,
    /// Upper deviation, kept for display.
    #[serde(with = "crate::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub upper_dev: Option<Decimal>,
    /// Lower deviation, kept for display.
    #[serde(with = "crate::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub lower_dev: Option<Decimal>,
    /// Upper limit, always stored explicitly.
    #[serde(with = "crate::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub upper_limit: Option<Decimal>,
    /// Lower limit, always stored explicitly.
    #[serde(with = "crate::decimal::serde_str_option")]
    #[schemars(schema_with = "decimal_option_schema")]
    #[cfg_attr(feature = "specta", specta(type = Option<String>))]
    pub lower_limit: Option<Decimal>,
    /// Fit designation such as `H7`.
    pub fit: Option<String>,
    /// Number of features the characteristic stands for, from `4X` or `2 PL` (D-22). At least 1.
    pub quantity: u32,
    /// Importance class.
    pub classification: Classification,
    /// How the characteristic is inspected.
    pub inspection: Inspection,
    /// False for reference and basic dimensions (D-25).
    pub inspect: bool,
    /// Review state.
    pub status: CharacteristicStatus,
    /// How the characteristic came into the project.
    pub origin: Origin,
    /// Regions of the drawing the characteristic was read from (FR-CHR-09).
    pub sources: Vec<SourceRegion>,
    /// Free text.
    pub comment: String,
}

impl Characteristic {
    /// A characteristic placed by hand: kind `other`, no values, quantity 1, inspected,
    /// accepted.
    pub fn manual(id: CharId, number: u32) -> Self {
        Self {
            id,
            number,
            kind: CharacteristicKind::Other,
            requirement_text: String::new(),
            nominal: None,
            unit: None,
            upper_dev: None,
            lower_dev: None,
            upper_limit: None,
            lower_limit: None,
            fit: None,
            quantity: 1,
            classification: Classification::None,
            inspection: Inspection::default(),
            inspect: true,
            status: CharacteristicStatus::Accepted,
            origin: Origin::Manual,
            sources: Vec::new(),
            comment: String::new(),
        }
    }

    /// Sets field values in the given order (FR-CHR-02).
    ///
    /// Limits follow the deviations: if the values change the nominal or a deviation but no
    /// limit, and nominal and both deviations are present afterwards, the limits are set to
    /// nominal plus deviation. Limits set explicitly are kept as given.
    pub fn set_values(&mut self, values: &[FieldValue]) -> Result<(), FieldError> {
        let mut deviation_touched = false;
        let mut limit_touched = false;
        for value in values {
            match value {
                FieldValue::Kind(kind) => self.kind = *kind,
                FieldValue::RequirementText(text) => self.requirement_text = text.trim().into(),
                FieldValue::Nominal(v) => (self.nominal, deviation_touched) = (v.0, true),
                FieldValue::Unit(unit) => self.unit = *unit,
                FieldValue::UpperDev(v) => (self.upper_dev, deviation_touched) = (v.0, true),
                FieldValue::LowerDev(v) => (self.lower_dev, deviation_touched) = (v.0, true),
                FieldValue::UpperLimit(v) => (self.upper_limit, limit_touched) = (v.0, true),
                FieldValue::LowerLimit(v) => (self.lower_limit, limit_touched) = (v.0, true),
                FieldValue::Fit(fit) => {
                    self.fit = fit
                        .as_deref()
                        .map(str::trim)
                        .filter(|f| !f.is_empty())
                        .map(str::to_owned);
                }
                FieldValue::Quantity(0) => return Err(FieldError::ZeroQuantity),
                FieldValue::Quantity(q) => self.quantity = *q,
                FieldValue::Classification(c) => self.classification = *c,
                FieldValue::InspectionMethod(text) => self.inspection.method.clone_from(text),
                FieldValue::Gauge(text) => self.inspection.gauge.clone_from(text),
                FieldValue::Sampling(text) => self.inspection.sampling.clone_from(text),
                FieldValue::Frequency(text) => self.inspection.frequency.clone_from(text),
                FieldValue::Inspect(inspect) => self.inspect = *inspect,
                FieldValue::Status(status) => self.status = *status,
                FieldValue::Comment(text) => self.comment.clone_from(text),
            }
        }
        if deviation_touched
            && !limit_touched
            && let (Some(nominal), Some(upper), Some(lower)) =
                (self.nominal, self.upper_dev, self.lower_dev)
        {
            self.upper_limit = nominal.checked_add(upper);
            self.lower_limit = nominal.checked_add(lower);
        }
        Ok(())
    }
}

/// One field value of a characteristic, used by commands that create or edit characteristics.
///
/// Serialized as `{ "field": "nominal", "value": "12.5" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    /// Kind.
    Kind(CharacteristicKind),
    /// Requirement text; leading and trailing white space is removed.
    RequirementText(String),
    /// Nominal value.
    Nominal(OptionalDecimal),
    /// Unit.
    Unit(Option<Unit>),
    /// Upper deviation.
    UpperDev(OptionalDecimal),
    /// Lower deviation.
    LowerDev(OptionalDecimal),
    /// Upper limit.
    UpperLimit(OptionalDecimal),
    /// Lower limit.
    LowerLimit(OptionalDecimal),
    /// Fit designation; empty text clears it.
    Fit(Option<String>),
    /// Quantity, at least 1.
    Quantity(u32),
    /// Classification.
    Classification(Classification),
    /// Inspection method.
    InspectionMethod(String),
    /// Gauge.
    Gauge(String),
    /// Sampling plan.
    Sampling(String),
    /// Inspection frequency.
    Frequency(String),
    /// Whether the characteristic is inspected.
    Inspect(bool),
    /// Review state.
    Status(CharacteristicStatus),
    /// Comment.
    Comment(String),
}

/// A field value that cannot be stored.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// Quantity 0.
    #[error("quantity must be at least 1")]
    ZeroQuantity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn dec(text: &str) -> Option<Decimal> {
        crate::decimal::parse_decimal(text)
    }

    fn sample() -> Characteristic {
        Characteristic::manual(CharId::from_uuid(Uuid::from_u128(7)), 1)
    }

    #[test]
    fn limits_follow_nominal_and_deviations() {
        let mut c = sample();
        c.set_values(&[
            FieldValue::Nominal(dec("30").into()),
            FieldValue::UpperDev(dec("0.021").into()),
            FieldValue::LowerDev(dec("0").into()),
        ])
        .unwrap();
        assert_eq!((c.upper_limit, c.lower_limit), (dec("30.021"), dec("30")));

        // Changing the nominal later moves the limits with it, exactly.
        c.set_values(&[FieldValue::Nominal(dec("12.5").into())])
            .unwrap();
        assert_eq!(c.upper_limit.unwrap().to_string(), "12.521");
        assert_eq!(c.lower_limit.unwrap().to_string(), "12.5");
    }

    #[test]
    fn explicit_limits_win_over_deviations() {
        let mut c = sample();
        c.set_values(&[
            FieldValue::Nominal(dec("10").into()),
            FieldValue::UpperDev(dec("0.1").into()),
            FieldValue::LowerDev(dec("-0.1").into()),
            FieldValue::UpperLimit(dec("10.05").into()),
        ])
        .unwrap();
        assert_eq!(c.upper_limit, dec("10.05"));
        assert_eq!(c.lower_limit, None);
    }

    #[test]
    fn rejects_zero_quantity_and_trims_text() {
        let mut c = sample();
        assert_eq!(
            c.set_values(&[FieldValue::Quantity(0)]),
            Err(FieldError::ZeroQuantity)
        );
        c.set_values(&[
            FieldValue::RequirementText("  Ø8 H7 ".into()),
            FieldValue::Fit(Some(" ".into())),
        ])
        .unwrap();
        assert_eq!(c.requirement_text, "Ø8 H7");
        assert_eq!(c.fit, None);
    }

    #[test]
    fn field_values_use_adjacent_tags_and_decimal_strings() {
        let value = FieldValue::Nominal(dec("12.50").into());
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "field": "nominal", "value": "12.50" })
        );
        assert_eq!(serde_json::from_value::<FieldValue>(json).unwrap(), value);
        let cleared = serde_json::json!({ "field": "upper_limit", "value": null });
        assert_eq!(
            serde_json::from_value::<FieldValue>(cleared).unwrap(),
            FieldValue::UpperLimit(None.into())
        );
        let float = serde_json::json!({ "field": "nominal", "value": 12.5 });
        assert!(serde_json::from_value::<FieldValue>(float).is_err());
    }
}
