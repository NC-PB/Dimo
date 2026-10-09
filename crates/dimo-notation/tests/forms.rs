//! Table driven tests for every callout form listed in T2.1 (spec 08 stage 6, FR-REC-08,
//! FR-CHR-08, D-20, D-25).
//!
//! Each row gives the text and its canonical print. The canonical print shows kind prefix,
//! quantity, nominal (with its written decimal places), fit, tolerance, suffixes, markers and
//! unit, so equal prints mean equal parse results for these parts. Field level checks follow
//! for what the print does not show directly.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use dimo_core::characteristic::{CharacteristicKind, Unit};
use dimo_notation::{Callout, Kind, NumberForm, Suffix, Tolerance, parse_callout};
use rust_decimal::Decimal;

fn parse(text: &str) -> Callout {
    match parse_callout(text) {
        Ok(callout) => callout,
        Err(error) => panic!("{text:?} does not parse: {error}"),
    }
}

fn dec(text: &str) -> Decimal {
    Decimal::from_str_exact(text).unwrap()
}

/// Input text and its canonical print, for every form listed in T2.1.
const FORMS: &[(&str, &str)] = &[
    // Kind prefixes
    ("12.5", "12.5"),
    ("Ø12", "Ø12"),
    ("⌀12", "Ø12"),
    ("ø12", "Ø12"),
    ("DIA 12", "Ø12"),
    ("DIA12", "Ø12"),
    ("dia 12", "Ø12"),
    ("R5", "R5"),
    ("R 5", "R5"),
    ("SR10", "SR10"),
    ("SØ10", "SØ10"),
    ("S⌀10", "SØ10"),
    ("↧10", "↧10"),
    // Quantities (D-22)
    ("4X Ø6.6", "4X Ø6.6"),
    ("4x Ø6.6", "4X Ø6.6"),
    ("4 × Ø6.6", "4X Ø6.6"),
    ("4×Ø6.6", "4X Ø6.6"),
    ("4XØ6.6", "4X Ø6.6"),
    ("2 PL R5", "2X R5"),
    ("R5 2 PL", "2X R5"),
    ("R5 2PL", "2X R5"),
    ("R5 3 PLACES", "3X R5"),
    ("4X 1x45°", "4X 1x45°"),
    // Suffixes
    ("4X Ø6.6 THRU", "4X Ø6.6 THRU"),
    ("Ø6.6 thru", "Ø6.6 THRU"),
    ("Ø6 ↧10", "Ø6 ↧10"),
    ("Ø6 ↧ 10", "Ø6 ↧10"),
    ("Ø6 H7 ↧10 THRU", "Ø6 H7 ↧10 THRU"),
    // Symmetric tolerance
    ("50±0.1", "50 ±0.1"),
    ("50 ± 0.1", "50 ±0.1"),
    ("50 +/-0.1", "50 ±0.1"),
    ("50 +-0.1", "50 ±0.1"),
    // Deviations, stacked lines joined by the caller
    ("25 +0.1 -0.05", "25 +0.1 -0.05"),
    ("25 +0.1/-0.05", "25 +0.1 -0.05"),
    ("25 +0.1 / -0.05", "25 +0.1 -0.05"),
    ("25+0.1-0.05", "25 +0.1 -0.05"),
    ("25 +0.1 \u{2212}0.05", "25 +0.1 -0.05"),
    // One sided
    ("25 +0.1 0", "25 +0.1 +0"),
    ("100 +0 -0.6", "100 +0 -0.6"),
    ("100 0 -0.6", "100 +0 -0.6"),
    ("100 +0 \u{2212}0.6", "100 +0 -0.6"),
    // Limit dimensions
    ("12.02/11.98", "12.02 / 11.98"),
    ("12.02 / 11.98", "12.02 / 11.98"),
    ("12.02-11.98", "12.02 / 11.98"),
    ("12.02 11.98", "12.02 / 11.98"),
    ("11.98-12.02", "11.98 / 12.02"),
    // MIN and MAX
    ("12 MIN", "12 MIN"),
    ("12 max", "12 MAX"),
    ("R0.5 MAX", "R0.5 MAX"),
    // Fits
    ("Ø30 H7", "Ø30 H7"),
    ("Ø30H7", "Ø30 H7"),
    ("Ø8 g6", "Ø8 g6"),
    ("Ø8 c10", "Ø8 c10"),
    ("Ø8 js6", "Ø8 js6"),
    ("Ø8 JS7", "Ø8 JS7"),
    ("Ø30 H7/g6", "Ø30 H7/g6"),
    ("R15 H7", "R15 H7"),
    ("20 h9", "20 h9"),
    // Fit with printed deviations
    ("Ø8 f7 -0.0127 -0.0279", "Ø8 f7 -0.0127 -0.0279"),
    ("Ø8 f7-0.0127-0.0279", "Ø8 f7 -0.0127 -0.0279"),
    ("Ø30 H7 +0.0203 -0", "Ø30 H7 +0.0203 -0"),
    ("Ø30 H7 +0.021 0", "Ø30 H7 +0.021 +0"),
    // Angles
    ("90.0°", "90.0°"),
    ("30°", "30°"),
    ("30°15'", "30°15'"),
    ("30° 15'", "30°15'"),
    ("30°15'20\"", "30°15'20\""),
    ("30°15\u{2032}20\u{2033}", "30°15'20\""),
    ("90.0° +0.0° \u{2212}0.1°", "90.0° +0.0° -0.1°"),
    ("30° ±0.5°", "30° ±0.5°"),
    ("30° ±0.5", "30° ±0.5°"),
    ("30° ±0°30'", "30° ±0°30'"),
    ("30° ±30'", "30° ±0°30'"),
    ("30° 2 PL", "30° 2 PL"),
    // Chamfers
    ("1x45°", "1x45°"),
    ("1 x 45°", "1x45°"),
    ("1X45°", "1x45°"),
    ("1×45°", "1x45°"),
    ("0.5x30°", "0.5x30°"),
    ("C1", "C1"),
    ("C 0.5", "C0.5"),
    // Threads
    ("M8", "M8"),
    ("M8x1.25", "M8x1.25"),
    ("M8 x 1.25", "M8x1.25"),
    ("M8×1.25", "M8x1.25"),
    ("M8x1.25-6H", "M8x1.25-6H"),
    ("M8-6g", "M8-6g"),
    ("M10x1-5H6H", "M10x1-5H6H"),
    ("4X M6 THRU", "4X M6 THRU"),
    // Reference and basic (D-25)
    ("(42)", "(42)"),
    ("( 42 )", "(42)"),
    ("(Ø42)", "(Ø42)"),
    ("(4X Ø6)", "4X (Ø6)"),
    ("42 REF", "(42)"),
    ("[25]", "[25]"),
    ("[30°]", "[30°]"),
    // Number formats
    ("12,5", "12.5"),
    ("Ø0,8", "Ø0.8"),
    ("0.5", "0.5"),
    (".5", ".5"),
    (",5", ".5"),
    ("50 ±0,1", "50 ±0.1"),
    // Inch (D-20)
    (".250", ".250"),
    (".250\"", ".250\""),
    ("1/4", "1/4"),
    ("1 1/4", "1 1/4"),
    ("1/4\"", "1/4\""),
    ("2.5\"", "2.5\""),
    ("2.5 in", "2.5\""),
    ("2.5in", "2.5\""),
    ("Ø.500 ±.005", "Ø.500 ±.005"),
    ("1.250\" ±.005\"", "1.250\" ±.005"),
    ("12/11", "12 / 11"),
    ("12 mm", "12mm"),
    // Whitespace around the callout
    ("  Ø12  ", "Ø12"),
    ("Ø30 H7\n+0.0203\n-0", "Ø30 H7 +0.0203 -0"),
];

