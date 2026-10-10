//! The interpretation step of box select and typed callouts (spec 08 stage 7, FR-TOL-01).
//!
//! Recognition parses a callout and hands it to an [`Interpreter`], which turns it into limits
//! and a derivation. The tolerance engine of `dimo-tolerance` is the real interpreter; it is
//! plugged in by the app through this trait, so `dimo-detect` does not decide tolerance rules.
//! [`CalloutOnly`] is the fallback: it reads only what is written on the callout.

use dimo_core::characteristic::Unit;
use dimo_core::derivation::{DerivationHint, DerivationRule, ToleranceDerivation};
use dimo_core::proposal::EngineVersion;
use dimo_notation::{Callout, Kind, Tolerance};
use rust_decimal::Decimal;

/// Limits and derivation of one callout, the result of an [`Interpreter`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Interpretation {
    /// Unit of nominal, deviations and limits.
    pub unit: Option<Unit>,
    /// Nominal value (converted if the interpreter converts units).
    pub nominal: Option<Decimal>,
    /// Upper deviation.
    pub upper_dev: Option<Decimal>,
    /// Lower deviation.
    pub lower_dev: Option<Decimal>,
    /// Upper limit.
    pub upper_limit: Option<Decimal>,
    /// Lower limit.
    pub lower_limit: Option<Decimal>,
    /// Which rule produced the limits (FR-TOL-08). `None` when the interpreter applied no rule.
    pub derivation: Option<ToleranceDerivation>,
}

/// Turns a parsed callout into limits (spec 08 stage 7).
///
/// Implementations carry their own context (project tolerance settings, tables); only the
/// drawing unit changes per call, because it is a property of the sheet (D-20).
pub trait Interpreter {
    /// Limits and derivation of `callout`. `basic` and `reference` on the callout are already
    /// final (a basic dimension frame found in the PDF sets `basic`). `drawing_unit` is the unit
    /// of the sheet, for callouts that write none.
    fn interpret(&self, callout: &Callout, drawing_unit: Unit) -> Interpretation;

    /// Name and version of the interpreter, stored on proposals.
    fn engine(&self) -> EngineVersion;
}

/// Fallback interpreter: the callout alone, without tables or project settings.
///
/// Fills the unit (written, else degrees for angles, inch for inch notation, else the drawing
/// unit), the nominal, and deviations and limits only when they are written on the callout
/// (`±`, two deviations, a limit pair, `MIN`, `MAX`); then the rule is `explicit`. A fit
/// without written deviations, or no tolerance at all, gives no limits and no derivation: that
/// needs the tolerance engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CalloutOnly;

/// Engine name of [`CalloutOnly`].
pub const CALLOUT_ONLY_ENGINE: &str = "dimo-detect/callout-only";

impl Interpreter for CalloutOnly {
    fn interpret(&self, callout: &Callout, drawing_unit: Unit) -> Interpretation {
        let unit = callout.unit.or(Some(match callout.kind {
            Kind::Angle => Unit::Deg,
            _ if callout.is_inch() => Unit::In,
            _ => drawing_unit,
        }));
        let nominal = callout.nominal.value;
        let mut out = Interpretation {
            unit,
            nominal: Some(nominal),
            ..Interpretation::default()
        };
        let explicit = match &callout.tolerance {
            None => false,
            Some(Tolerance::Symmetric(t)) => {
                let t = t.value.abs();
                set_deviations(&mut out, nominal, t, -t);
                true
            }
            Some(Tolerance::Deviations { upper, lower }) => {
                let (hi, lo) = ordered(upper.value, lower.value);
                set_deviations(&mut out, nominal, hi, lo);
                true
            }
            Some(Tolerance::Limits { other }) => {
                let (hi, lo) = ordered(nominal, other.value);
                out.upper_limit = Some(hi);
                out.lower_limit = Some(lo);
                true
            }
            Some(Tolerance::Min) => {
                out.lower_limit = Some(nominal);
                true
            }
            Some(Tolerance::Max) => {
                out.upper_limit = Some(nominal);
                true
            }
        };
        if explicit {
            let mut derivation = ToleranceDerivation::new(DerivationRule::Explicit);
            if callout.reference {
                derivation.hints.push(DerivationHint::ReferenceDimension);
            }
            if callout.basic {
                derivation.hints.push(DerivationHint::BasicDimension);
            }
            out.derivation = Some(derivation);
        }
        out
    }

