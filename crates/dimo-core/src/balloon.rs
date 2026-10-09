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

/// The numbers that turn a [`BalloonStyle`] and a balloon text into the balloon's size and
/// font size (D-24, FR-BAL-03). The ballooned PDF and the viewport use the same values: the
/// export through [`BalloonStyle::layout`], the viewport through the generated constant
/// `BALLOON_METRICS` in the TypeScript bindings. A test checks that both give the same sizes.
///
/// The rule, with `h` the style size in sheet units:
///
/// - height `h`, outline width `outline_mm` in sheet units, font size `font_share * h`;
/// - text width estimated per character: `narrow_em` for `.` and `,`, `digit_em` for every
///   other character, times the font size;
/// - circle: width `h`; if the text is wider than `circle_text_share * h` the font shrinks
///   until it fits;
/// - rectangle: width `max(h, text + 2 * box_padding_share * h)`;
/// - flag: the rectangle width plus `flag_point_share * h` for the pointed end, at least
///   `flag_min_width_share * h`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonMetrics {
    /// Sheet units per millimeter on the printed sheet (72 / 25.4).
    pub units_per_mm: f64,
    /// Font size as a share of the balloon height.
    pub font_share: f64,
    /// Advance of a digit, in em, also used for any other character except `.` and `,`
    /// (balloon texts are numbers). At least the digit advance of the bundled bold font, so
    /// the estimate never runs short for numbers.
    pub digit_em: f64,
    /// Advance of `.` and `,` in em.
    pub narrow_em: f64,
    /// Share of the circle diameter the text may fill before the font shrinks.
    pub circle_text_share: f64,
    /// Free space left and right of the text in a rectangle or flag, as a share of the height.
    pub box_padding_share: f64,
    /// Extra width of a flag for its pointed end, as a share of the height.
    pub flag_point_share: f64,
    /// Smallest flag width as a share of the height.
    pub flag_min_width_share: f64,
}

/// The balloon layout rule (D-24). See [`BalloonMetrics`].
pub const BALLOON_METRICS: BalloonMetrics = BalloonMetrics {
    units_per_mm: UNITS_PER_MM,
    font_share: 0.5,
    digit_em: 0.6,
    narrow_em: 0.3,
    circle_text_share: 0.78,
    box_padding_share: 0.25,
    flag_point_share: 0.5,
    flag_min_width_share: 1.5,
};

/// Size of one balloon in sheet units, from [`BalloonStyle::layout`]. The shape is drawn in
/// the box of `width` x `height` around the balloon position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonLayout {
    /// Box width.
    pub width: f64,
    /// Box height.
    pub height: f64,
    /// Outline and leader width.
    pub stroke: f64,
    /// Font size (em) of the text.
    pub font_size: f64,
}

impl BalloonMetrics {
    /// Estimated width of `text` in em.
    pub fn text_width_em(&self, text: &str) -> f64 {
        text.chars()
            .map(|c| {
                if matches!(c, '.' | ',') {
                    self.narrow_em
                } else {
                    self.digit_em
                }
            })
            .sum()
    }

    /// Layout of a balloon in `style` showing `text`.
    pub fn layout(&self, style: &BalloonStyle, text: &str) -> BalloonLayout {
        let height = style.size_mm * self.units_per_mm;
        let stroke = style.outline_mm * self.units_per_mm;
        let mut font_size = height * self.font_share;
        let text_width = self.text_width_em(text) * font_size;
        let padded = text_width + 2.0 * self.box_padding_share * height;
        let width = match style.shape {
            BalloonShape::Circle => {
                let room = height * self.circle_text_share;
                if text_width > room {
                    font_size *= room / text_width;
                }
                height
            }
            BalloonShape::Rectangle => height.max(padded),
            BalloonShape::Flag => {
                (height * self.flag_min_width_share).max(padded + height * self.flag_point_share)
            }
        };
        BalloonLayout {
            width,
            height,
            stroke,
            font_size,
        }
    }
}

impl BalloonStyle {
    /// Size of a balloon in this style showing `text`, by the rule of [`BALLOON_METRICS`].
    pub fn layout(&self, text: &str) -> BalloonLayout {
        BALLOON_METRICS.layout(self, text)
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
    fn layout_follows_the_metrics() {
        let circle = BalloonStyle::default();
        let one = circle.layout("7");
        assert!((one.height - 7.0 * UNITS_PER_MM).abs() < 1e-12);
        assert!((one.width - one.height).abs() < 1e-12);
        assert!((one.stroke - 0.35 * UNITS_PER_MM).abs() < 1e-12);
        assert!((one.font_size - one.height / 2.0).abs() < 1e-12);
        // Four digits do not fit at full size: the circle keeps its size, the font shrinks.
        let long = circle.layout("1234");
        assert!((long.width - one.width).abs() < 1e-12);
        assert!(long.font_size < one.font_size);
        assert!((4.0 * 0.6 * long.font_size - 0.78 * long.height).abs() < 1e-9);
        // Boxes grow instead.
        let rect = BalloonStyle {
            shape: BalloonShape::Rectangle,
            ..BalloonStyle::default()
        };
        let wide = rect.layout("1234");
        assert!((wide.font_size - one.font_size).abs() < 1e-12);
        assert!((wide.width - (2.4 * wide.font_size + 0.5 * wide.height)).abs() < 1e-9);
        let short = rect.layout("1");
        assert!((short.width - short.height).abs() < 1e-12);
        let flag = BalloonStyle {
            shape: BalloonShape::Flag,
            ..BalloonStyle::default()
        };
        let flag_one = flag.layout("1");
        assert!((flag_one.width - 1.5 * flag_one.height).abs() < 1e-12);
        assert!(flag.layout("12345").width > flag_one.width);
        assert!((BALLOON_METRICS.text_width_em("1.2") - 1.5).abs() < 1e-12);
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