#[test]
fn every_form_parses_to_its_canonical_print() {
    for (text, canonical) in FORMS {
        let callout = parse(text);
        assert_eq!(&callout.to_canonical(), canonical, "input {text:?}");
        // The canonical print parses back to the same callout.
        assert_eq!(parse(canonical), callout, "canonical {canonical:?}");
    }
}

#[test]
fn kinds_map_to_the_domain_model() {
    let table: &[(&str, CharacteristicKind)] = &[
        ("12", CharacteristicKind::Linear),
        ("Ø12", CharacteristicKind::Diameter),
        ("SØ12", CharacteristicKind::Diameter),
        ("R12", CharacteristicKind::Radius),
        ("SR12", CharacteristicKind::SphericalRadius),
        ("30°", CharacteristicKind::Angle),
        ("30°15'", CharacteristicKind::Angle),
        ("1x45°", CharacteristicKind::Chamfer),
        ("C1", CharacteristicKind::Chamfer),
        ("M8x1.25-6H", CharacteristicKind::Thread),
        ("↧10", CharacteristicKind::Depth),
        ("(42)", CharacteristicKind::Linear),
    ];
    for (text, kind) in table {
        assert_eq!(parse(text).characteristic_kind(), *kind, "input {text:?}");
    }
}

#[test]
fn numbers_are_exact_decimals_with_their_written_places() {
    let callout = parse("90.0°");
    assert_eq!(callout.nominal.value, dec("90.0"));
    assert_eq!(callout.nominal.value.scale(), 1);
    assert_eq!(callout.unit, Some(Unit::Deg));

    let callout = parse("Ø30 H7 +0.0203 -0");
    let Some(Tolerance::Deviations { upper, lower }) = callout.tolerance else {
        panic!("deviations expected");
    };
    assert_eq!(upper.value, dec("0.0203"));
    assert!(lower.value.is_zero());

    // U+2212 and the hyphen give the same value.
    let a = parse("100 +0 \u{2212}0.6");
    let b = parse("100 +0 -0.6");
    assert_eq!(a, b);
    let Some(Tolerance::Deviations { lower, .. }) = a.tolerance else {
        panic!("deviations expected");
    };
    assert_eq!(lower.value, dec("-0.6"));

    // Decimal comma.
    assert_eq!(parse("12,5").nominal.value, dec("12.5"));

    // Angle in degrees and minutes, value in degrees.
    let callout = parse("30°15'");
    assert_eq!(callout.nominal.value, dec("30.25"));
    assert_eq!(
        callout.nominal.form,
        NumberForm::DegMinSec {
            degrees: 30,
            minutes: 15,
            seconds: None
        }
    );
}

