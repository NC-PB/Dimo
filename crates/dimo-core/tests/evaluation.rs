//! Pass/fail evaluation (T3.3, FR-MEA-03, FR-MEA-06, M3 decisions 3, 4 and 5).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use std::str::FromStr;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::derivation::DerivationRule;
use dimo_core::{
    AttributeValue, ChangeSource, CharId, Characteristic, CharacteristicField,
    CharacteristicStatus, Command, CommandError, DisplayNumber, Document, FixedEnvironment,
    MeasurementResult, OverallVerdict, ResultEntry, ResultParseError, ResultSource, ResultType,
    ResultValue, SampleId, ToleranceDerivation, Unit, Verdict, VerdictCounts,
    characteristic_history, evaluate, measured_characteristics, measurement_state,
};
use proptest::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

fn dec(text: &str) -> Decimal {
    parse_decimal(text).unwrap()
}

fn opt(text: &str) -> Option<Decimal> {
    (!text.is_empty()).then(|| dec(text))
}

fn characteristic(lower: Option<Decimal>, upper: Option<Decimal>, unit: Unit) -> Characteristic {
    let mut c = Characteristic::manual(
        CharId::from_uuid(Uuid::from_u128(1)),
        DisplayNumber::from_str("1").unwrap(),
    );
    c.unit = Some(unit);
    c.lower_limit = lower;
    c.upper_limit = upper;
    c
}

fn result(value: ResultValue) -> MeasurementResult {
    MeasurementResult {
        sample: SampleId::from_uuid(Uuid::from_u128(2)),
        characteristic: CharId::from_uuid(Uuid::from_u128(1)),
        value,
        source: ResultSource::Manual,
        comment: String::new(),
    }
}

fn variable(text: &str) -> MeasurementResult {
    result(ResultValue::Variable { value: dec(text) })
}

fn attribute(value: AttributeValue) -> MeasurementResult {
    result(ResultValue::Attribute { value })
}

use Verdict::{Fail, Missing, NotEvaluated, Pass};

