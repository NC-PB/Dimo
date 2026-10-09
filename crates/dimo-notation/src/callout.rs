//! The structured form of one dimension or tolerance callout and its canonical printer.
//!
//! The types describe what is written on the drawing, not what it means. Turning a callout into
//! limits (fit tables, general tolerances, unit conversion) is the job of `dimo-tolerance`
//! (spec 08 stage 7).

use std::fmt::{self, Write as _};

use dimo_core::characteristic::{CharacteristicKind, Unit};
use rust_decimal::Decimal;

/// One parsed callout, for example `4X Ø6.6 THRU`, `Ø30 H7 +0.0203 -0` or `(42)`
/// (spec 08 stage 6, FR-REC-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Callout {
    /// Number of identical features (`4X`, `4x`, `4 ×`, `2 PL`). `None` when not written.
    /// D-22: one balloon with this quantity.
    pub quantity: Option<u32>,
    /// What the callout dimensions, from the prefix (`Ø`, `R`, `M`, ...) or the shape of the value.
    pub kind: Kind,
    /// The nominal value. For an angle in degrees, for a chamfer the leg length, for a thread
    /// the major diameter, for a limit dimension the first written limit.
    pub nominal: Number,
    /// ISO 286 fit designation such as `H7` or `H7/g6`.
    pub fit: Option<Fit>,
    /// Tolerance written on the callout. `None` when the callout has only a nominal or a fit.
    pub tolerance: Option<Tolerance>,
    /// Text after the dimension, in written order.
    pub suffixes: Vec<Suffix>,
    /// Reference dimension, written in parentheses `(42)` or with `REF`. D-25, FR-CHR-08.
    pub reference: bool,
    /// Basic (theoretically exact) dimension. In text form written in square brackets `[42]`;
    /// the framed form is detected from vector paths by the caller. D-25, FR-CHR-08.
    pub basic: bool,
    /// Unit written on the callout: `Deg` for angles, `In` for `"` or `in`, `Mm` for `mm`.
    /// `None` when no unit is written; the drawing unit applies then (D-20: never inferred
    /// from the sheet size).
    pub unit: Option<Unit>,
}

/// The kind of a callout, with the parts that only some kinds have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Linear size or distance without a prefix.
    Linear,
    /// Diameter: `Ø`, `⌀` or `DIA`.
    Diameter,
    /// Radius: `R`.
    Radius,
    /// Spherical radius: `SR`.
    SphericalRadius,
    /// Spherical diameter: `SØ`.
    SphericalDiameter,
    /// Angle: the nominal carries `°` (decimal degrees or degrees, minutes, seconds).
    Angle,
    /// Chamfer: `1x45°` (angle given) or `C1` (45 degrees implied, angle `None`).
    Chamfer {
        /// The chamfer angle in degrees. `None` for the `C` form.
        angle: Option<Number>,
    },
    /// ISO metric thread: `M8`, `M8x1.25`, `M8x1.25-6H`.
    Thread {
        /// Pitch, `None` for coarse pitch threads written without it.
        pitch: Option<Number>,
        /// Tolerance class such as `6H`, `6g` or `5H6H`.
        class: Option<String>,
    },
    /// Depth of a hole or pocket written on its own: `↧10`.
    Depth,
}

impl Kind {
    /// The characteristic kind of the domain model. A spherical diameter maps to `Diameter`.
    pub fn characteristic_kind(&self) -> CharacteristicKind {
        match self {
            Self::Linear => CharacteristicKind::Linear,
            Self::Diameter | Self::SphericalDiameter => CharacteristicKind::Diameter,
            Self::Radius => CharacteristicKind::Radius,
            Self::SphericalRadius => CharacteristicKind::SphericalRadius,
            Self::Angle => CharacteristicKind::Angle,
            Self::Chamfer { .. } => CharacteristicKind::Chamfer,
            Self::Thread { .. } => CharacteristicKind::Thread,
            Self::Depth => CharacteristicKind::Depth,
        }
    }

