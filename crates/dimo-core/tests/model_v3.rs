//! Data model version 3 (T3.1): samples, results, result types, report info, revision labels
//! and issued reports (M3 decisions 1, 2, 4, 6, 7, 8, 16 and 17).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    AttributeValue, AuditAction, Change, CharId, Command, CommandError, Document, FixedEnvironment,
    LockReason, MeasurementResult, Project, ReportFormat, ReportInfo, ReportKind, ResultCell,
    ResultEntry, ResultParseError, ResultSource, ResultType, ResultValue, SampleId, SerialError,
    Sha256Hex, TemplateRef,
};
use rust_decimal::Decimal;

fn dec(text: &str) -> Decimal {
    parse_decimal(text).unwrap()
}

/// Characteristics: 1 with limits (auto is variable), 2 without (auto is attribute), 3 with
/// limits; and `samples` samples.
fn doc_with(samples: u32) -> (Document, FixedEnvironment) {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    let limits = || {
        vec![
            FieldValue::Nominal(parse_decimal("10").into()),
            FieldValue::UpperDev(parse_decimal("0.1").into()),
            FieldValue::LowerDev(parse_decimal("-0.1").into()),
        ]
    };
    for (i, values) in [limits(), vec![], limits()].into_iter().enumerate() {
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

fn chars(doc: &Document) -> Vec<CharId> {
    common::order(doc.project())
}

fn samples(doc: &Document) -> Vec<SampleId> {
    doc.project().samples.iter().map(|s| s.id).collect()
}

fn entry(sample: SampleId, characteristic: CharId, text: &str) -> ResultEntry {
    ResultEntry {
        sample,
        characteristic,
        text: text.into(),
        comment: None,
    }
}

/// Every cell of the grid as text: `v<value>`, `ok`, `not_ok` or `-`, rows are samples.
fn grid(project: &Project) -> Vec<Vec<String>> {
    project
        .samples
        .iter()
        .map(|s| {
            project
                .characteristics
                .iter()
                .map(|c| {
                    let cell = ResultCell {
                        sample: s.id,
                        characteristic: c.id,
                    };
                    match project.result(cell).map(|r| r.value) {
                        None => "-".into(),
                        Some(ResultValue::Variable { value }) => format!("v{value}"),
                        Some(ResultValue::Attribute {
                            value: AttributeValue::Ok,
                        }) => "ok".into(),
                        Some(ResultValue::Attribute { .. }) => "not_ok".into(),
                    }
                })
                .collect()
        })
        .collect()
}

#[test]
fn new_projects_have_no_samples_and_auto_result_types() {
    let (doc, _) = doc_with(0);
    let p = doc.project();
    assert!(p.samples.is_empty() && p.results.is_empty() && p.issued_reports.is_empty());
    assert_eq!(p.report_info, ReportInfo::default());
    let types: Vec<(ResultType, ResultType)> = p
        .characteristics
        .iter()
        .map(|c| (c.result_type, c.effective_result_type()))
        .collect();
    assert_eq!(
        types,
        [
            (ResultType::Auto, ResultType::Variable),
            (ResultType::Auto, ResultType::Attribute),
            (ResultType::Auto, ResultType::Variable),
        ]
    );
}

/// Decision 6: serials continue the highest plain number, the inspector is the audit user,
/// 1 to 100 at once, at most 500 per project.
#[test]
fn add_samples_numbers_serials_and_checks_counts() {
    let (mut doc, mut env) = doc_with(2);
    let serials: Vec<&str> = doc
        .project()
        .samples
        .iter()
        .map(|s| s.serial.as_str())
        .collect();
    assert_eq!(serials, ["1", "2"]);
    assert!(
        doc.project()
            .samples
            .iter()
            .all(|s| s.inspector == "inspector"
                && s.date.is_empty()
                && s.gauges.is_empty()
                && s.comment.is_empty())
    );

    let mut renamed = doc.project().samples[1].clone();
    renamed.serial = "  7 ".into();
    doc.execute(Command::UpdateSample { sample: renamed }, &mut env)
        .unwrap();
    assert_eq!(doc.project().samples[1].serial, "7");
    doc.execute(Command::AddSamples { count: 1 }, &mut env)
        .unwrap();
    assert_eq!(doc.project().samples[2].serial, "8");

    for count in [0, 101] {
        assert_eq!(
            doc.execute(Command::AddSamples { count }, &mut env),
            Err(CommandError::InvalidSampleCount)
        );
    }
    for _ in 0..4 {
        doc.execute(Command::AddSamples { count: 100 }, &mut env)
            .unwrap();
    }
    assert_eq!(doc.project().samples.len(), 403);
    assert_eq!(
        doc.execute(Command::AddSamples { count: 98 }, &mut env),
        Err(CommandError::TooManySamples)
    );
    doc.execute(Command::AddSamples { count: 97 }, &mut env)
        .unwrap();
    assert_eq!(doc.project().samples.len(), 500);
    assert_eq!(
        doc.execute(Command::AddSamples { count: 1 }, &mut env),
        Err(CommandError::TooManySamples)
    );
    common::check_invariants(doc.project());
}

#[test]
fn update_sample_checks_serial_and_date() {
    let (mut doc, mut env) = doc_with(2);
    let first = doc.project().samples[0].clone();
    let cases: [(&str, &str, Option<CommandError>); 6] = [
        (" ", "", Some(CommandError::Serial(SerialError::Empty))),
        (
            "2",
            "",
            Some(CommandError::Serial(SerialError::Duplicate("2".into()))),
        ),
        (
            "1",
            "2026-02-30",
            Some(CommandError::InvalidDate("2026-02-30".into())),
        ),
        (
            "1",
            "12.10.2026",
            Some(CommandError::InvalidDate("12.10.2026".into())),
        ),
        ("A-17", "2026-10-12", None),
        ("a-17", "", None),
    ];
    for (serial, date, error) in cases {
        let mut sample = first.clone();
        serial.clone_into(&mut sample.serial);
        date.clone_into(&mut sample.date);
        sample.gauges = "caliper 0-150".into();
        let before = doc.project().clone();
        let result = doc.execute(Command::UpdateSample { sample }, &mut env);
        if let Some(error) = error {
            assert_eq!(result, Err(error), "{serial:?} {date:?}");
            assert_eq!(*doc.project(), before);
        } else {
            result.unwrap();
            assert_eq!(doc.project().samples[0].serial, serial);
            assert_eq!(doc.project().samples[0].gauges, "caliper 0-150");
        }
    }
    let mut unknown = first;
    unknown.id = SampleId::from_uuid(uuid::Uuid::from_u128(u128::MAX));
    assert_eq!(
        doc.execute(
            Command::UpdateSample {
                sample: unknown.clone()
            },
            &mut env
        ),
        Err(CommandError::UnknownSample(unknown.id))
    );
}

/// Decisions 7 and 8: text parsed by the effective result type, one undo step for a paste,
/// empty text clears, refused as a whole on one bad cell.
#[test]
fn set_results_parses_by_type_as_one_undo_step() {
    let (mut doc, mut env) = doc_with(2);
    let (c, s) = (chars(&doc), samples(&doc));
    let before = doc.project().clone();
    let patch = doc
        .execute(
            Command::SetResults {
                results: vec![
                    entry(s[1], c[2], "10,05"),
                    entry(s[0], c[0], "9.950"),
                    entry(s[0], c[1], "o"),
                    entry(s[1], c[1], " x "),
                    entry(s[1], c[0], "\u{2212}.5"),
                ],
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(patch.changes.len(), 5);
    assert_eq!(
        grid(doc.project()),
        [["v9.950", "ok", "-"], ["v-0.5", "not_ok", "v10.05"]]
    );
    common::check_invariants(doc.project());
    // The typed scale is kept in the file (rule 5, decision 8).
    let json = serde_json::to_value(doc.project()).unwrap();
    assert_eq!(
        json["results"][0],
        serde_json::json!({
            "sample": s[0].to_string(), "characteristic": c[0].to_string(),
            "value": { "type": "variable", "value": "9.950" },
            "source": { "type": "manual" }, "comment": ""
        })
    );

    // A bad cell refuses the whole paste.
    let after = doc.project().clone();
    let bad = doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[2], "10"), entry(s[0], c[1], "12")],
        },
        &mut env,
    );
    assert_eq!(
        bad,
        Err(CommandError::InvalidResult {
            characteristic: c[1],
            error: ResultParseError::NotAnAttribute
        })
    );
    assert_eq!(
        doc.execute(
            Command::SetResults {
                results: vec![entry(s[0], c[0], "ok")],
            },
            &mut env
        ),
        Err(CommandError::InvalidResult {
            characteristic: c[0],
            error: ResultParseError::NotANumber
        })
    );
    assert_eq!(*doc.project(), after);

    // Same value, other digits is a change; same text is none; empty clears; comment kept.
    let with_comment = ResultEntry {
        comment: Some("burr".into()),
        ..entry(s[0], c[0], "9.95")
    };
    let patch = doc
        .execute(
            Command::SetResults {
                results: vec![with_comment],
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(patch.changes.len(), 1);
    let patch = doc
        .execute(
            Command::SetResults {
                results: vec![entry(s[0], c[0], "9.95")],
            },
            &mut env,
        )
        .unwrap();
    assert!(patch.is_empty());
    let cell = ResultCell {
        sample: s[0],
        characteristic: c[0],
    };
    assert_eq!(doc.project().result(cell).unwrap().comment, "burr");
    doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[0], "  ")],
        },
        &mut env,
    )
    .unwrap();
    assert!(doc.project().result(cell).is_none());

    while doc.can_undo() && *doc.project() != before {
        doc.undo(&mut env).unwrap();
    }
    assert_eq!(*doc.project(), before);
}

