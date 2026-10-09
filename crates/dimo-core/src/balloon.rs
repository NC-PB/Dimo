//! Balloons and their style (data model `Balloon`, D-24, FR-BAL-12).

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::id::{BalloonId, CharId, SheetId};

/// PDF user units per millimeter (72 units per inch).
pub const UNITS_PER_MM: f64 = 72.0 / 25.4;

/// Outline shape of a balloon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum BalloonShape {
    /// Circle (D-24 default).
    Circle,
    /// Flag: a rectangle with a pointed side.
    Flag,
    /// Rectangle.
    Rectangle,
}

/// An sRGB color as `#RRGGBB` with uppercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct Color(String);

/// Error for text that is not a `#RRGGBB` color.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid color {0:?}, expected #RRGGBB")]
pub struct ColorError(String);

impl Color {
    /// Parses `#RRGGBB` (any case) and stores it in uppercase.
    pub fn parse(text: &str) -> Result<Self, ColorError> {
        match text.strip_prefix('#') {
            Some(hex) if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
                Ok(Self(text.to_ascii_uppercase()))
            }
            _ => Err(ColorError(text.to_owned())),
        }
    }

    /// Red, green and blue components.
    pub fn rgb(&self) -> [u8; 3] {
        let channel = |i: usize| u8::from_str_radix(&self.0[i..i + 2], 16).unwrap_or(0);
        [channel(1), channel(3), channel(5)]
    }

    /// The `#RRGGBB` text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn known(text: &'static str) -> Self {
        Self(text.to_owned())
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Color {
    type Error = ColorError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        color.0
    }
}

impl JsonSchema for Color {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Color".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string", "pattern": "^#[0-9A-F]{6}$" })
    }
}

/// How balloons are drawn (D-24). The project holds one default, balloons may override parts.
///
/// Sizes are millimeters on the sheet printed at its nominal paper size; multiply by
/// [`UNITS_PER_MM`] for sheet units. Because they live in sheet space they scale with the sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonStyle {
    /// Outline shape.
    pub shape: BalloonShape,
    /// Diameter of a circle, height of a flag or rectangle, in mm.
    pub size_mm: f64,
    /// Outline width in mm.
    pub outline_mm: f64,
    /// Outline and leader color.
    pub outline_color: Color,
    /// Fill color.
    pub fill_color: Color,
    /// Color of the number.
    pub text_color: Color,
    /// Whether a leader line is drawn from the balloon to its anchor.
    pub leader: bool,
}

impl Default for BalloonStyle {
    /// D-24: circle, 7 mm, white fill, 0.35 mm outline in `#0057B8`, black number, leader on.
    fn default() -> Self {
        Self {
            shape: BalloonShape::Circle,
            size_mm: 7.0,
            outline_mm: 0.35,
            outline_color: Color::known("#0057B8"),
            fill_color: Color::known("#FFFFFF"),
            text_color: Color::known("#000000"),
            leader: true,
        }
    }
}

impl BalloonStyle {
    /// True if sizes are finite and positive.
    pub fn is_valid(&self) -> bool {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        positive(self.size_mm) && positive(self.outline_mm)
    }

    /// This style with the fields set in `style_override` replaced.
    #[must_use]
    pub fn with_override(&self, style_override: &BalloonStyleOverride) -> Self {
        let o = style_override;
        Self {
            shape: o.shape.unwrap_or(self.shape),
            size_mm: o.size_mm.unwrap_or(self.size_mm),
            outline_mm: o.outline_mm.unwrap_or(self.outline_mm),
            outline_color: o
                .outline_color
                .clone()
                .unwrap_or_else(|| self.outline_color.clone()),
            fill_color: o
                .fill_color
                .clone()
                .unwrap_or_else(|| self.fill_color.clone()),
            text_color: o
                .text_color
                .clone()
                .unwrap_or_else(|| self.text_color.clone()),
            leader: o.leader.unwrap_or(self.leader),
        }
    }
}

/// Per balloon deviations from the project default style. `null` means "use the default".
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonStyleOverride {
    /// Outline shape.
    pub shape: Option<BalloonShape>,
    /// Size in mm.
    pub size_mm: Option<f64>,
    /// Outline width in mm.
    pub outline_mm: Option<f64>,
    /// Outline and leader color.
    pub outline_color: Option<Color>,
    /// Fill color.
    pub fill_color: Option<Color>,
    /// Number color.
    pub text_color: Option<Color>,
    /// Leader line on or off.
    pub leader: Option<bool>,
}

impl BalloonStyleOverride {
    /// True if nothing is overridden.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// `self` with every field that is set in `other` taken from `other`.
    #[must_use]
    pub fn merged(&self, other: &Self) -> Self {
        Self {
            shape: other.shape.or(self.shape),
            size_mm: other.size_mm.or(self.size_mm),
            outline_mm: other.outline_mm.or(self.outline_mm),
            outline_color: other
                .outline_color
                .clone()
                .or_else(|| self.outline_color.clone()),
            fill_color: other.fill_color.clone().or_else(|| self.fill_color.clone()),
            text_color: other.text_color.clone().or_else(|| self.text_color.clone()),
            leader: other.leader.or(self.leader),
        }
    }

    /// True if all set sizes are finite and positive.
    pub fn is_valid(&self) -> bool {
        let positive = |v: Option<f64>| v.is_none_or(|v| v.is_finite() && v > 0.0);
        positive(self.size_mm) && positive(self.outline_mm)
    }
}

/// A balloon on a sheet that marks a characteristic (data model `Balloon`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Balloon {
    /// Stable ID.
    pub id: BalloonId,
    /// The characteristic this balloon marks. The number shown is the characteristic number.
    pub characteristic: CharId,
    /// Sheet the balloon is drawn on.
    pub sheet: SheetId,
    /// Center of the balloon in sheet space.
    pub position: Point,
    /// End of the leader line in sheet space, on the feature.
    pub anchor: Point,
    /// Deviations from the project default style.
    pub style: BalloonStyleOverride,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_is_d24() {
        let style = BalloonStyle::default();
        assert_eq!(style.shape, BalloonShape::Circle);
        assert!((style.size_mm - 7.0).abs() < f64::EPSILON);
        assert_eq!(style.outline_color.rgb(), [0x00, 0x57, 0xB8]);
        assert!(style.leader && style.is_valid());
    }

    #[test]
    fn colors_are_normalized_and_checked() {
        assert_eq!(Color::parse("#0057b8").unwrap().as_str(), "#0057B8");
        for bad in ["0057B8", "#0057B", "#0057BG", "#0057B80", ""] {
            assert!(Color::parse(bad).is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<Color>("\"blue\"").is_err());
    }

    #[test]
    fn overrides_merge_and_apply() {
        let flag = BalloonStyleOverride {
            shape: Some(BalloonShape::Flag),
            ..Default::default()
        };
        let no_leader = BalloonStyleOverride {
            leader: Some(false),
            ..Default::default()
        };
        let merged = flag.merged(&no_leader);
        let style = BalloonStyle::default().with_override(&merged);
        assert_eq!(style.shape, BalloonShape::Flag);
        assert!(!style.leader);
        assert!(BalloonStyleOverride::default().is_empty());
        assert!(
            !BalloonStyleOverride {
                size_mm: Some(-1.0),
                ..Default::default()
            }
            .is_valid()
        );
    }
}