    /// True for the kinds that can carry an ISO 286 fit.
    pub fn takes_fit(&self) -> bool {
        matches!(
            self,
            Self::Linear
                | Self::Diameter
                | Self::Radius
                | Self::SphericalRadius
                | Self::SphericalDiameter
                | Self::Depth
        )
    }
}

/// A number as written. `value` is exact for decimals and inch fractions (rule 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    /// The value. Signed for deviations, keeps the written decimal places (`90.0` has scale 1),
    /// which decimal place rules need (FR-TOL-06). For degrees, minutes and seconds it is the
    /// value in degrees, rounded to 28 digits when not exact (for example `10'`).
    pub value: Decimal,
    /// How the number was written.
    pub form: NumberForm,
}

/// How a number was written. Needed to print it back and to see inch notation (D-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberForm {
    /// Decimal with a leading digit: `12`, `0.25`, `12,5`.
    Decimal,
    /// Decimal without a leading zero: `.250`. Inch notation (D-20).
    NoLeadingZero,
    /// Inch fraction: `1/4`, `1 1/4`. The denominator is a power of two from 2 to 128 and the
    /// numerator is smaller than the denominator, so the value is exact. Inch notation (D-20).
    Fraction {
        /// Whole part, 0 for a plain fraction.
        whole: u32,
        /// Numerator.
        numerator: u32,
        /// Denominator.
        denominator: u32,
    },
    /// Degrees, minutes and optional seconds: `30°15'`, `0°0'30"`.
    DegMinSec {
        /// Whole degrees.
        degrees: u32,
        /// Minutes, below 60.
        minutes: u32,
        /// Seconds, below 60.
        seconds: Option<u32>,
    },
}

impl Number {
    /// A plain decimal number.
    pub fn decimal(value: Decimal) -> Self {
        Self {
            value,
            form: NumberForm::Decimal,
        }
    }

    /// An inch fraction `whole numerator/denominator`. `None` unless the denominator is a power
    /// of two from 2 to 128 and `1 <= numerator < denominator`, which keeps the value exact.
    pub fn fraction(whole: u32, numerator: u32, denominator: u32) -> Option<Self> {
        if !is_inch_fraction(numerator, denominator) {
            return None;
        }
        let part = Decimal::from(numerator).checked_div(Decimal::from(denominator))?;
        let value = Decimal::from(whole).checked_add(part)?;
        Some(Self {
            value,
            form: NumberForm::Fraction {
                whole,
                numerator,
                denominator,
            },
        })
    }

    /// An angle in degrees, minutes and optional seconds. `None` unless minutes and seconds
    /// are below 60. The value in degrees is rounded to 28 digits when not exact.
    pub fn deg_min_sec(degrees: u32, minutes: u32, seconds: Option<u32>) -> Option<Self> {
        if minutes >= 60 || seconds.is_some_and(|s| s >= 60) {
            return None;
        }
        let mut value = Decimal::from(degrees)
            .checked_add(Decimal::from(minutes).checked_div(Decimal::from(60))?)?;
        if let Some(seconds) = seconds {
            value = value.checked_add(Decimal::from(seconds).checked_div(Decimal::from(3600))?)?;
        }
        Some(Self {
            value,
            form: NumberForm::DegMinSec {
                degrees,
                minutes,
                seconds,
            },
        })
    }

    /// True when the number is written in inch notation (fraction or no leading zero, D-20).
    pub fn is_inch_notation(&self) -> bool {
        matches!(
            self.form,
            NumberForm::NoLeadingZero | NumberForm::Fraction { .. }
        )
    }

    /// The magnitude as written, without a sign and without a degree sign for decimals.
    fn magnitude(&self) -> String {
        match &self.form {
            NumberForm::Decimal => self.value.abs().to_string(),
            NumberForm::NoLeadingZero => {
                let text = self.value.abs().to_string();
                match text.strip_prefix('0') {
                    Some(rest) if rest.starts_with('.') => rest.to_owned(),
                    _ => text,
                }
            }
            NumberForm::Fraction {
                whole,
                numerator,
                denominator,
            } => {
                if *whole == 0 {
                    format!("{numerator}/{denominator}")
                } else {
                    format!("{whole} {numerator}/{denominator}")
                }
            }
            NumberForm::DegMinSec {
                degrees,
                minutes,
                seconds,
            } => match seconds {
                Some(seconds) => format!("{degrees}°{minutes}'{seconds}\""),
                None => format!("{degrees}°{minutes}'"),
            },
        }
    }