#[test]
fn inch_values_are_marked_d20() {
    for text in [
        ".250",
        "1/4",
        "1 1/4",
        "2.5\"",
        "2.5 in",
        "Ø.500 ±.005",
        "1.250 ±.005",
        "Ø6 ↧.5",
    ] {
        assert!(parse(text).is_inch(), "{text:?} should be inch");
    }
    for text in ["0.250", "12.5", "Ø30 H7 +0.0203 -0", "12 mm", "90.0°"] {
        assert!(!parse(text).is_inch(), "{text:?} should not be inch");
    }
    assert_eq!(parse("1 1/4").nominal.value, dec("1.25"));
    assert_eq!(parse("1/64").nominal.value, dec("0.015625"));
    assert_eq!(parse(".250").nominal.value, dec("0.250"));
    assert_eq!(parse("2.5\"").unit, Some(Unit::In));
    assert_eq!(parse("2.5in").unit, Some(Unit::In));
    assert_eq!(parse("12 mm").unit, Some(Unit::Mm));
    // No marker: the unit comes from the drawing, never from the sheet size.
    assert_eq!(parse("12").unit, None);
}

#[test]
fn reference_and_basic_markers_d25() {
    let callout = parse("(42)");
    assert!(callout.reference);
    assert!(!callout.basic);
    assert_eq!(callout.nominal.value, dec("42"));
    assert_eq!(callout.tolerance, None);

    assert!(parse("42 REF").reference);
    let callout = parse("[25]");
    assert!(callout.basic);
    assert!(!callout.reference);

    let callout = parse("[25] REF");
    assert!(callout.basic && callout.reference);
    assert_eq!(callout.to_canonical(), "[25] REF");
}

#[test]
fn quantity_suffix_and_parts() {
    let callout = parse("4X Ø6.6 THRU");
    assert_eq!(callout.quantity, Some(4));
    assert_eq!(callout.kind, Kind::Diameter);
    assert_eq!(callout.suffixes, vec![Suffix::Thru]);

    let callout = parse("M8x1.25-6H");
    assert_eq!(callout.nominal.value, dec("8"));
    let Kind::Thread { pitch, class } = callout.kind else {
        panic!("thread expected");
    };
    assert_eq!(pitch.map(|p| p.value), Some(dec("1.25")));
    assert_eq!(class.as_deref(), Some("6H"));

    let callout = parse("1x45°");
    assert_eq!(callout.nominal.value, dec("1"));
    let Kind::Chamfer { angle: Some(angle) } = callout.kind else {
        panic!("chamfer with angle expected");
    };
    assert_eq!(angle.value, dec("45"));
    assert_eq!(parse("C1").kind, Kind::Chamfer { angle: None });

    let callout = parse("Ø30 H7/g6");
    let fit = callout.fit.unwrap();
    assert!(fit.first.is_hole());
    assert_eq!(fit.first.deviation, "H");
    assert_eq!(fit.first.grade, 7);
    let second = fit.second.unwrap();
    assert!(!second.is_hole());
    assert_eq!(second.to_string(), "g6");

    let callout = parse("12.02-11.98");
    assert_eq!(callout.nominal.value, dec("12.02"));
    assert_eq!(
        callout.tolerance,
        Some(Tolerance::Limits {
            other: dimo_notation::Number::decimal(dec("11.98"))
        })
    );
}

#[test]
fn free_text_and_other_symbols_are_not_callouts() {
    for text in [
        "",
        "   ",
        "BREAK ALL SHARP EDGES AND REMOVE BURRS",
        "Ra 1.6",
        "Ø",
        "50±",
        "25 -0.05",
        "4X",
        "Ø8 H7 abc",
        "30°75'",
        "30.5°15'",
        "(42",
        "[42)",
        "4X R5 2 PL",
        "0X R5",
        "M8x",
        "M8-",
        "SX5",
        "99999999999999999999999999999999",
    ] {
        assert!(parse_callout(text).is_err(), "{text:?} should not parse");
    }
}