/// Decision 3, variable results: inclusive limits, exact comparison on the typed value, one
/// step of the typed scale outside fails, one sided limits, no limits, inch and degrees, mixed
/// scales. Columns: lower limit, upper limit (empty: none), unit, value, verdict.
#[test]
fn variable_results_against_limits() {
    let cases: &[(&str, &str, Unit, &str, Verdict)] = &[
        // On each limit, inside, and one step of the typed scale outside.
        ("9.95", "10.05", Unit::Mm, "9.95", Pass),
        ("9.95", "10.05", Unit::Mm, "10.05", Pass),
        ("9.95", "10.05", Unit::Mm, "10.00", Pass),
        ("9.95", "10.05", Unit::Mm, "9.94", Fail),
        ("9.95", "10.05", Unit::Mm, "10.06", Fail),
        ("9.95", "10.05", Unit::Mm, "9.949", Fail),
        ("9.95", "10.05", Unit::Mm, "10.051", Fail),
        ("9.95", "10.05", Unit::Mm, "10.0500001", Fail),
        ("9.95", "10.05", Unit::Mm, "9.9499999", Fail),
        ("9.95", "10.05", Unit::Mm, "10.1", Fail),
        ("9.95", "10.05", Unit::Mm, "10", Pass),
        // Never rounded: 10.054 would round to the limit at two places, it still fails.
        ("9.95", "10.05", Unit::Mm, "10.054", Fail),
        ("9.95", "10.05", Unit::Mm, "9.946", Fail),
        // Negative values and limits.
        ("-0.2", "-0.1", Unit::Mm, "-0.1", Pass),
        ("-0.2", "-0.1", Unit::Mm, "-0.2", Pass),
        ("-0.2", "-0.1", Unit::Mm, "-0.09", Fail),
        ("-0.2", "-0.1", Unit::Mm, "-0.21", Fail),
        ("-0.2", "-0.1", Unit::Mm, "0", Fail),
        // Zero width tolerance.
        ("5", "5", Unit::Mm, "5.000", Pass),
        ("5", "5", Unit::Mm, "5.001", Fail),
        // Mixed scales: `12` against `12.000` in both directions.
        ("11.9", "12", Unit::Mm, "12.000", Pass),
        ("11.9", "12", Unit::Mm, "12.0001", Fail),
        ("12.000", "12.1", Unit::Mm, "12", Pass),
        ("12.000", "12.1", Unit::Mm, "11.9999", Fail),
        ("12.000", "12.100", Unit::Mm, "12.1", Pass),
        // One sided: MAX (upper only) and MIN (lower only).
        ("", "0.8", Unit::Mm, "0.8", Pass),
        ("", "0.8", Unit::Mm, "-100", Pass),
        ("", "0.8", Unit::Mm, "0.81", Fail),
        ("", "0.8", Unit::Mm, "0.801", Fail),
        ("20", "", Unit::Mm, "20", Pass),
        ("20", "", Unit::Mm, "99999", Pass),
        ("20", "", Unit::Mm, "19.9", Fail),
        ("20", "", Unit::Mm, "19.999", Fail),
        // No limits: not evaluated, never pass or fail.
        ("", "", Unit::Mm, "12.3", NotEvaluated),
        ("", "", Unit::Mm, "0", NotEvaluated),
        ("", "", Unit::Mm, "-1", NotEvaluated),
        // Inch: compared in inch as entered, no conversion.
        ("0.4995", "0.5005", Unit::In, "0.5005", Pass),
        ("0.4995", "0.5005", Unit::In, "0.4995", Pass),
        ("0.4995", "0.5005", Unit::In, "0.5006", Fail),
        ("0.4995", "0.5005", Unit::In, "0.49949", Fail),
        ("0.4995", "0.5005", Unit::In, "12.7", Fail),
        // Decimal degrees.
        ("29.5", "30.5", Unit::Deg, "30.5", Pass),
        ("29.5", "30.5", Unit::Deg, "29.5", Pass),
        ("29.5", "30.5", Unit::Deg, "30.51", Fail),
        ("29.5", "30.5", Unit::Deg, "29.49", Fail),
        ("", "90.25", Unit::Deg, "90.250", Pass),
        ("", "90.25", Unit::Deg, "90.2501", Fail),
    ];
    for &(lower, upper, unit, value, want) in cases {
        let c = characteristic(opt(lower), opt(upper), unit);
        assert_eq!(
            evaluate(&c, Some(&variable(value))),
            want,
            "[{lower}, {upper}] {unit:?} value {value}"
        );
    }
}

/// Decision 3: attribute results ignore limits; no result is missing.
#[test]
fn attribute_and_missing_results() {
    let limits = [("", ""), ("9.95", "10.05"), ("", "0.8"), ("20", "")];
    for (lower, upper) in limits {
        let mut c = characteristic(opt(lower), opt(upper), Unit::Mm);
        for result_type in [ResultType::Auto, ResultType::Attribute] {
            c.result_type = result_type;
            assert_eq!(evaluate(&c, Some(&attribute(AttributeValue::Ok))), Pass);
            assert_eq!(evaluate(&c, Some(&attribute(AttributeValue::NotOk))), Fail);
            assert_eq!(evaluate(&c, None), Missing);
        }
    }
}

/// Decision 3: limits from draft tables (D-43) evaluate normally; the row carries the mark.
#[test]
fn draft_derivations_evaluate_normally() {
    for draft in [false, true] {
        let mut c = characteristic(opt("9.8"), opt("10.2"), Unit::Mm);
        c.derivation = Some(ToleranceDerivation {
            draft,
            ..ToleranceDerivation::new(DerivationRule::Explicit)
        });
        assert_eq!(evaluate(&c, Some(&variable("10.2"))), Pass);
        assert_eq!(evaluate(&c, Some(&variable("10.21"))), Fail);
        assert_eq!(evaluate(&c, Some(&variable("9.79"))), Fail);
    }
}