    /// The number as written, with `°` appended for decimals when `degrees` is set.
    fn print(&self, degrees: bool) -> String {
        let mut text = self.magnitude();
        if degrees && !matches!(self.form, NumberForm::DegMinSec { .. }) {
            text.push('°');
        }
        text
    }

    /// Like [`Number::print`] with an explicit sign (`+` or `-`).
    fn print_signed(&self, degrees: bool) -> String {
        let sign = if self.value.is_sign_negative() {
            '-'
        } else {
            '+'
        };
        format!("{sign}{}", self.print(degrees))
    }
}

/// Inch fraction denominators are powers of two from 2 to 128; the numerator is smaller.
pub(crate) fn is_inch_fraction(numerator: u32, denominator: u32) -> bool {
    (2..=128).contains(&denominator)
        && denominator.is_power_of_two()
        && (1..denominator).contains(&numerator)
}

/// One ISO 286 tolerance class: fundamental deviation letters and grade, e.g. `H7`, `js6`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToleranceClass {
    /// Fundamental deviation letters: upper case for holes (`H`, `JS`), lower case for
    /// shafts (`g`, `js`).
    pub deviation: String,
    /// International tolerance grade, 1 to 18.
    pub grade: u8,
}

impl ToleranceClass {
    /// True for a hole (upper case letters).
    pub fn is_hole(&self) -> bool {
        self.deviation.chars().all(|c| c.is_ascii_uppercase())
    }
}

impl fmt::Display for ToleranceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.deviation, self.grade)
    }
}

/// A fit designation: one class (`H7`) or a hole and shaft pair (`H7/g6`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fit {
    /// The class written first.
    pub first: ToleranceClass,
    /// The class after `/`, if any.
    pub second: Option<ToleranceClass>,
}

impl fmt::Display for Fit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.second {
            Some(second) => write!(f, "{}/{second}", self.first),
            None => write!(f, "{}", self.first),
        }
    }
}

/// A tolerance written on the callout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tolerance {
    /// `±0.1`. The value is unsigned.
    Symmetric(Number),
    /// Two deviations, signed: `+0.1 -0.05`, `+0.1/-0.05`, `+0.1 0`, `+0 -0.6`.
    /// `upper` is the one written first (stacked: the upper line). An unsigned `0` is stored as
    /// positive zero. The interpreter checks which one is larger.
    Deviations {
        /// The deviation written first.
        upper: Number,
        /// The deviation written second.
        lower: Number,
    },
    /// Limit dimension `12.02/11.98`, `12.02-11.98` or two stacked lines `12.02 11.98`. The
    /// callout's nominal is the first written limit, this is the second, as written (which one
    /// is larger is not checked here).
    Limits {
        /// The second written limit.
        other: Number,
    },
    /// `MIN`: the nominal is a lower limit.
    Min,
    /// `MAX`: the nominal is an upper limit.
    Max,
}

/// Text after the dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suffix {
    /// `THRU`.
    Thru,
    /// Depth `↧10`.
    Depth(Number),
}

impl Callout {
    /// The characteristic kind of the domain model.
    pub fn characteristic_kind(&self) -> CharacteristicKind {
        self.kind.characteristic_kind()
    }

    /// True when the callout is in inch: an inch unit marker, or any number written as an inch
    /// fraction or without a leading zero (D-20).
    pub fn is_inch(&self) -> bool {
        if self.unit == Some(Unit::In) {
            return true;
        }
        let mut numbers: Vec<&Number> = vec![&self.nominal];
        match &self.tolerance {
            Some(Tolerance::Symmetric(n) | Tolerance::Limits { other: n }) => numbers.push(n),
            Some(Tolerance::Deviations { upper, lower }) => {
                numbers.push(upper);
                numbers.push(lower);
            }
            Some(Tolerance::Min | Tolerance::Max) | None => {}
        }
        for suffix in &self.suffixes {
            if let Suffix::Depth(n) = suffix {
                numbers.push(n);
            }
        }
        numbers.iter().any(|n| n.is_inch_notation())
    }