#[test]
fn clear_results_and_unknown_cells() {
    let (mut doc, mut env) = doc_with(1);
    let (c, s) = (chars(&doc), samples(&doc));
    doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[0], "10"), entry(s[0], c[1], "ok")],
        },
        &mut env,
    )
    .unwrap();
    let cells = |pairs: &[(SampleId, CharId)]| -> Vec<ResultCell> {
        pairs
            .iter()
            .map(|&(sample, characteristic)| ResultCell {
                sample,
                characteristic,
            })
            .collect()
    };
    let patch = doc
        .execute(
            Command::ClearResults {
                cells: cells(&[(s[0], c[0]), (s[0], c[2])]),
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(patch.changes.len(), 1, "a cell without result is no change");
    assert_eq!(grid(doc.project()), [["-", "ok", "-"]]);
    let unknown_sample = SampleId::from_uuid(uuid::Uuid::from_u128(u128::MAX));
    let unknown_char = CharId::from_uuid(uuid::Uuid::from_u128(u128::MAX));
    assert_eq!(
        doc.execute(
            Command::ClearResults {
                cells: cells(&[(unknown_sample, c[0])])
            },
            &mut env
        ),
        Err(CommandError::UnknownSample(unknown_sample))
    );
    assert_eq!(
        doc.execute(
            Command::SetResults {
                results: vec![entry(s[0], unknown_char, "1")]
            },
            &mut env
        ),
        Err(CommandError::UnknownCharacteristic(unknown_char))
    );
}

/// T3.1 acceptance: deleting a characteristic with results and undoing restores the results
/// unchanged (M3 decision 16).
#[test]
fn deleting_a_characteristic_with_results_and_undoing_restores_them() {
    let (mut doc, mut env) = doc_with(3);
    let (c, s) = (chars(&doc), samples(&doc));
    let mut results = Vec::new();
    for (i, sample) in s.iter().enumerate() {
        results.push(entry(*sample, c[0], &format!("10.0{i}")));
        results.push(entry(*sample, c[1], if i == 1 { "x" } else { "ok" }));
        results.push(ResultEntry {
            comment: Some(format!("note {i}")),
            ..entry(*sample, c[2], "9,90")
        });
    }
    doc.execute(Command::SetResults { results }, &mut env)
        .unwrap();
    let before = doc.project().clone();

    let patch = doc
        .execute(
            Command::DeleteCharacteristics {
                ids: vec![c[1], c[2]],
            },
            &mut env,
        )
        .unwrap();
    // Results first, then the balloon, then the characteristic, per characteristic.
    let kinds: Vec<&str> = patch
        .changes
        .iter()
        .map(|change| match change {
            Change::ResultChanged { .. } => "result",
            Change::BalloonRemoved { .. } => "balloon",
            Change::CharacteristicRemoved { .. } => "char",
            Change::CharacteristicChanged { .. } => "renumber",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "result", "result", "result", "balloon", "char", "result", "result", "result",
            "balloon", "char"
        ]
    );
    assert_eq!(doc.project().results.len(), 3);
    common::check_invariants(doc.project());

    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), before);
    // Byte identical, including the typed digits and the comments.
    assert_eq!(
        serde_json::to_string(doc.project()).unwrap(),
        serde_json::to_string(&before).unwrap()
    );
    doc.redo(&mut env).unwrap();
    assert_eq!(doc.project().results.len(), 3);
}

