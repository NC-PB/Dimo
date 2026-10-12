//! Angular general tolerances by the shorter leg (T2.9a, FR-TOL-02, ISO 2768-1 table 3): range
//! bounds of every row and class, inch legs converted exactly, and legs that cannot apply.
//!
//! The expected values are the rows of the draft table `data/tolerances/iso-2768-1.toml`
//! (D-43): up to 10 mm, over 10 to 50, over 50 to 120, over 120 to 400, over 400; lower bound
//! exclusive, upper bound inclusive. Minutes of arc become degrees rounded toward zero to six
//! places.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::d;
use dimo_core::characteristic::{Length, Unit};
use dimo_core::derivation::{DerivationRule, TableRef};
use dimo_core::project::{TableClass, ToleranceSettings};
use dimo_notation::parse_callout;
use dimo_tolerance::{Interpretation, Measures, Note, TableSet, ToleranceContext, interpret_with};

fn context(class: &str, drawing_unit: Unit) -> ToleranceContext {
    let settings = ToleranceSettings {
        general: Some(TableClass {
            table: TableRef {
                id: "iso-2768-1".into(),
                version: 1,
            },
            class: class.into(),
        }),
        ..ToleranceSettings::default()
    };
    ToleranceContext::for_project(&settings, TableSet::shipped().unwrap(), [])
        .unwrap()
        .with_drawing_unit(drawing_unit)
}

fn leg(value: &str, unit: Unit) -> Length {
    Length {
        value: d(value),
        unit,
    }
}

fn run(text: &str, class: &str, drawing_unit: Unit, leg: Option<Length>) -> Interpretation {
    interpret_with(
        &parse_callout(text).unwrap(),
        &context(class, drawing_unit),
        &Measures::with_shorter_leg(leg),
    )
}

/// (class, leg, leg unit, expected ± in degrees). Each bound is checked on both sides.
#[rustfmt::skip]
const CASES: &[(&str, &str, Unit, &str)] = &[
    // Class m, every row and both sides of every bound, in mm.
    ("m", "0.5", Unit::Mm, "1"),
    ("m", "10", Unit::Mm, "1"),
    ("m", "10.001", Unit::Mm, "0.5"),
    ("m", "50", Unit::Mm, "0.5"),
    ("m", "50.001", Unit::Mm, "0.333333"),
    ("m", "120", Unit::Mm, "0.333333"),
    ("m", "120.001", Unit::Mm, "0.166666"),
    ("m", "400", Unit::Mm, "0.166666"),
    ("m", "400.001", Unit::Mm, "0.083333"),
    ("m", "5000", Unit::Mm, "0.083333"),
    // Classes f, c and v.
    ("f", "10", Unit::Mm, "1"),
    ("f", "40", Unit::Mm, "0.5"),
    ("f", "400.001", Unit::Mm, "0.083333"),
    ("c", "10", Unit::Mm, "1.5"),
    ("c", "10.001", Unit::Mm, "1"),
    ("c", "120", Unit::Mm, "0.5"),
    ("c", "400", Unit::Mm, "0.25"),
    ("c", "401", Unit::Mm, "0.166666"),
    ("v", "8", Unit::Mm, "3"),
    ("v", "50", Unit::Mm, "2"),
    ("v", "51", Unit::Mm, "1"),
    ("v", "121", Unit::Mm, "0.5"),
    ("v", "1000", Unit::Mm, "0.333333"),
    // Inch legs, converted exactly (25.4 mm per inch) next to the mm bounds.
    ("m", "0.3937", Unit::In, "1"),         // 9.99998 mm
    ("m", "0.3938", Unit::In, "0.5"),       // 10.00252 mm
    ("m", "1.9685", Unit::In, "0.5"),       // 49.9999 mm
    ("m", "1.9686", Unit::In, "0.333333"),  // 50.00244 mm
    ("m", "4.7244", Unit::In, "0.333333"),  // 119.99976 mm
    ("m", "4.7245", Unit::In, "0.166666"),  // 120.0023 mm
    ("m", "15.748", Unit::In, "0.166666"),  // 399.9992 mm
    ("m", "15.7481", Unit::In, "0.083333"), // 400.00174 mm
    ("v", "1.5", Unit::In, "2"),            // 38.1 mm
];

// T2.9a acceptance: the shorter leg gives the ISO 2768-1 angle limits.
#[test]
fn leg_selects_the_row_of_table_3() {
    for &(class, value, unit, tol) in CASES {
        let given = leg(value, unit);
        let i = run("30°", class, Unit::Mm, Some(given));
        let label = format!("class {class}, leg {value} {unit:?}");
        let t = d(tol);
        assert_eq!(i.unit, Some(Unit::Deg), "{label}");
        assert_eq!((i.upper_dev, i.lower_dev), (Some(t), Some(-t)), "{label}");
        assert_eq!(
            (i.upper_limit, i.lower_limit),
            (Some(d("30") + t), Some(d("30") - t)),
            "{label}"
        );
        assert!(
            matches!(i.derivation.rule, DerivationRule::General { .. }),
            "{label}"
        );
        assert!(i.notes.is_empty(), "{label}");
        assert_eq!(i.shorter_leg, Some(given), "{label}");
    }
}

// The sheet unit does not matter for the angle: the leg carries its own unit.
#[test]
fn leg_unit_is_independent_of_the_sheet_unit() {
    let on_inch_sheet = run("45°", "m", Unit::In, Some(leg("40", Unit::Mm)));
    let on_mm_sheet = run("45°", "m", Unit::Mm, Some(leg("40", Unit::Mm)));
    assert_eq!(on_inch_sheet, on_mm_sheet);
    assert_eq!(on_inch_sheet.upper_limit, Some(d("45.5")));
    assert_eq!(on_inch_sheet.derivation.conversion, None);
}

// Without a usable leg: no limits, the note asks for it.
#[test]
fn unusable_legs_give_the_note() {
    for given in [
        None,
        Some(leg("0", Unit::Mm)),
        Some(leg("-5", Unit::Mm)),
        Some(leg("40", Unit::Deg)),
    ] {
        let i = run("30°", "m", Unit::Mm, given);
        assert_eq!((i.upper_limit, i.lower_limit), (None, None), "{given:?}");
        assert_eq!(i.derivation.rule, DerivationRule::NoToleranceDefined);
        assert_eq!(i.notes, [Note::ShorterLegUnknown], "{given:?}");
        assert_eq!(i.shorter_leg, None, "{given:?}");
    }
}

// A leg does not change anything that is not an angular general tolerance.
#[test]
fn leg_is_ignored_where_it_does_not_apply() {
    let given = Some(leg("40", Unit::Mm));
    for text in ["50", "30° ±0.5°", "R5"] {
        let with = run(text, "m", Unit::Mm, given);
        let without = run(text, "m", Unit::Mm, None);
        assert_eq!(with, without, "{text}");
        assert_eq!(with.shorter_leg, None, "{text}");
    }
}
