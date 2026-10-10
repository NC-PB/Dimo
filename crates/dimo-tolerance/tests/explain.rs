//! Golden tests of the explanation builder: one explanation per rule, English and German
//! (T2.5, FR-TOL-08). Review changes with `cargo insta review`.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::context::{Cfg, context};
use common::d;
use dimo_core::characteristic::Unit;
use dimo_core::derivation::ToleranceDerivation;
use dimo_notation::parse_callout;
use dimo_tolerance::{ExplainValues, Language, Measures, explain, interpret_with};

/// Interpret `text` and render the explanation in both languages.
fn both(text: &str, cfg: Cfg, unit: Unit, output: Option<Unit>, leg: Option<&str>) -> String {
    let ctx = context(cfg, unit, output);
    let measures = Measures {
        shorter_leg: leg.map(d),
    };
    let i = interpret_with(&parse_callout(text).unwrap(), &ctx, &measures);
    let values = ExplainValues::from(&i);
    format!(
        "callout: {text}\nen: {}\nde: {}\n",
        explain(&i.derivation, &values, Language::English),
        explain(&i.derivation, &values, Language::German)
    )
}

#[test]
fn explicit_with_fit_hint() {
    insta::assert_snapshot!(both("Ø30 H7 +0.0203 -0", Cfg::None, Unit::Mm, None, None));
}

#[test]
fn explicit_symmetric() {
    insta::assert_snapshot!(both("12.39±0.1", Cfg::None, Unit::Mm, None, None));
}

#[test]
fn explicit_min() {
    insta::assert_snapshot!(both("10 MIN", Cfg::None, Unit::Mm, None, None));
}

#[test]
fn fit() {
    insta::assert_snapshot!(both("Ø8 f7", Cfg::None, Unit::Mm, None, None));
}

#[test]
fn fit_pair() {
    insta::assert_snapshot!(both("Ø8 H7/g6", Cfg::General, Unit::Mm, None, None));
}

#[test]
fn drawing_rule() {
    insta::assert_snapshot!(both("45", Cfg::DrawingRule, Unit::Mm, None, None));
}

#[test]
fn general() {
    insta::assert_snapshot!(both("150", Cfg::DrawingRule, Unit::Mm, None, None));
}

#[test]
fn general_angle() {
    insta::assert_snapshot!(both("90°", Cfg::General, Unit::Mm, None, Some("100")));
}

#[test]
fn custom_table() {
    insta::assert_snapshot!(both("45", Cfg::CustomGeneral, Unit::Mm, None, None));
}

#[test]
fn decimal_rule() {
    insta::assert_snapshot!(both("12.50", Cfg::Decimal, Unit::Mm, None, None));
}

#[test]
fn no_tolerance_defined() {
    insta::assert_snapshot!(both("50", Cfg::None, Unit::Mm, None, None));
}

#[test]
fn reference() {
    insta::assert_snapshot!(both("(42)", Cfg::General, Unit::Mm, None, None));
}

#[test]
fn basic() {
    insta::assert_snapshot!(both("[25]", Cfg::General, Unit::Mm, None, None));
}

#[test]
fn inch_with_mm_table() {
    insta::assert_snapshot!(both(".250", Cfg::General, Unit::Mm, None, None));
}

#[test]
fn converted_to_mm() {
    insta::assert_snapshot!(both(
        "1.000\" ±.005",
        Cfg::None,
        Unit::Mm,
        Some(Unit::Mm),
        None
    ));
}

#[test]
fn manual() {
    let values = ExplainValues {
        nominal: Some(d("10")),
        unit: Some(Unit::Mm),
        upper_dev: Some(d("0.1")),
        lower_dev: Some(d("-0.2")),
        upper_limit: Some(d("10.1")),
        lower_limit: Some(d("9.8")),
    };
    let derivation = ToleranceDerivation::manual();
    insta::assert_snapshot!(format!(
        "en: {}\nde: {}\n",
        explain(&derivation, &values, Language::English),
        explain(&derivation, &values, Language::German)
    ));
}
