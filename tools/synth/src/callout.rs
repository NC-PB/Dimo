//! The callouts the generator writes and their expected values (T0.11, T2.9).
//!
//! Every callout is one of the [`Form`]s, the "common callouts" of the M2 exit criterion
//! (docs/plan/M2.md T2.9). The expected limits come from small tables written into this file
//! from ISO 2768-1 and ISO 286, never from `dimo-tolerance`, so the evaluation compares the
//! engine against an independent truth. The tables are drafts like every table an agent writes
//! (D-43): the owner checks them together with the shipped tables.

use dimo_core::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use rust_decimal::Decimal;

use crate::Rng;

pub(crate) const MINUS: char = '\u{2212}';
pub(crate) const OSLASH: char = 'Ø';
pub(crate) const PLUS_MINUS: char = '±';
pub(crate) const DEGREE: char = '°';

/// The general tolerance class the expected limits of untoleranced callouts assume: ISO 2768-1
/// class m (written into the truth as `tolerance_settings`).
pub const GENERAL_TABLE: &str = "iso-2768-1";
/// Version of [`GENERAL_TABLE`] the truth names.
pub const GENERAL_TABLE_VERSION: u32 = 1;
/// Class column of [`GENERAL_TABLE`].
pub const GENERAL_CLASS: &str = "m";

/// ISO 2768-1:1989 table 1, class m: (upper bound of the size range in mm, inclusive, ±value in
/// hundredths of a mm). The first range starts at 0.5 mm, every range is "over ... up to".
const LINEAR_M: &[(i64, i64)] = &[
    (3, 10),
    (6, 10),
    (30, 20),
    (120, 30),
    (400, 50),
    (1000, 80),
    (2000, 120),
    (4000, 200),
];

/// ISO 2768-1:1989 table 2 (broken edges: external radii and chamfer heights), class m: (upper
/// bound in mm, inclusive, ±value in tenths of a mm); `None` is open ended.
const RADIUS_CHAMFER_M: &[(Option<i64>, i64)] = &[(Some(3), 2), (Some(6), 5), (None, 10)];

/// ISO 2768-1:1989 table 3, class m, by the length of the shorter leg: (upper bound in mm,
/// inclusive, ± minutes of arc). Only the rows the generator uses: their values are whole or
/// half degrees, so the limits are exact decimals.
const ANGULAR_M: &[(i64, i64)] = &[(10, 60), (50, 30)];

/// Fit table, micrometres: (nominal mm, designation, upper deviation, lower deviation).
/// Values from ISO 286-2 limit deviation tables (ISO 286-1 fundamental deviations plus IT
/// grades) for the ranges over 3 up to 6 mm (5), over 6 up to 10 mm (8, 10), over 18 up to
/// 30 mm (20) and over 30 up to 50 mm (40). The two letter classes `cd`, `ef`, `fg` and `CD`,
/// `EF`, `FG` are the intermediate deviations the standard defines for small sizes.
/// DRAFT, not verified by the owner (AGENTS.md rule 7, D-43).
pub const FITS: &[(i64, &str, i64, i64)] = &[
    (10, "H7", 15, 0),
    (10, "h6", 0, -9),
    (10, "f7", -13, -28),
    (10, "g6", -5, -14),
    (10, "k6", 10, 1),
    (20, "H7", 21, 0),
    (20, "h6", 0, -13),
    (20, "f7", -20, -41),
    (20, "g6", -7, -20),
    (20, "k6", 15, 2),
    (40, "H7", 25, 0),
    (40, "h6", 0, -16),
    (40, "f7", -25, -50),
    (40, "g6", -9, -25),
    (40, "k6", 18, 2),
    (5, "cd8", -46, -64),
    (5, "ef7", -14, -26),
    (5, "fg6", -6, -14),
    (8, "cd7", -56, -71),
    (8, "ef7", -18, -33),
    (8, "fg6", -8, -17),
    (8, "CD8", 78, 56),
    (8, "EF8", 40, 18),
    (8, "FG7", 23, 8),
];