/// Decision 3: the summary of a group of cells.
#[test]
fn summaries_follow_decision_3() {
    let cases: &[(&[Verdict], OverallVerdict)] = &[
        (&[], OverallVerdict::Open),
        (&[Pass], OverallVerdict::Pass),
        (&[Pass, Pass, Pass], OverallVerdict::Pass),
        (&[Fail], OverallVerdict::Fail),
        (&[Pass, Fail], OverallVerdict::Fail),
        (&[Missing, Fail, NotEvaluated], OverallVerdict::Fail),
        (&[Pass, Missing], OverallVerdict::Open),
        (&[Pass, NotEvaluated], OverallVerdict::Open),
        (&[NotEvaluated], OverallVerdict::Open),
        (&[Missing], OverallVerdict::Open),
        (&[Missing, Missing], OverallVerdict::Open),
    ];
    for &(cells, want) in cases {
        let mut counts = VerdictCounts::default();
        for &v in cells {
            counts.add(v);
        }
        assert_eq!(counts.total() as usize, cells.len());
        assert_eq!(counts.verdict(), want, "{cells:?}");
    }
}

// --- Project level: measured set, summaries, pinning ---

fn limits(nominal: &str, dev: &str) -> Vec<FieldValue> {
    vec![
        FieldValue::Nominal(parse_decimal(nominal).into()),
        FieldValue::UpperDev(parse_decimal(dev).into()),
        FieldValue::LowerDev(parse_decimal(&format!("-{dev}")).into()),
    ]
}

/// Characteristics: 1 `10 ±0.1`, 2 without limits, 3 `20 ±0.05`; `samples` samples.
fn doc_with(samples: u32) -> (Document, FixedEnvironment) {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    for (i, values) in [limits("10", "0.1"), vec![], limits("20", "0.05")]
        .into_iter()
        .enumerate()
    {
        let x = 100.0 + 50.0 * f64::from(u32::try_from(i).unwrap());
        doc.execute(common::add_at(doc.project(), x, 200.0, values), &mut env)
            .unwrap();
    }
    if samples > 0 {
        doc.execute(Command::AddSamples { count: samples }, &mut env)
            .unwrap();
    }
    (doc, env)
}

fn ids(doc: &Document) -> (Vec<CharId>, Vec<SampleId>) {
    let p = doc.project();
    (
        p.characteristics.iter().map(|c| c.id).collect(),
        p.samples.iter().map(|s| s.id).collect(),
    )
}

fn set(doc: &mut Document, env: &mut FixedEnvironment, cells: &[(SampleId, CharId, &str)]) {
    doc.execute(
        Command::SetResults {
            results: cells
                .iter()
                .map(|&(sample, characteristic, text)| ResultEntry {
                    sample,
                    characteristic,
                    text: text.into(),
                    comment: None,
                })
                .collect(),
        },
        env,
    )
    .unwrap();
}

