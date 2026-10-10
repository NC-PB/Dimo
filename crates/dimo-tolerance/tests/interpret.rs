//! Tolerance engine: precedence levels and conflict cases (T2.5, FR-TOL-01, FR-TOL-02,
//! FR-TOL-04, FR-TOL-06, FR-TOL-07, FR-TOL-09, D-25, D-43).

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::context::{Cfg, SHOP_TABLE, context, settings, shop_entry};
use common::d;
use dimo_core::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use dimo_core::derivation::{
    DerivationHint, DerivationRule, RangeBound, SizeRange, TableRef, UnitConversion,
};
use dimo_core::project::{TableClass, ToleranceSettings};
use dimo_notation::parse_callout;
use dimo_tolerance::{
    ContextError, Interpretation, Measures, Note, TableError, TableSet, ToleranceContext,
    interpret_with, load_custom_table,
};

/// One expected interpretation. Decimals as text; `None` means no value.
struct Case {
    text: &'static str,
    cfg: Cfg,
    unit: Unit,
    output: Option<Unit>,
    leg: Option<&'static str>,
    rule: ToleranceRule,
    nominal: &'static str,
    upper: Option<&'static str>,
    lower: Option<&'static str>,
    upper_dev: Option<&'static str>,
    lower_dev: Option<&'static str>,
    draft: bool,
    hints: &'static [&'static str],
    notes: &'static [Note],
}

const BASE: Case = Case {
    text: "",
    cfg: Cfg::None,
    unit: Unit::Mm,
    output: None,
    leg: None,
    rule: ToleranceRule::NoToleranceDefined,
    nominal: "",
    upper: None,
    lower: None,
    upper_dev: None,
    lower_dev: None,
    draft: false,
    hints: &[],
    notes: &[],
};

fn run(case: &Case) -> Interpretation {
    let callout = parse_callout(case.text).unwrap_or_else(|e| panic!("{}: {e}", case.text));
    let ctx = context(case.cfg, case.unit, case.output);
    let measures = Measures {
        shorter_leg: case.leg.map(d),
    };
    interpret_with(&callout, &ctx, &measures)
}