/// Metric threads: (major diameter, coarse pitch in hundredths of a mm, fine pitch or 0).
const THREADS: &[(i64, i64, i64)] = &[
    (3, 50, 0),
    (4, 70, 0),
    (5, 80, 0),
    (6, 100, 75),
    (8, 125, 100),
    (10, 150, 125),
    (12, 175, 150),
    (16, 200, 150),
    (20, 250, 150),
];

/// The forms of the M2 exit criterion. The evaluation classifies truth entries from their
/// fields, so this list only drives the generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Linear size without tolerance: general tolerance.
    Linear,
    /// Diameter without tolerance.
    Diameter,
    /// Radius without tolerance.
    Radius,
    /// Angle without tolerance (general tolerance needs the shorter leg).
    Angle,
    /// Chamfer without tolerance, `1x45°` or `C1`.
    Chamfer,
    /// `±` tolerance.
    Symmetric,
    /// Two different stacked deviations.
    Asymmetric,
    /// Stacked deviations, one of them zero.
    OneSided,
    /// Limit dimension: two stacked limits without a main text.
    Limit,
    /// Fit designation only.
    Fit,
    /// Fit designation with printed deviations.
    FitPrinted,
    /// Metric thread.
    Thread,
    /// Reference dimension in parentheses.
    Reference,
}

impl Form {
    /// Every form, in the order the generator picks from.
    pub const ALL: [Form; 13] = [
        Form::Linear,
        Form::Diameter,
        Form::Radius,
        Form::Angle,
        Form::Chamfer,
        Form::Symmetric,
        Form::Asymmetric,
        Form::OneSided,
        Form::Limit,
        Form::Fit,
        Form::FitPrinted,
        Form::Thread,
        Form::Reference,
    ];
}

/// Where a text object of a callout sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    /// The main text, at the start.
    Main,
    /// Small upper deviation right of the main text.
    Upper,
    /// Small lower deviation right of the main text.
    Lower,
    /// Upper limit of a limit dimension, full size at the start.
    LimitUpper,
    /// Lower limit of a limit dimension, below the upper one.
    LimitLower,
}

/// One generated callout: its text objects and the truth values.
#[derive(Debug, Clone)]
pub(crate) struct Callout {
    pub form: Form,
    pub kind: CharacteristicKind,
    /// Text objects in reading order (main, upper, lower).
    pub texts: Vec<(String, Slot)>,
    pub nominal: Decimal,
    pub unit: Unit,
    pub fit: Option<String>,
    pub rule: ToleranceRule,
    /// Upper and lower limit.
    pub limits: Option<(Decimal, Decimal)>,
    pub inspect: bool,
    pub review_note: Option<String>,
}