#[test]
fn deleting_samples_removes_their_results() {
    let (mut doc, mut env) = doc_with(3);
    let (c, s) = (chars(&doc), samples(&doc));
    let results = s.iter().map(|sample| entry(*sample, c[0], "10")).collect();
    doc.execute(Command::SetResults { results }, &mut env)
        .unwrap();
    let before = doc.project().clone();
    doc.execute(
        Command::DeleteSamples {
            ids: vec![s[0], s[2], s[0]],
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(samples(&doc), [s[1]]);
    assert_eq!(doc.project().results.len(), 1);
    common::check_invariants(doc.project());
    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), before);
}

/// Results stay sorted by sample order, then placement order, when either order changes
/// (decision 1, byte identical saves).
#[test]
fn results_follow_sample_and_placement_order() {
    let (mut doc, mut env) = doc_with(3);
    let (c, s) = (chars(&doc), samples(&doc));
    let mut results = Vec::new();
    for sample in s.iter().rev() {
        for characteristic in c.iter().rev() {
            let text = if *characteristic == c[1] { "ok" } else { "10" };
            results.push(entry(*sample, *characteristic, text));
        }
    }
    doc.execute(Command::SetResults { results }, &mut env)
        .unwrap();
    common::check_invariants(doc.project());
    let before = doc.project().clone();

    doc.execute(
        Command::MoveSamples {
            ids: vec![s[2]],
            before: Some(s[0]),
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(samples(&doc), [s[2], s[0], s[1]]);
    common::check_invariants(doc.project());
    doc.execute(
        Command::MoveCharacteristics {
            ids: vec![c[2]],
            before: Some(c[0]),
        },
        &mut env,
    )
    .unwrap();
    common::check_invariants(doc.project());
    assert_eq!(
        doc.execute(
            Command::MoveSamples {
                ids: vec![s[1]],
                before: Some(s[1]),
            },
            &mut env
        ),
        Err(CommandError::InvalidMoveTarget)
    );
    let patch = doc
        .execute(
            Command::MoveSamples {
                ids: vec![s[1]],
                before: None,
            },
            &mut env,
        )
        .unwrap();
    assert!(patch.is_empty(), "already last");
    doc.undo(&mut env).unwrap();
    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), before);
}