/// Decision 5: inspected, not rejected, in number order.
#[test]
fn measured_set_is_inspected_not_rejected_in_number_order() {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    for i in 0..5u32 {
        doc.execute(
            common::add_at(doc.project(), 100.0 + 40.0 * f64::from(i), 200.0, vec![]),
            &mut env,
        )
        .unwrap();
    }
    let c: Vec<CharId> = common::order(doc.project());
    doc.execute(
        Command::UpdateFields {
            ids: vec![c[1]],
            values: vec![FieldValue::Inspect(false)],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::UpdateFields {
            ids: vec![c[3]],
            values: vec![FieldValue::Status(CharacteristicStatus::Rejected)],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::UpdateFields {
            ids: vec![c[2]],
            values: vec![FieldValue::Status(CharacteristicStatus::Proposed)],
        },
        &mut env,
    )
    .unwrap();
    // Number order differs from placement order (possible while numbering is locked).
    let mut project = doc.project().clone();
    project.characteristics.swap(0, 4);
    let measured: Vec<CharId> = measured_characteristics(&project)
        .iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(measured, [c[0], c[2], c[4]]);
    let state = measurement_state(&project);
    let rows: Vec<CharId> = state.rows.iter().map(|r| r.characteristic).collect();
    assert_eq!(rows, [c[0], c[2], c[4]]);
}

fn remove_limits(doc: &mut Document, env: &mut FixedEnvironment, id: CharId) {
    let values = vec![
        FieldValue::UpperDev(None.into()),
        FieldValue::LowerDev(None.into()),
    ];
    doc.execute(
        Command::UpdateFields {
            ids: vec![id],
            values,
        },
        env,
    )
    .unwrap();
    let c = doc.project().characteristic(id).unwrap();
    assert_eq!((c.upper_limit, c.lower_limit), (None, None));
}

fn result_types(doc: &Document) -> Vec<ResultType> {
    let p = doc.project();
    p.characteristics.iter().map(|c| c.result_type).collect()
}

fn counts(pass: u32, fail: u32, not_evaluated: u32, missing: u32) -> VerdictCounts {
    VerdictCounts {
        pass,
        fail,
        not_evaluated,
        missing,
    }
}

use OverallVerdict as O;

/// Decision 3: verdicts per cell, summaries per sample, per characteristic and overall.
#[test]
fn measurement_state_summarizes_rows_samples_and_report() {
    let (mut doc, mut env) = doc_with(0);
    let state = measurement_state(doc.project());
    assert_eq!((state.rows.len(), state.samples.len()), (3, 0));
    assert_eq!(state.verdict, O::Open, "no samples is open");
    assert!(state.rows.iter().all(|r| r.verdict == O::Open));

    doc.execute(Command::AddSamples { count: 3 }, &mut env)
        .unwrap();
    let (c, s) = ids(&doc);
    let state = measurement_state(doc.project());
    assert_eq!((state.counts, state.verdict), (counts(0, 0, 0, 9), O::Open));

    // Sample 1 all pass, sample 2 one fail, sample 3 one missing.
    let cells = [
        (s[0], c[0], "10.1"),
        (s[0], c[1], "ok"),
        (s[0], c[2], "19.95"),
        (s[1], c[0], "9.9"),
        (s[1], c[1], "x"),
        (s[1], c[2], "20"),
        (s[2], c[0], "10"),
        (s[2], c[2], "20.05"),
    ];
    set(&mut doc, &mut env, &cells);
    let state = measurement_state(doc.project());
    let cells: Vec<&[Verdict]> = state.rows.iter().map(|r| r.cells.as_slice()).collect();
    let want: [&[Verdict]; 3] = [&[Pass; 3], &[Pass, Fail, Missing], &[Pass; 3]];
    assert_eq!(cells, want);
    let rows: Vec<O> = state.rows.iter().map(|r| r.verdict).collect();
    assert_eq!(rows, [O::Pass, O::Fail, O::Pass]);
    let samples: Vec<O> = state.samples.iter().map(|s| s.verdict).collect();
    assert_eq!(samples, [O::Pass, O::Fail, O::Open]);
    assert_eq!(state.samples[2].counts, counts(2, 0, 0, 1));
    assert_eq!(state.rows[1].counts, counts(1, 1, 0, 1));
    assert_eq!((state.counts, state.verdict), (counts(7, 1, 0, 1), O::Fail));
    let types: Vec<ResultType> = state.rows.iter().map(|r| r.result_type).collect();
    assert_eq!(types, result_types(&doc));

    // Fix the fail and fill the gap: everything passes.
    set(
        &mut doc,
        &mut env,
        &[(s[1], c[1], "o"), (s[2], c[1], "i.O.")],
    );
    let state = measurement_state(doc.project());
    assert_eq!(state.verdict, O::Pass);
    assert!(state.samples.iter().all(|s| s.verdict == O::Pass));
}

/// Decision 3: a variable result without limits is not evaluated and keeps the sample, the
/// row and the report open, never failing them.
#[test]
fn not_evaluated_keeps_summaries_open() {
    let (mut doc, mut env) = doc_with(2);
    let (c, s) = ids(&doc);
    let cells: Vec<(SampleId, CharId, &str)> = s
        .iter()
        .flat_map(|&s| [(s, c[0], "10"), (s, c[1], "ok"), (s, c[2], "20")])
        .collect();
    set(&mut doc, &mut env, &cells);
    assert_eq!(measurement_state(doc.project()).verdict, O::Pass);
    remove_limits(&mut doc, &mut env, c[2]);
    let state = measurement_state(doc.project());
    assert_eq!(state.rows[2].cells, [NotEvaluated; 2]);
    assert_eq!(state.rows[2].verdict, O::Open);
    assert_eq!((state.counts, state.verdict), (counts(4, 0, 2, 0), O::Open));
    assert!(state.samples.iter().all(|s| s.verdict == O::Open));
}

/// Decision 4 (owner addition): the first result pins an `auto` characteristic; undo restores
/// `auto`; removing the limits afterwards keeps the type variable.
#[test]
fn first_result_pins_auto_result_type() {
    let (mut doc, mut env) = doc_with(2);
    let (c, s) = ids(&doc);
    let auto = doc.project().clone();
    set(&mut doc, &mut env, &[(s[0], c[0], "10.02")]);
    let pinned = [ResultType::Variable, ResultType::Auto, ResultType::Auto];
    assert_eq!(result_types(&doc), pinned);

    // One undo step removes the result and restores `auto`.
    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), auto);
    doc.redo(&mut env).unwrap();
    assert_eq!(result_types(&doc), pinned);

    // The pin shows in the history as a rule change of the result type (FR-CHR-10).
    let history = characteristic_history(doc.audit(), c[0]);
    let last = history.last().unwrap();
    assert_eq!(last.source, ChangeSource::Rule);
    assert_eq!(last.fields, [CharacteristicField::ResultType]);

    // Limits removed: the type stays variable, numbers are still accepted, words are not.
    remove_limits(&mut doc, &mut env, c[0]);
    let ch = &doc.project().characteristics[0];
    assert_eq!(ch.result_type, ResultType::Variable);
    assert_eq!(ch.effective_result_type(), ResultType::Variable);
    set(&mut doc, &mut env, &[(s[1], c[0], "10.3")]);
    let ok = ResultEntry {
        sample: s[1],
        characteristic: c[0],
        text: "ok".into(),
        comment: None,
    };
    let refused = doc.execute(Command::SetResults { results: vec![ok] }, &mut env);
    let error = ResultParseError::NotANumber;
    assert_eq!(
        refused,
        Err(CommandError::InvalidResult {
            characteristic: c[0],
            error
        })
    );
    let state = measurement_state(doc.project());
    assert_eq!(state.rows[0].cells, [NotEvaluated, NotEvaluated]);

    // An attribute characteristic is pinned too: limits added later do not flip it.
    set(&mut doc, &mut env, &[(s[0], c[1], "nio")]);
    assert_eq!(result_types(&doc)[1], ResultType::Attribute);
    let values = limits("5", "0.1");
    doc.execute(
        Command::UpdateFields {
            ids: vec![c[1]],
            values,
        },
        &mut env,
    )
    .unwrap();
    let ch = &doc.project().characteristics[1];
    assert!(ch.upper_limit.is_some());
    assert_eq!(ch.effective_result_type(), ResultType::Attribute);
    let state = measurement_state(doc.project());
    assert_eq!(state.rows[1].cells, [Fail, Missing]);
}