impl Callout {
    /// The requirement text: the text objects joined with one space (truth convention).
    pub fn requirement_text(&self) -> String {
        self.texts
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Number formatting: decimal point or comma, Unicode minus for negative values.
#[derive(Debug, Clone, Copy)]
struct Fmt {
    comma: bool,
}

impl Fmt {
    fn num(self, v: Decimal) -> String {
        let text = v.abs().to_string();
        let text = if self.comma {
            text.replace('.', ",")
        } else {
            text
        };
        if v.is_sign_negative() && !v.is_zero() {
            format!("{MINUS}{text}")
        } else {
            text
        }
    }

    /// A deviation with its sign: `+0.1`, `−0.05`; zero as `zero`.
    fn dev(self, v: Decimal, zero: &str) -> String {
        if v.is_zero() {
            zero.to_owned()
        } else if v.is_sign_negative() {
            self.num(v)
        } else {
            format!("+{}", self.num(v))
        }
    }
}

fn pick<T: Copy>(rng: &mut Rng, items: &[T]) -> T {
    let last = u32::try_from(items.len() - 1).unwrap_or(0);
    items[rng.range(0, last) as usize]
}

/// A size from `lo` to `hi` with 0 to `max_scale` decimals, the last decimal never 0 (so the
/// printed text equals the decimal).
fn size(rng: &mut Rng, lo: u32, hi: u32, max_scale: u32) -> Decimal {
    let scale = rng.range(0, max_scale);
    let mut mantissa = i64::from(rng.range(lo, hi));
    for digit in 0..scale {
        let d = if digit + 1 == scale {
            rng.range(1, 9)
        } else {
            rng.range(0, 9)
        };
        mantissa = mantissa * 10 + i64::from(d);
    }
    Decimal::new(mantissa, scale)
}

fn linear_m(size: Decimal) -> Decimal {
    LINEAR_M
        .iter()
        .find(|(max, _)| size <= Decimal::from(*max))
        .map_or(Decimal::ZERO, |(_, v)| Decimal::new(*v, 2))
}

fn radius_chamfer_m(size: Decimal) -> Decimal {
    RADIUS_CHAMFER_M
        .iter()
        .find(|(max, _)| max.is_none_or(|m| size <= Decimal::from(m)))
        .map_or(Decimal::ZERO, |(_, v)| Decimal::new(*v, 1))
}

/// General tolerance note shared by the untoleranced forms.
fn general_note() -> String {
    format!("Limits from ISO 2768-1 class {GENERAL_CLASS} (generator table, draft, D-43).")
}

/// What every form shares: the form, the number format and the diameter choice.
struct Draw {
    form: Form,
    f: Fmt,
    prefix: String,
    length_kind: CharacteristicKind,
}

impl Draw {
    fn base(
        &self,
        kind: CharacteristicKind,
        texts: Vec<(String, Slot)>,
        nominal: Decimal,
        rule: ToleranceRule,
        limits: Option<(Decimal, Decimal)>,
    ) -> Callout {
        Callout {
            form: self.form,
            kind,
            texts,
            nominal,
            unit: Unit::Mm,
            fit: None,
            rule,
            limits,
            inspect: true,
            review_note: None,
        }
    }
}

/// Limits of `n ± t`.
fn sym(n: Decimal, t: Decimal) -> (Decimal, Decimal) {
    (n + t, n - t)
}

/// Generate one callout. The form is picked uniformly; every fourth callout of a drawing is
/// rotated by the caller.
pub(crate) fn make_callout(rng: &mut Rng) -> Callout {
    let form = pick(rng, &Form::ALL);
    // One callout in four writes decimal commas.
    let f = Fmt {
        comma: rng.range(0, 3) == 0,
    };
    let diameter = rng.range(0, 2) == 0;
    let d = Draw {
        form,
        f,
        prefix: if diameter {
            OSLASH.to_string()
        } else {
            String::new()
        },
        length_kind: if diameter {
            CharacteristicKind::Diameter
        } else {
            CharacteristicKind::Linear
        },
    };
    match form {
        Form::Linear | Form::Diameter | Form::Radius | Form::Angle | Form::Chamfer => {
            untoleranced(&d, rng)
        }
        Form::Symmetric | Form::Asymmetric | Form::OneSided | Form::Limit => toleranced(&d, rng),
        Form::Fit | Form::FitPrinted | Form::Thread | Form::Reference => fits_and_others(&d, rng),
    }
}

/// Sizes without a tolerance: ISO 2768-1 class m.
fn untoleranced(draw: &Draw, rng: &mut Rng) -> Callout {
    let (form, f) = (draw.form, draw.f);
    let base = |kind, texts, nominal, rule, limits| draw.base(kind, texts, nominal, rule, limits);
    match form {
        Form::Linear | Form::Diameter => {
            let n = size(rng, 5, 250, 2);
            let (p, kind) = if form == Form::Diameter {
                (OSLASH.to_string(), CharacteristicKind::Diameter)
            } else {
                (String::new(), CharacteristicKind::Linear)
            };
            let mut c = base(
                kind,
                vec![(format!("{p}{}", f.num(n)), Slot::Main)],
                n,
                ToleranceRule::General,
                Some(sym(n, linear_m(n))),
            );
            c.review_note = Some(general_note());
            c
        }
        Form::Radius => {
            let n = size(rng, 1, 30, 1);
            let mut c = base(
                CharacteristicKind::Radius,
                vec![(format!("R{}", f.num(n)), Slot::Main)],
                n,
                ToleranceRule::General,
                Some(sym(n, radius_chamfer_m(n))),
            );
            c.review_note = Some(general_note());
            c
        }
        Form::Angle => {
            let n = Decimal::from(pick(rng, &[15, 30, 45, 60, 75, 90, 120, 135]));
            let (leg, minutes) = if rng.range(0, 1) == 0 {
                (8, ANGULAR_M[0].1)
            } else {
                (40, ANGULAR_M[1].1)
            };
            debug_assert!(leg <= ANGULAR_M[1].0);
            let t = Decimal::from(minutes) / Decimal::from(60);
            let mut c = base(
                CharacteristicKind::Angle,
                vec![(format!("{n}{DEGREE}"), Slot::Main)],
                n,
                ToleranceRule::General,
                Some(sym(n, t)),
            );
            c.unit = Unit::Deg;
            c.review_note = Some(format!(
                "Shorter leg of the angle on the part: {leg} mm, ISO 2768-1 table 3 class {GENERAL_CLASS} gives ±{minutes}'. The leg is not written on the sheet, so box select cannot know it in M2 (generator table, draft, D-43)."
            ));
            c
        }
        Form::Chamfer => {
            let n = pick(rng, &[5, 10, 15, 20, 30, 40, 50]);
            let n = Decimal::new(n, 1).normalize();
            let text = if rng.range(0, 1) == 0 {
                format!("{}x45{DEGREE}", f.num(n))
            } else {
                format!("C{}", f.num(n))
            };
            let mut c = base(
                CharacteristicKind::Chamfer,
                vec![(text, Slot::Main)],
                n,
                ToleranceRule::General,
                Some(sym(n, radius_chamfer_m(n))),
            );
            c.review_note = Some(general_note());
            c
        }
        _ => unreachable!("form {form:?} is generated elsewhere"),
    }
}

/// Written tolerances: `±`, deviations, limits.
fn toleranced(draw: &Draw, rng: &mut Rng) -> Callout {
    let (form, f) = (draw.form, draw.f);
    let (prefix, length_kind) = (draw.prefix.as_str(), draw.length_kind);
    let base = |kind, texts, nominal, rule, limits| draw.base(kind, texts, nominal, rule, limits);
    match form {
        Form::Symmetric => {
            let n = size(rng, 5, 250, 2);
            let t = Decimal::new(pick(rng, &[5, 10, 20, 50]), 2);
            base(
                length_kind,
                vec![(
                    format!("{prefix}{}{PLUS_MINUS}{}", f.num(n), f.num(t)),
                    Slot::Main,
                )],
                n,
                ToleranceRule::Explicit,
                Some(sym(n, t)),
            )
        }
        Form::Asymmetric => {
            let n = size(rng, 5, 250, 2);
            let a = Decimal::new(pick(rng, &[5, 10, 20, 30]), 2);
            let b = Decimal::new(pick(rng, &[2, 5, 10, 15]), 2);
            let b = if a == b { b / Decimal::TWO } else { b };
            // Mostly plus over minus; one in four both on one side of the nominal.
            let (upper, lower) = match rng.range(0, 3) {
                0 => (a.max(b), a.min(b)),
                1 => (-a.min(b), -a.max(b)),
                _ => (a, -b),
            };
            base(
                length_kind,
                vec![
                    (format!("{prefix}{}", f.num(n)), Slot::Main),
                    (f.dev(upper, "0"), Slot::Upper),
                    (f.dev(lower, "0"), Slot::Lower),
                ],
                n,
                ToleranceRule::Explicit,
                Some((n + upper, n + lower)),
            )
        }
        Form::OneSided => {
            let n = size(rng, 5, 250, 2);
            let t = Decimal::new(pick(rng, &[5, 10, 20, 50, 100]), 2);
            let (upper, lower, up_text, low_text) = match rng.range(0, 2) {
                0 => (t, Decimal::ZERO, f.dev(t, "0"), "0".to_owned()),
                1 => (Decimal::ZERO, -t, "+0".to_owned(), f.dev(-t, "0")),
                _ => (Decimal::ZERO, -t, "0".to_owned(), f.dev(-t, "0")),
            };
            base(
                length_kind,
                vec![
                    (format!("{prefix}{}", f.num(n)), Slot::Main),
                    (up_text, Slot::Upper),
                    (low_text, Slot::Lower),
                ],
                n,
                ToleranceRule::Explicit,
                Some((n + upper, n + lower)),
            )
        }
        Form::Limit => {
            let n = size(rng, 5, 120, 1);
            let a = Decimal::new(pick(rng, &[2, 5, 10, 20]), 2);
            let b = Decimal::new(pick(rng, &[2, 5, 10, 20]), 2);
            let (upper, lower) = (n + a, n - b);
            // The first written limit is the nominal (decision log 2026-10-10, T2.5).
            base(
                length_kind,
                vec![
                    (format!("{prefix}{}", f.num(upper)), Slot::LimitUpper),
                    (f.num(lower), Slot::LimitLower),
                ],
                upper,
                ToleranceRule::Explicit,
                Some((upper, lower)),
            )
        }
        _ => unreachable!("form {form:?} is generated elsewhere"),
    }
}

/// Fits, threads and reference dimensions.
fn fits_and_others(draw: &Draw, rng: &mut Rng) -> Callout {
    let (form, f) = (draw.form, draw.f);
    let base = |kind, texts, nominal, rule, limits| draw.base(kind, texts, nominal, rule, limits);
    match form {
        Form::Fit | Form::FitPrinted => {
            let (nominal, fit, es, ei) = pick(rng, FITS);
            let n = Decimal::from(nominal);
            let (upper, lower) = (Decimal::new(es, 3), Decimal::new(ei, 3));
            let mut texts = vec![(format!("{OSLASH}{nominal} {fit}"), Slot::Main)];
            let rule = if form == Form::FitPrinted {
                texts.push((f.dev(upper, "0"), Slot::Upper));
                texts.push((f.dev(lower, "0"), Slot::Lower));
                ToleranceRule::Explicit
            } else {
                ToleranceRule::Fit
            };
            let mut c = base(
                CharacteristicKind::Diameter,
                texts,
                n,
                rule,
                Some((n + upper, n + lower)),
            );
            c.fit = Some(fit.to_owned());
            c.review_note = Some(format!(
                "Fit limits from the generator table FITS (ISO 286-2, hand written, draft, D-43). Owner: check {fit} at {nominal} mm."
            ));
            c
        }
        Form::Thread => {
            let (d, coarse, fine) = pick(rng, THREADS);
            let style = rng.range(0, 3);
            let pitch = if fine != 0 && style == 3 {
                fine
            } else {
                coarse
            };
            let pitch = Decimal::new(pitch, 2).normalize();
            let class = if rng.range(0, 1) == 0 { "6H" } else { "6g" };
            let text = match style {
                0 => format!("M{d}"),
                1 => format!("M{d}x{}", f.num(pitch)),
                _ => format!("M{d}x{}-{class}", f.num(pitch)),
            };
            base(
                CharacteristicKind::Thread,
                vec![(text, Slot::Main)],
                Decimal::from(d),
                ToleranceRule::NoToleranceDefined,
                None,
            )
        }
        Form::Reference => {
            let n = size(rng, 5, 250, 1);
            let mut c = base(
                CharacteristicKind::Linear,
                vec![(format!("({})", f.num(n)), Slot::Main)],
                n,
                ToleranceRule::NoToleranceDefined,
                None,
            );
            c.inspect = false;
            c
        }
        _ => unreachable!("form {form:?} is generated elsewhere"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        text.parse().unwrap_or_default()
    }

    #[test]
    fn general_tables_by_size() {
        assert_eq!(linear_m(d("3")), d("0.1"));
        assert_eq!(linear_m(d("6")), d("0.1"));
        assert_eq!(linear_m(d("6.01")), d("0.2"));
        assert_eq!(linear_m(d("120")), d("0.3"));
        assert_eq!(linear_m(d("250")), d("0.5"));
        assert_eq!(radius_chamfer_m(d("0.5")), d("0.2"));
        assert_eq!(radius_chamfer_m(d("3")), d("0.2"));
        assert_eq!(radius_chamfer_m(d("4")), d("0.5"));
        assert_eq!(radius_chamfer_m(d("30")), d("1"));
    }

    #[test]
    fn number_formats() {
        let f = Fmt { comma: true };
        assert_eq!(f.num(d("12.5")), "12,5");
        assert_eq!(f.dev(d("-0.05"), "0"), "\u{2212}0,05");
        assert_eq!(Fmt { comma: false }.dev(d("0.1"), "0"), "+0.1");
        assert_eq!(Fmt { comma: false }.dev(Decimal::ZERO, "+0"), "+0");
    }
}