/// Decision 4: the result type cannot change while results of the other type exist.
#[test]
fn result_type_is_refused_while_results_of_the_other_type_exist() {
    let (mut doc, mut env) = doc_with(1);
    let (c, s) = (chars(&doc), samples(&doc));
    doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[0], "10.02"), entry(s[0], c[1], "ok")],
        },
        &mut env,
    )
    .unwrap();
    let set = |ids: Vec<CharId>, result_type| Command::SetResultType { ids, result_type };
    let cases = [
        (c[0], ResultType::Attribute, Some(c[0])),
        (c[0], ResultType::Variable, None),
        (c[0], ResultType::Auto, None),
        (c[1], ResultType::Variable, Some(c[1])),
        (c[1], ResultType::Attribute, None),
        (c[2], ResultType::Attribute, None),
    ];
    for (id, result_type, conflict) in cases {
        let before = doc.project().clone();
        let result = doc.execute(set(vec![id], result_type), &mut env);
        if let Some(id) = conflict {
            assert_eq!(result, Err(CommandError::ResultTypeConflict(id)));
            assert_eq!(*doc.project(), before);
        } else {
            result.unwrap();
            assert_eq!(
                doc.project().characteristic(id).unwrap().result_type,
                result_type
            );
        }
    }
    // Characteristic 3 is attribute now: numbers are refused, words accepted.
    assert!(
        doc.execute(
            Command::SetResults {
                results: vec![entry(s[0], c[2], "10")]
            },
            &mut env
        )
        .is_err()
    );
    doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[2], "n.i.O.")],
        },
        &mut env,
    )
    .unwrap();
    // A mixed batch is refused as a whole.
    let before = doc.project().clone();
    assert_eq!(
        doc.execute(set(vec![c[1], c[0]], ResultType::Attribute), &mut env),
        Err(CommandError::ResultTypeConflict(c[0]))
    );
    assert_eq!(*doc.project(), before);
    // After clearing, the type can change.
    doc.execute(
        Command::ClearResults {
            cells: vec![ResultCell {
                sample: s[0],
                characteristic: c[0],
            }],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(set(vec![c[0]], ResultType::Attribute), &mut env)
        .unwrap();
    let history = dimo_core::characteristic_history(doc.audit(), c[0]);
    let last = history.last().unwrap();
    assert_eq!(last.fields, [dimo_core::CharacteristicField::ResultType]);
    assert_eq!(last.source, dimo_core::ChangeSource::Manual);
}

#[test]
fn report_info_and_revision_label() {
    let (mut doc, mut env) = doc_with(0);
    let info = ReportInfo {
        report_number: "R-0042".into(),
        report_date: "2026-10-12".into(),
        company_name: "Shop".into(),
        inspected_by: "inspector".into(),
        approved_by: "boss".into(),
        remarks: "first lot".into(),
    };
    doc.execute(Command::UpdateReportInfo { info: info.clone() }, &mut env)
        .unwrap();
    assert_eq!(doc.project().report_info, info);
    let patch = doc
        .execute(Command::UpdateReportInfo { info: info.clone() }, &mut env)
        .unwrap();
    assert!(patch.is_empty());
    let bad = ReportInfo {
        report_date: "2026-13-01".into(),
        ..info
    };
    assert_eq!(
        doc.execute(Command::UpdateReportInfo { info: bad }, &mut env),
        Err(CommandError::InvalidDate("2026-13-01".into()))
    );

    let revision = doc.project().current_revision;
    let patch = doc
        .execute(
            Command::UpdateRevisionLabel {
                revision,
                label: " B ".into(),
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(
        patch.changes,
        [Change::RevisionLabelChanged {
            revision,
            before: "A".into(),
            after: "B".into(),
        }]
    );
    let unknown = dimo_core::RevisionId::from_uuid(uuid::Uuid::from_u128(u128::MAX));
    assert_eq!(
        doc.execute(
            Command::UpdateRevisionLabel {
                revision: unknown,
                label: "C".into()
            },
            &mut env
        ),
        Err(CommandError::UnknownRevision(unknown))
    );
    doc.undo(&mut env).unwrap();
    assert_eq!(doc.project().current_revision().unwrap().label, "A");
}

/// Decision 17: lock and record are one undo step; the record takes revision, drawing hash and
/// time from the project and the environment.
#[test]
fn issued_report_is_recorded_with_the_lock_as_one_step() {
    let (mut doc, mut env) = doc_with(1);
    env.set_time(dimo_core::Timestamp::parse("2026-10-12T09:30:00Z").unwrap());
    let before = doc.project().clone();
    doc.execute(
        Command::Batch {
            commands: vec![
                Command::LockNumbering {
                    reason: LockReason::IssuedReport,
                },
                Command::RecordIssuedReport {
                    kind: ReportKind::InspectionReport,
                    template: TemplateRef {
                        id: "dimo-default".into(),
                        version: 1,
                    },
                    format: ReportFormat::Pdf,
                    output_sha256: Sha256Hex::from_bytes(&[9; 32]),
                },
            ],
        },
        &mut env,
    )
    .unwrap();
    let p = doc.project();
    assert!(p.is_numbering_locked());
    let report = &p.issued_reports[0];
    assert_eq!(report.revision, p.current_revision);
    assert_eq!(report.drawing_sha256, Sha256Hex::from_bytes(&[7; 32]));
    assert_eq!(report.exported_at.as_str(), "2026-10-12T09:30:00Z");
    assert_eq!(report.output_sha256, Sha256Hex::from_bytes(&[9; 32]));
    assert!(matches!(
        doc.audit().last().unwrap().action,
        AuditAction::Command {
            command: Command::Batch { .. }
        }
    ));
    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), before);
}

/// Decision 2: the model stores only what the user entered; there is no verdict anywhere.
#[test]
fn no_verdict_is_stored() {
    let (mut doc, mut env) = doc_with(1);
    let (c, s) = (chars(&doc), samples(&doc));
    doc.execute(
        Command::SetResults {
            results: vec![entry(s[0], c[0], "12")],
        },
        &mut env,
    )
    .unwrap();
    let json = serde_json::to_string(doc.project()).unwrap();
    for word in ["verdict", "pass", "fail", "evaluation"] {
        assert!(!json.contains(word), "{word} in {json}");
    }
    let result = &doc.project().results[0];
    assert_eq!(
        *result,
        MeasurementResult {
            sample: s[0],
            characteristic: c[0],
            value: ResultValue::Variable { value: dec("12") },
            source: ResultSource::Manual,
            comment: String::new(),
        }
    );
}