/// Clearing a cell pins nothing; an explicit result type is never changed by a result.
#[test]
fn only_a_set_result_pins_and_only_auto() {
    let (mut doc, mut env) = doc_with(1);
    let (c, s) = ids(&doc);
    set(&mut doc, &mut env, &[(s[0], c[2], "")]);
    assert_eq!(result_types(&doc)[2], ResultType::Auto);
    let to_attribute = Command::SetResultType {
        ids: vec![c[2]],
        result_type: ResultType::Attribute,
    };
    doc.execute(to_attribute, &mut env).unwrap();
    let plus = ResultEntry {
        sample: s[0],
        characteristic: c[2],
        text: "+".into(),
        comment: None,
    };
    let patch = doc
        .execute(
            Command::SetResults {
                results: vec![plus],
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(patch.changes.len(), 1);
    assert_eq!(result_types(&doc)[2], ResultType::Attribute);
}
// --- Property tests (at least 10,000 cases each) ---

/// A decimal with up to 9 digits before and 0 to 6 after the point.
fn decimal() -> impl Strategy<Value = Decimal> {
    (-999_999_999_999_999i64..=999_999_999_999_999, 0u32..=6)
        .prop_map(|(mantissa, scale)| Decimal::new(mantissa, scale))
}

/// The smallest step of the scale `scale`, e.g. `0.001` for three places.
fn step(scale: u32) -> Decimal {
    Decimal::new(1, scale)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// A value between the limits (inclusive) always passes, never fails.
    #[test]
    fn between_limits_never_fails(a in decimal(), b in decimal(), v in decimal(), sides in 0u8..3) {
        let mut sorted = [a, b, v];
        sorted.sort();
        let [lower, value, upper] = sorted;
        // Same value, other scale (`12` against `12.000`).
        let rescaled = {
            let mut r = value;
            r.rescale(value.scale() + 2);
            r
        };
        let (lower, upper) = match sides {
            0 => (Some(lower), Some(upper)),
            1 => (None, Some(upper)),
            _ => (Some(lower), None),
        };
        let c = characteristic(lower, upper, Unit::Mm);
        prop_assert_eq!(evaluate(&c, Some(&variable_of(value))), Pass);
        prop_assert_eq!(evaluate(&c, Some(&variable_of(rescaled))), Pass);
        for limit in [lower, upper].into_iter().flatten() {
            prop_assert_eq!(evaluate(&c, Some(&variable_of(limit))), Pass);
        }
    }

    /// A value outside a limit, even by the smallest step of its typed scale, always fails,
    /// never passes.
    #[test]
    fn outside_limits_never_passes(
        a in decimal(), b in decimal(), scale in 0u32..=7, far in decimal(), sides in 0u8..3,
    ) {
        let (lower, upper) = if a <= b { (a, b) } else { (b, a) };
        let (lower, upper) = match sides {
            0 => (Some(lower), Some(upper)),
            1 => (None, Some(upper)),
            _ => (Some(lower), None),
        };
        let c = characteristic(lower, upper, Unit::Mm);
        let mut outside = Vec::new();
        if let Some(u) = upper {
            let s = scale.max(u.scale());
            outside.push(u + step(s));
            outside.push(u + far.abs() + step(s));
        }
        if let Some(l) = lower {
            let s = scale.max(l.scale());
            outside.push(l - step(s));
            outside.push(l - far.abs() - step(s));
        }
        for value in outside {
            prop_assert_eq!(evaluate(&c, Some(&variable_of(value))), Fail, "value {}", value);
        }
    }
}

fn variable_of(value: Decimal) -> MeasurementResult {
    result(ResultValue::Variable { value })
}

/// Decision 4: setting `auto` again on a characteristic with results keeps the pinned type, so
/// removing the limits cannot flip the row and mix types (spec review of T3.3).
#[test]
fn auto_again_with_results_keeps_the_pin() {
    let (mut doc, mut env) = doc_with(2);
    let (c, s) = ids(&doc);
    set(&mut doc, &mut env, &[(s[0], c[0], "10.02")]);
    let to_auto = Command::SetResultType {
        ids: vec![c[0], c[1]],
        result_type: ResultType::Auto,
    };
    doc.execute(to_auto, &mut env).unwrap();
    let pinned = [ResultType::Variable, ResultType::Auto, ResultType::Auto];
    assert_eq!(result_types(&doc), pinned);
    remove_limits(&mut doc, &mut env, c[0]);
    let ok = ResultEntry {
        sample: s[1],
        characteristic: c[0],
        text: "ok".into(),
        comment: None,
    };
    let refused = doc.execute(Command::SetResults { results: vec![ok] }, &mut env);
    let error = ResultParseError::NotANumber;
    assert_eq!(
        refused,
        Err(CommandError::InvalidResult {
            characteristic: c[0],
            error
        })
    );
    // Without results, `auto` is stored as asked.
    set(&mut doc, &mut env, &[(s[0], c[0], "")]);
    let to_auto = Command::SetResultType {
        ids: vec![c[0]],
        result_type: ResultType::Auto,
    };
    doc.execute(to_auto, &mut env).unwrap();
    assert_eq!(result_types(&doc)[0], ResultType::Auto);
}

/// Decision 3 and D-43: rows with limits from a draft table carry the draft mark.
#[test]
fn rows_carry_the_draft_mark() {
    let (mut doc, mut env) = doc_with(1);
    let (c, _) = ids(&doc);
    let draft = ToleranceDerivation {
        draft: true,
        ..ToleranceDerivation::new(DerivationRule::Explicit)
    };
    let values = vec![FieldValue::Derivation(Some(draft))];
    doc.execute(
        Command::UpdateFields {
            ids: vec![c[2]],
            values,
        },
        &mut env,
    )
    .unwrap();
    let state = measurement_state(doc.project());
    let marks: Vec<bool> = state.rows.iter().map(|r| r.draft).collect();
    assert_eq!(marks, [false, false, true]);
}
