//! Characteristics whose limits or result type changed after the last issued report (M3
//! decision 16, T3.5): the hint marker of the measurement grid.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    CharId, Command, Document, FixedEnvironment, ReportFormat, ReportKind, ResultType, Sha256Hex,
    TemplateRef, changed_since_issued_report,
};

/// Two characteristics with limits 9.9 to 10.1, one without limits.
fn doc() -> (Document, FixedEnvironment, Vec<CharId>) {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    let limits = || {
        vec![
            FieldValue::Nominal(parse_decimal("10").into()),
            FieldValue::UpperDev(parse_decimal("0.1").into()),
            FieldValue::LowerDev(parse_decimal("-0.1").into()),
        ]
    };
    for (i, values) in [limits(), limits(), vec![]].into_iter().enumerate() {
        let x = 100.0 + 50.0 * f64::from(u32::try_from(i).unwrap());
        doc.execute(common::add_at(doc.project(), x, 200.0, values), &mut env)
            .unwrap();
    }
    let ids = common::order(doc.project());
    (doc, env, ids)
}

fn issue(doc: &mut Document, env: &mut FixedEnvironment, hash: u8) {
    doc.execute(
        Command::RecordIssuedReport {
            kind: ReportKind::InspectionReport,
            template: TemplateRef {
                id: "dimo-default".into(),
                version: 1,
            },
            format: ReportFormat::Pdf,
            output_sha256: Sha256Hex::from_bytes(&[hash; 32]),
        },
        env,
    )
    .unwrap();
}

fn set_upper_dev(doc: &mut Document, env: &mut FixedEnvironment, id: CharId, text: &str) {
    doc.execute(
        Command::UpdateFields {
            ids: vec![id],
            values: vec![FieldValue::UpperDev(parse_decimal(text).into())],
        },
        env,
    )
    .unwrap();
}

fn changed(doc: &Document) -> Vec<CharId> {
    changed_since_issued_report(doc.project(), doc.audit())
}

#[test]
fn nothing_is_marked_without_an_issued_report() {
    let (mut doc, mut env, ids) = doc();
    set_upper_dev(&mut doc, &mut env, ids[0], "0.2");
    assert_eq!(changed(&doc), Vec::<CharId>::new());
}

#[test]
fn limit_and_result_type_changes_after_the_issue_are_marked() {
    let (mut doc, mut env, ids) = doc();
    set_upper_dev(&mut doc, &mut env, ids[0], "0.05"); // before the issue: not marked
    issue(&mut doc, &mut env, 1);
    assert_eq!(changed(&doc), Vec::<CharId>::new());
    set_upper_dev(&mut doc, &mut env, ids[1], "0.2");
    doc.execute(
        Command::SetResultType {
            ids: vec![ids[2]],
            result_type: ResultType::Variable,
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(changed(&doc), vec![ids[1], ids[2]]);
    // A comment is not a limit.
    doc.execute(
        Command::UpdateFields {
            ids: vec![ids[0]],
            values: vec![FieldValue::Comment("check".into())],
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(changed(&doc), vec![ids[1], ids[2]]);
}

#[test]
fn reverted_changes_and_pins_to_the_same_type_are_not_marked() {
    let (mut doc, mut env, ids) = doc();
    issue(&mut doc, &mut env, 1);
    set_upper_dev(&mut doc, &mut env, ids[0], "0.2");
    set_upper_dev(&mut doc, &mut env, ids[0], "0.1");
    // Undo of a change also reverts it.
    set_upper_dev(&mut doc, &mut env, ids[1], "0.3");
    doc.undo(&mut env).unwrap();
    // Pinning `auto` to the type it already had changes nothing for the report.
    doc.execute(
        Command::SetResultType {
            ids: vec![ids[0]],
            result_type: ResultType::Variable,
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(changed(&doc), Vec::<CharId>::new());
}

#[test]
fn written_form_of_a_limit_counts() {
    let (mut doc, mut env, ids) = doc();
    issue(&mut doc, &mut env, 1);
    set_upper_dev(&mut doc, &mut env, ids[0], "0.10");
    assert_eq!(changed(&doc), vec![ids[0]]);
}

#[test]
fn counts_from_the_last_issue_and_follows_undo_of_an_issue() {
    let (mut doc, mut env, ids) = doc();
    issue(&mut doc, &mut env, 1);
    set_upper_dev(&mut doc, &mut env, ids[0], "0.2");
    issue(&mut doc, &mut env, 2);
    assert!(
        changed(&doc).is_empty(),
        "the second report has the new limit"
    );
    doc.undo(&mut env).unwrap(); // the second issue is gone, the first one counts again
    assert_eq!(changed(&doc), vec![ids[0]]);
    doc.redo(&mut env).unwrap();
    assert_eq!(changed(&doc), Vec::<CharId>::new());
}

#[test]
fn characteristics_added_after_the_issue_are_not_marked() {
    let (mut doc, mut env, _) = doc();
    issue(&mut doc, &mut env, 1);
    doc.execute(
        common::add_at(
            doc.project(),
            300.0,
            300.0,
            vec![FieldValue::Nominal(parse_decimal("5").into())],
        ),
        &mut env,
    )
    .unwrap();
    assert_eq!(changed(&doc), Vec::<CharId>::new());
}