    fn engine(&self) -> EngineVersion {
        EngineVersion::new(CALLOUT_ONLY_ENGINE, env!("CARGO_PKG_VERSION"))
    }
}

/// The larger value first. Zeros lose their sign, so `-0` reads as `0`.
fn ordered(a: Decimal, b: Decimal) -> (Decimal, Decimal) {
    let (a, b) = (unsigned_zero(a), unsigned_zero(b));
    if a >= b { (a, b) } else { (b, a) }
}

fn unsigned_zero(mut v: Decimal) -> Decimal {
    if v.is_zero() {
        v.set_sign_positive(true);
    }
    v
}

fn set_deviations(out: &mut Interpretation, nominal: Decimal, upper: Decimal, lower: Decimal) {
    let (upper, lower) = (unsigned_zero(upper), unsigned_zero(lower));
    out.upper_dev = Some(upper);
    out.lower_dev = Some(lower);
    out.upper_limit = nominal.checked_add(upper);
    out.lower_limit = nominal.checked_add(lower);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dimo_notation::parse_callout;

    fn d(text: &str) -> Option<Decimal> {
        text.parse().ok()
    }

    fn run(text: &str) -> Interpretation {
        let callout = parse_callout(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        CalloutOnly.interpret(&callout, Unit::Mm)
    }

    fn limits(i: &Interpretation) -> (Option<Decimal>, Option<Decimal>) {
        (i.upper_limit, i.lower_limit)
    }

    fn rule(i: &Interpretation) -> Option<&DerivationRule> {
        i.derivation.as_ref().map(|d| &d.rule)
    }

    // FR-REC-02, ADR 0006: explicit tolerances give the printed limits with rule explicit.
    #[test]
    fn written_tolerances_give_limits() {
        let cases = [
            ("Ø30 H7 +0.0203 -0", "30.0203", "30"),
            ("100 +0 −0.6", "100", "99.4"),
            ("12.39±0.1", "12.49", "12.29"),
            ("Ø8 f7 -0.0127 -0.0279", "7.9873", "7.9721"),
            ("90.0° +0.0° −0.1°", "90.0", "89.9"),
            ("12.02/11.98", "12.02", "11.98"),
            ("11.98/12.02", "12.02", "11.98"),
        ];
        for (text, upper, lower) in cases {
            let i = run(text);
            assert_eq!(limits(&i), (d(upper), d(lower)), "{text}");
            assert_eq!(rule(&i), Some(&DerivationRule::Explicit), "{text}");
        }
    }

    #[test]
    fn deviations_are_ordered_and_unsigned_at_zero() {
        let i = run("Ø30 H7 +0.0203 -0");
        assert_eq!((i.upper_dev, i.lower_dev), (d("0.0203"), d("0")));
        assert!(!i.lower_dev.unwrap_or_default().is_sign_negative());
        let i = run("25 -0.1 +0.2");
        assert_eq!((i.upper_dev, i.lower_dev), (d("0.2"), d("-0.1")));
    }

    #[test]
    fn min_and_max_set_one_limit() {
        assert_eq!(limits(&run("10 MIN")), (None, d("10")));
        assert_eq!(limits(&run("10 MAX")), (d("10"), None));
    }

    #[test]
    fn no_written_tolerance_gives_no_rule() {
        for text in ["50", "Ø8 c10", "M8x1.25-6H", "(42)"] {
            let i = run(text);
            assert_eq!(limits(&i), (None, None), "{text}");
            assert_eq!(i.derivation, None, "{text}");
        }
    }

    #[test]
    fn units_from_callout_kind_and_drawing() {
        assert_eq!(run("90.0°").unit, Some(Unit::Deg));
        assert_eq!(run("Ø.250").unit, Some(Unit::In));
        assert_eq!(run("50").unit, Some(Unit::Mm));
        let callout = parse_callout("50").unwrap();
        assert_eq!(
            CalloutOnly.interpret(&callout, Unit::In).unit,
            Some(Unit::In)
        );
    }

    #[test]
    fn reference_and_basic_are_hinted() {
        let mut callout = parse_callout("25±0.1").unwrap();
        callout.basic = true;
        let i = CalloutOnly.interpret(&callout, Unit::Mm);
        let hints = i.derivation.map(|d| d.hints).unwrap_or_default();
        assert_eq!(hints, [DerivationHint::BasicDimension]);
    }
}