fn hint_names(i: &Interpretation) -> Vec<String> {
    i.derivation
        .hints
        .iter()
        .map(|h| {
            serde_json::to_value(h).unwrap()["hint"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[rustfmt::skip]
fn cases() -> Vec<Case> {
    use ToleranceRule as R;
    vec![
        // Level 1, explicit: always wins, the fit table only gives a hint (spec 08 stage 7).
        Case { text: "Ø30 H7 +0.0203 -0", rule: R::Explicit, nominal: "30", upper: Some("30.0203"), lower: Some("30"), upper_dev: Some("0.0203"), lower_dev: Some("0"), hints: &["fit_deviations_differ"], ..BASE },
        Case { text: "Ø30 H7 +0.021 0", rule: R::Explicit, nominal: "30", upper: Some("30.021"), lower: Some("30"), upper_dev: Some("0.021"), lower_dev: Some("0"), ..BASE },
        Case { text: "Ø30 H7 +0.0203 -0", cfg: Cfg::DrawingRule, rule: R::Explicit, nominal: "30", upper: Some("30.0203"), lower: Some("30"), upper_dev: Some("0.0203"), lower_dev: Some("0"), hints: &["fit_deviations_differ"], ..BASE },
        Case { text: "Ø8 f7 -0.0127 -0.0279", rule: R::Explicit, nominal: "8", upper: Some("7.9873"), lower: Some("7.9721"), upper_dev: Some("-0.0127"), lower_dev: Some("-0.0279"), hints: &["fit_deviations_differ"], ..BASE },
        Case { text: "50±0.1", cfg: Cfg::GeneralDecimal, rule: R::Explicit, nominal: "50", upper: Some("50.1"), lower: Some("49.9"), upper_dev: Some("0.1"), lower_dev: Some("-0.1"), ..BASE },
        // Deviations in written order, the engine decides which is upper.
        Case { text: "10 -0.1 +0.2", rule: R::Explicit, nominal: "10", upper: Some("10.2"), lower: Some("9.9"), upper_dev: Some("0.2"), lower_dev: Some("-0.1"), ..BASE },
        Case { text: "100 +0 −0.6", rule: R::Explicit, nominal: "100", upper: Some("100"), lower: Some("99.4"), upper_dev: Some("0"), lower_dev: Some("-0.6"), ..BASE },
        // Limit dimensions: nominal is the first written limit.
        Case { text: "12.02/11.98", cfg: Cfg::General, rule: R::Explicit, nominal: "12.02", upper: Some("12.02"), lower: Some("11.98"), upper_dev: Some("0"), lower_dev: Some("-0.04"), ..BASE },
        Case { text: "11.98/12.02", rule: R::Explicit, nominal: "11.98", upper: Some("12.02"), lower: Some("11.98"), upper_dev: Some("0.04"), lower_dev: Some("0"), ..BASE },
        Case { text: "10 MIN", cfg: Cfg::General, rule: R::Explicit, nominal: "10", lower: Some("10"), lower_dev: Some("0"), ..BASE },
        Case { text: "10 MAX", rule: R::Explicit, nominal: "10", upper: Some("10"), upper_dev: Some("0"), ..BASE },
        Case { text: "90.0° +0.0° −0.1°", rule: R::Explicit, unit: Unit::Mm, nominal: "90.0", upper: Some("90.0"), lower: Some("89.9"), upper_dev: Some("0"), lower_dev: Some("-0.1"), ..BASE },
        // Level 2, fit: wins over drawing rule, general and decimal rules.
        Case { text: "Ø8 f7", rule: R::Fit, nominal: "8", upper: Some("7.987"), lower: Some("7.972"), upper_dev: Some("-0.013"), lower_dev: Some("-0.028"), draft: true, ..BASE },
        Case { text: "Ø8 f7", cfg: Cfg::DrawingRule, rule: R::Fit, nominal: "8", upper: Some("7.987"), lower: Some("7.972"), upper_dev: Some("-0.013"), lower_dev: Some("-0.028"), draft: true, ..BASE },
        Case { text: "R15 H7", cfg: Cfg::General, rule: R::Fit, nominal: "15", upper: Some("15.018"), lower: Some("15"), upper_dev: Some("0.018"), lower_dev: Some("0"), draft: true, ..BASE },
        // A fit that gives no limits stops the search: no general tolerance for it.
        Case { text: "Ø8 H7/g6", cfg: Cfg::General, nominal: "8", hints: &["unknown_fit"], notes: &[Note::FitPair], ..BASE },
        Case { text: "Ø600 H7", cfg: Cfg::General, nominal: "600", hints: &["size_outside_table"], ..BASE },
        // Level 3, drawing rule (custom table assigned in the project).
        Case { text: "45", cfg: Cfg::DrawingRule, rule: R::DrawingRule, nominal: "45", upper: Some("45.05"), lower: Some("44.95"), upper_dev: Some("0.05"), lower_dev: Some("-0.05"), draft: true, ..BASE },
        Case { text: "45.25", cfg: Cfg::DrawingRule, rule: R::DrawingRule, nominal: "45.25", upper: Some("45.30"), lower: Some("45.20"), upper_dev: Some("0.05"), lower_dev: Some("-0.05"), draft: true, ..BASE },
        // Drawing rule does not cover the size: hint, then the general tolerance.
        Case { text: "150", cfg: Cfg::DrawingRule, rule: R::General, nominal: "150", upper: Some("150.5"), lower: Some("149.5"), upper_dev: Some("0.5"), lower_dev: Some("-0.5"), draft: true, hints: &["size_outside_table"], ..BASE },
        // Drawing rule has no part for radii: general tolerance without a hint.
        Case { text: "R5", cfg: Cfg::DrawingRule, rule: R::General, nominal: "5", upper: Some("5.5"), lower: Some("4.5"), upper_dev: Some("0.5"), lower_dev: Some("-0.5"), draft: true, ..BASE },
        // Level 4, general tolerance by size range, linear, radius, chamfer and angle parts.
        Case { text: "45", cfg: Cfg::General, rule: R::General, nominal: "45", upper: Some("45.3"), lower: Some("44.7"), upper_dev: Some("0.3"), lower_dev: Some("-0.3"), draft: true, ..BASE },
        Case { text: "30", cfg: Cfg::General, rule: R::General, nominal: "30", upper: Some("30.2"), lower: Some("29.8"), upper_dev: Some("0.2"), lower_dev: Some("-0.2"), draft: true, ..BASE },
        Case { text: "12.50", cfg: Cfg::GeneralDecimal, rule: R::General, nominal: "12.50", upper: Some("12.70"), lower: Some("12.30"), upper_dev: Some("0.2"), lower_dev: Some("-0.2"), draft: true, ..BASE },
        Case { text: "4X Ø6.6 THRU", cfg: Cfg::General, rule: R::General, nominal: "6.6", upper: Some("6.8"), lower: Some("6.4"), upper_dev: Some("0.2"), lower_dev: Some("-0.2"), draft: true, ..BASE },
        Case { text: "R5", cfg: Cfg::General, rule: R::General, nominal: "5", upper: Some("5.5"), lower: Some("4.5"), upper_dev: Some("0.5"), lower_dev: Some("-0.5"), draft: true, ..BASE },
        Case { text: "1x45°", cfg: Cfg::General, rule: R::General, nominal: "1", upper: Some("1.2"), lower: Some("0.8"), upper_dev: Some("0.2"), lower_dev: Some("-0.2"), draft: true, ..BASE },
        Case { text: "45", cfg: Cfg::CustomGeneral, rule: R::CustomTable, nominal: "45", upper: Some("45.05"), lower: Some("44.95"), upper_dev: Some("0.05"), lower_dev: Some("-0.05"), draft: true, ..BASE },
        Case { text: "90°", cfg: Cfg::General, leg: Some("50"), rule: R::General, nominal: "90", upper: Some("90.5"), lower: Some("89.5"), upper_dev: Some("0.5"), lower_dev: Some("-0.5"), draft: true, ..BASE },
        // 20' is not exact in degrees: rounded toward zero to 6 places.
        Case { text: "90°", cfg: Cfg::General, leg: Some("100"), rule: R::General, nominal: "90", upper: Some("90.333333"), lower: Some("89.666667"), upper_dev: Some("0.333333"), lower_dev: Some("-0.333333"), draft: true, ..BASE },
        Case { text: "90°", cfg: Cfg::General, nominal: "90", notes: &[Note::ShorterLegUnknown], ..BASE },
        // Outside the general table: hint, then the decimal rule if one matches.
        Case { text: "C0.3", cfg: Cfg::General, nominal: "0.3", hints: &["size_outside_table"], ..BASE },
        Case { text: "5000", cfg: Cfg::GeneralDecimal, nominal: "5000", hints: &["size_outside_table"], ..BASE },
        Case { text: "5000.5", cfg: Cfg::GeneralDecimal, rule: R::DecimalRule, nominal: "5000.5", upper: Some("5000.6"), lower: Some("5000.4"), upper_dev: Some("0.1"), lower_dev: Some("-0.1"), hints: &["size_outside_table"], ..BASE },
        // Level 5, decimal place rule.
        Case { text: "12.5", cfg: Cfg::Decimal, rule: R::DecimalRule, nominal: "12.5", upper: Some("12.6"), lower: Some("12.4"), upper_dev: Some("0.1"), lower_dev: Some("-0.1"), ..BASE },
        Case { text: "12.50", cfg: Cfg::Decimal, rule: R::DecimalRule, nominal: "12.50", upper: Some("12.55"), lower: Some("12.45"), upper_dev: Some("0.05"), lower_dev: Some("-0.05"), ..BASE },
        Case { text: "12", cfg: Cfg::Decimal, nominal: "12", ..BASE },
        Case { text: "90.0°", cfg: Cfg::Decimal, nominal: "90.0", ..BASE },
        Case { text: "2.5", cfg: Cfg::Decimal, unit: Unit::In, rule: R::DecimalRule, nominal: "2.5", upper: Some("2.6"), lower: Some("2.4"), upper_dev: Some("0.1"), lower_dev: Some("-0.1"), ..BASE },
        // No tolerance and no general rule.
        Case { text: "50", nominal: "50", ..BASE },
        Case { text: "90.0°", nominal: "90.0", ..BASE },
        Case { text: "M8x1.25-6H", cfg: Cfg::General, nominal: "8", ..BASE },
        // Reference and basic dimensions: no limits even with a tolerance (D-25, FR-CHR-08).
        Case { text: "(42)", cfg: Cfg::General, nominal: "42", hints: &["reference_dimension"], ..BASE },
        Case { text: "(42±0.1)", nominal: "42", hints: &["reference_dimension"], ..BASE },
        Case { text: "[25]", cfg: Cfg::General, nominal: "25", hints: &["basic_dimension"], ..BASE },
        // Unit conversion (FR-TOL-09): mm table values on an inch dimension, rounded inward.
        Case { text: ".250", cfg: Cfg::General, rule: R::General, nominal: "0.250", upper: Some("0.2578"), lower: Some("0.2422"), upper_dev: Some("0.0078"), lower_dev: Some("-0.0078"), draft: true, ..BASE },
        Case { text: "1.000\" ±.005", output: Some(Unit::Mm), rule: R::Explicit, nominal: "25.400", upper: Some("25.527"), lower: Some("25.273"), upper_dev: Some("0.127"), lower_dev: Some("-0.127"), ..BASE },
        Case { text: "50 ±0.1", output: Some(Unit::In), rule: R::Explicit, nominal: "1.9685", upper: Some("1.9724"), lower: Some("1.9646"), upper_dev: Some("0.0039"), lower_dev: Some("-0.0039"), ..BASE },
        Case { text: "Ø8 f7", unit: Unit::In, output: Some(Unit::Mm), rule: R::Fit, nominal: "203.200", upper: Some("203.150"), lower: Some("203.104"), upper_dev: Some("-0.050"), lower_dev: Some("-0.096"), draft: true, ..BASE },
    ]
}

/// FR-TOL-01: every precedence level and conflict case, table driven.
#[test]
fn precedence_table() {
    let mut failures = Vec::new();
    for case in cases() {
        let i = run(&case);
        let got = (
            i.derivation.rule.kind(),
            i.nominal,
            i.upper_limit,
            i.lower_limit,
            i.upper_dev,
            i.lower_dev,
            i.derivation.draft,
            hint_names(&i),
            i.notes.clone(),
        );
        let want = (
            Some(case.rule),
            Some(d(case.nominal)),
            case.upper.map(d),
            case.lower.map(d),
            case.upper_dev.map(d),
            case.lower_dev.map(d),
            case.draft,
            case.hints
                .iter()
                .map(|h| (*h).to_owned())
                .collect::<Vec<_>>(),
            case.notes.to_vec(),
        );
        // Decimal equality ignores the scale; the nominal must also keep its written digits.
        let scale_ok = i.nominal.map(|n| n.to_string()) == Some(case.nominal.to_owned());
        if got != want || !scale_ok {
            failures.push(format!("{}: got {got:?}, want {want:?}", case.text));
        }
        let reference =
            case.hints.contains(&"reference_dimension") || case.hints.contains(&"basic_dimension");
        assert_eq!(i.inspect, !reference, "{}", case.text);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn derivation_details() {
    let ctx = context(Cfg::DrawingRule, Unit::Mm, None);
    let run = |text: &str| dimo_tolerance::interpret(&parse_callout(text).unwrap(), &ctx);

    // FR-TOL-04: fit with table, version and the step of the standard.
    let fit = run("Ø8 f7");
    let DerivationRule::Fit { table, fit, range } = fit.derivation.rule else {
        panic!("not a fit")
    };
    assert_eq!(table.id, "iso-286");
    assert_eq!(fit, "f7");
    assert_eq!(
        range,
        Some(SizeRange {
            min: Some(RangeBound {
                value: d("6"),
                inclusive: false
            }),
            max: Some(RangeBound {
                value: d("10"),
                inclusive: true
            }),
        })
    );

    // Hint keeps the table values; the written ones are used (spec 08 stage 7).
    let explicit = run("Ø30 H7 +0.0203 -0");
    assert_eq!(
        explicit.derivation.hints,
        vec![DerivationHint::FitDeviationsDiffer {
            fit: "H7".into(),
            table_upper_dev: Some(d("0.021")),
            table_lower_dev: Some(d("0")),
        }]
    );
    assert!(!explicit.derivation.draft, "explicit limits use no table");
    assert_eq!(explicit.fit.as_deref(), Some("H7"));
    assert_eq!(explicit.kind, CharacteristicKind::Diameter);

    let rule = run("45");
    let DerivationRule::DrawingRule { lookup } = &rule.derivation.rule else {
        panic!("not a drawing rule")
    };
    assert_eq!(lookup.table.id, "shop");
    assert_eq!(lookup.table.version, 2);
    assert_eq!(lookup.part, "linear");
    assert_eq!(lookup.class, "a");

    let outside = run("150");
    assert_eq!(
        outside.derivation.hints,
        vec![DerivationHint::SizeOutsideTable {
            table: TableRef {
                id: "shop".into(),
                version: 2
            }
        }]
    );

    let quantity = run("4X Ø6.6 THRU");
    assert_eq!(quantity.quantity, 4);
    assert_eq!(run("Ø6.6").quantity, 1);

    let decimal = dimo_tolerance::interpret(
        &parse_callout("12.50").unwrap(),
        &context(Cfg::Decimal, Unit::Mm, None),
    );
    assert_eq!(
        decimal.derivation.rule,
        DerivationRule::DecimalRule {
            places: 2,
            tolerance: d("0.05")
        }
    );
    assert_eq!(decimal.unit, Some(Unit::Mm));
}

/// FR-TOL-09: the derivation records the conversion and its rounding.
#[test]
fn unit_conversion_is_recorded() {
    let inch = dimo_tolerance::interpret(
        &parse_callout(".250").unwrap(),
        &context(Cfg::General, Unit::Mm, None),
    );
    assert_eq!(inch.unit, Some(Unit::In));
    assert_eq!(
        inch.derivation.conversion,
        Some(UnitConversion {
            from: Unit::Mm,
            to: Unit::In,
            places: 4
        })
    );
    let to_mm = dimo_tolerance::interpret(
        &parse_callout("1.000\" ±.005").unwrap(),
        &context(Cfg::None, Unit::Mm, Some(Unit::Mm)),
    );
    assert_eq!(to_mm.unit, Some(Unit::Mm));
    assert_eq!(
        to_mm.derivation.conversion,
        Some(UnitConversion {
            from: Unit::In,
            to: Unit::Mm,
            places: 3
        })
    );
    let none = dimo_tolerance::interpret(
        &parse_callout("50").unwrap(),
        &context(Cfg::None, Unit::In, Some(Unit::Mm)),
    );
    assert_eq!(none.nominal, Some(d("1270.000")));
    assert!(none.derivation.conversion.is_some());
    assert_eq!(none.upper_limit, None);

    // Angles are never converted.
    let angle = dimo_tolerance::interpret(
        &parse_callout("30°±1°").unwrap(),
        &context(Cfg::None, Unit::In, Some(Unit::Mm)),
    );
    assert_eq!(angle.unit, Some(Unit::Deg));
    assert_eq!(angle.derivation.conversion, None);
    assert_eq!(angle.upper_limit, Some(d("31")));
}

/// Loader check from T2.4: id and version inside a stored table must match the settings.
#[test]
fn stored_custom_table_must_match_settings() {
    let entry = shop_entry();
    assert!(load_custom_table(&entry, "shop.toml", SHOP_TABLE).is_ok());
    let mut other = entry.clone();
    other.table.version = 3;
    assert!(matches!(
        load_custom_table(&other, "shop.toml", SHOP_TABLE),
        Err(TableError::Mismatch { .. })
    ));
    other = entry.clone();
    other.table.id = "plant".into();
    let err = load_custom_table(&other, "shop.toml", SHOP_TABLE).unwrap_err();
    assert_eq!(
        err.to_string(),
        "shop.toml: the file holds table shop version 2, the project expects plant version 2"
    );

    // Through the context constructor.
    let mut s = settings(Cfg::DrawingRule);
    s.custom_tables[0].table.version = 1;
    s.drawing_rule.as_mut().unwrap().table.version = 1;
    let err =
        ToleranceContext::for_project(&s, TableSet::shipped().unwrap(), [("shop", SHOP_TABLE)])
            .unwrap_err();
    assert!(
        matches!(err, ContextError::Table(TableError::Mismatch { .. })),
        "{err:?}"
    );
    let err = ToleranceContext::for_project(
        &settings(Cfg::DrawingRule),
        TableSet::shipped().unwrap(),
        [],
    )
    .unwrap_err();
    assert_eq!(err, ContextError::MissingTable("shop".into()));
}

#[test]
fn context_refuses_settings_the_tables_cannot_serve() {
    let shipped = || TableSet::shipped().unwrap();
    let with = |edit: fn(&mut ToleranceSettings)| {
        let mut s = settings(Cfg::General);
        edit(&mut s);
        ToleranceContext::new(&s, shipped()).unwrap_err()
    };
    assert!(matches!(
        with(|s| s.general.as_mut().unwrap().table.version = 9),
        ContextError::Version {
            expected: 9,
            found: 1,
            ..
        }
    ));
    assert!(matches!(
        with(|s| s.general.as_mut().unwrap().table.id = "iso-286".into()),
        ContextError::WrongKind { .. }
    ));
    assert!(matches!(
        with(|s| s.general.as_mut().unwrap().class = "x".into()),
        ContextError::UnknownClass { .. }
    ));
    assert!(matches!(
        with(|s| s.general.as_mut().unwrap().table.id = "missing".into()),
        ContextError::MissingTable(_)
    ));
    // The drawing rule must be a custom table of the project (settings rule from T2.4).
    assert!(matches!(
        with(|s| {
            s.drawing_rule = Some(TableClass {
                table: s.general.clone().unwrap().table,
                class: "m".into(),
            });
        }),
        ContextError::Settings(_)
    ));
    let ctx = ToleranceContext::new(&ToleranceSettings::default(), shipped()).unwrap();
    assert_eq!(
        ctx.fit_table().map(dimo_tolerance::Table::id),
        Some("iso-286")
    );
    assert_eq!(ctx.drawing_unit(), Unit::Mm);
}

/// Without a fit table every fit is unknown and gives no limits.
#[test]
fn fit_without_fit_table() {
    let only_general = TableSet::new(
        dimo_tolerance::shipped_tables()
            .unwrap()
            .into_iter()
            .filter(|t| t.id() != "iso-286")
            .collect(),
    )
    .unwrap();
    let ctx = ToleranceContext::new(&settings(Cfg::General), only_general).unwrap();
    let i = dimo_tolerance::interpret(&parse_callout("Ø8 f7").unwrap(), &ctx);
    assert_eq!(i.derivation.rule, DerivationRule::NoToleranceDefined);
    assert_eq!(
        i.derivation.hints,
        vec![DerivationHint::UnknownFit { fit: "f7".into() }]
    );
}