    /// Print the callout in its canonical text form. Parsing the result with
    /// [`crate::parse_callout`] gives back an equal callout (NFR-REC parser property tests).
    ///
    /// Canonical choices: `Ø` for diameters, `X` for quantities (` N PL` after the dimension
    /// for angles, because `2X 30°` reads as a chamfer), `x` in chamfers and threads, decimal
    /// point, hyphen minus, `"` for inch and `mm` right after the nominal, limits as `a / b`,
    /// deviations always signed, reference in parentheses, basic in square brackets, both
    /// together as `[..] REF`.
    pub fn to_canonical(&self) -> String {
        let angle = matches!(self.kind, Kind::Angle);
        let mut core = String::new();
        match &self.kind {
            Kind::Diameter => core.push('Ø'),
            Kind::Radius => core.push('R'),
            Kind::SphericalRadius => core.push_str("SR"),
            Kind::SphericalDiameter => core.push_str("SØ"),
            Kind::Chamfer { angle: None } => core.push('C'),
            Kind::Linear | Kind::Angle | Kind::Chamfer { angle: Some(_) } => {}
            Kind::Thread { .. } => core.push('M'),
            Kind::Depth => core.push('↧'),
        }
        core.push_str(&self.nominal.print(angle));
        match self.unit {
            Some(Unit::In) => core.push('"'),
            Some(Unit::Mm) => core.push_str("mm"),
            Some(Unit::Deg) | None => {}
        }
        match &self.kind {
            Kind::Chamfer { angle: Some(a) } => {
                // `1mm x 45°`: a unit word must not run into the `x`.
                core.push_str(if self.unit == Some(Unit::Mm) {
                    " x "
                } else {
                    "x"
                });
                core.push_str(&a.print(true));
            }
            Kind::Thread { pitch, class } => {
                if let Some(pitch) = pitch {
                    core.push('x');
                    core.push_str(&pitch.print(false));
                }
                if let Some(class) = class {
                    core.push('-');
                    core.push_str(class);
                }
            }
            _ => {}
        }
        if let Some(fit) = &self.fit {
            core.push(' ');
            core.push_str(&fit.to_string());
        }
        match &self.tolerance {
            None => {}
            Some(Tolerance::Symmetric(n)) => {
                core.push_str(" ±");
                core.push_str(&n.print(angle));
            }
            Some(Tolerance::Deviations { upper, lower }) => {
                core.push(' ');
                core.push_str(&upper.print_signed(angle));
                core.push(' ');
                core.push_str(&lower.print_signed(angle));
            }
            Some(Tolerance::Limits { other }) => {
                core.push_str(" / ");
                core.push_str(&other.print(angle));
            }
            Some(Tolerance::Min) => core.push_str(" MIN"),
            Some(Tolerance::Max) => core.push_str(" MAX"),
        }

        let mut out = String::new();
        let quantity_as_suffix = angle;
        if let (Some(q), false) = (self.quantity, quantity_as_suffix) {
            let _ = write!(out, "{q}X ");
        }
        if self.basic {
            out.push('[');
            out.push_str(&core);
            out.push(']');
        } else if self.reference {
            out.push('(');
            out.push_str(&core);
            out.push(')');
        } else {
            out.push_str(&core);
        }
        for suffix in &self.suffixes {
            match suffix {
                Suffix::Thru => out.push_str(" THRU"),
                Suffix::Depth(n) => {
                    out.push_str(" ↧");
                    out.push_str(&n.print(false));
                }
            }
        }
        if let (Some(q), true) = (self.quantity, quantity_as_suffix) {
            let _ = write!(out, " {q} PL");
        }
        if self.basic && self.reference {
            out.push_str(" REF");
        }
        out
    }
}

impl fmt::Display for Callout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_canonical())
    }
}
