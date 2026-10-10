//! Patch content, undo, audit and validation of single commands (05 Architecture,
//! principle 1, NFR-REL-02, FR-BAL-12, FR-DOC-05).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    AuditAction, BalloonMove, BalloonShape, BalloonStyle, BalloonStyleOverride, Change,
    CharacteristicKind, Command, CommandError, Document, LockReason, OrientedBox, Point, Rotation,
    Scale, Size, TextSource, Unit,
};

fn dec(text: &str) -> dimo_core::decimal::OptionalDecimal {
    parse_decimal(text).into()
}

fn doc_with(count: usize) -> (Document, dimo_core::FixedEnvironment) {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    for i in 0..count {
        let x = 100.0 + 50.0 * f64::from(u32::try_from(i).unwrap());
        doc.execute(common::add_at(doc.project(), x, 200.0, vec![]), &mut env)
            .unwrap();
    }
    (doc, env)
}

#[test]
fn add_inserts_characteristic_then_balloon() {
    let (mut doc, mut env) = doc_with(0);
    let sheet = common::sheet(doc.project(), 0);
    let region = OrientedBox {
        center: Point { x: 80.0, y: 200.0 },
        size: Size {
            width: 30.0,
            height: 8.0,
        },
        angle: 0.0,
    };
    let patch = doc
        .execute(
            Command::AddCharacteristic {
                sheet,
                position: Point { x: 100.0, y: 180.0 },
                anchor: Point { x: 90.0, y: 200.0 },
                region: Some(region),
                values: vec![
                    FieldValue::Kind(CharacteristicKind::Diameter),
                    FieldValue::RequirementText("Ø8 ±0.1".into()),
                    FieldValue::Nominal(dec("8")),
                    FieldValue::UpperDev(dec("0.1")),
                    FieldValue::LowerDev(dec("-0.1")),
                ],
                insert_after: None,
            },
            &mut env,
        )
        .unwrap();
    let [
        Change::CharacteristicInserted {
            index: 0,
            characteristic: c,
        },
        Change::BalloonInserted { index: 0, balloon },
    ] = patch.changes.as_slice()
    else {
        panic!("unexpected patch {patch:#?}");
    };
    // Fixture used IDs 1 to 3, so the new IDs are 4 and 5 (rule 11: injected, sequential).
    assert_eq!(c.id.to_string(), "00000000-0000-0000-0000-000000000004");
    assert_eq!(
        balloon.id.to_string(),
        "00000000-0000-0000-0000-000000000005"
    );
    assert_eq!(c.number.to_string(), "1");
    assert_eq!(c.unit, Some(Unit::Mm), "unit taken from the sheet");
    assert_eq!(c.upper_limit.unwrap().to_string(), "8.1");
    assert_eq!(c.lower_limit.unwrap().to_string(), "7.9");
    assert_eq!(c.sources.len(), 1);
    assert_eq!(c.sources[0].text_source, TextSource::Manual);
    assert_eq!(c.sources[0].region, region);
    assert_eq!(balloon.characteristic, c.id);
    assert_eq!(balloon.anchor, Point { x: 90.0, y: 200.0 });
    assert!(balloon.style.is_empty());

    // Undo removes in reverse order.
    let undo = doc.undo(&mut env).unwrap();
    assert!(matches!(
        undo.changes.as_slice(),
        [
            Change::BalloonRemoved { index: 0, .. },
            Change::CharacteristicRemoved { index: 0, .. }
        ]
    ));
    assert_eq!(doc.project().characteristics, []);
}

#[test]
fn locked_add_records_the_new_highest_number_first() {
    let (mut doc, mut env) = doc_with(2);
    doc.execute(
        Command::LockNumbering {
            reason: LockReason::IssuedReport,
        },
        &mut env,
    )
    .unwrap();
    let patch = doc
        .execute(common::add_at(doc.project(), 10.0, 10.0, vec![]), &mut env)
        .unwrap();
    let [
        Change::NumberingChanged { before, after },
        Change::CharacteristicInserted { characteristic, .. },
        Change::BalloonInserted { .. },
    ] = patch.changes.as_slice()
    else {
        panic!("unexpected patch {patch:#?}");
    };
    assert_eq!(before.lock.as_ref().unwrap().highest_number, 2);
    assert_eq!(after.lock.as_ref().unwrap().highest_number, 3);
    assert_eq!(characteristic.number.to_string(), "3");
}

#[test]
fn delete_removes_balloons_then_characteristics_then_renumbers() {
    let (mut doc, mut env) = doc_with(3);
    let first = common::order(doc.project())[0];
    let patch = doc
        .execute(
            Command::DeleteCharacteristics { ids: vec![first] },
            &mut env,
        )
        .unwrap();
    let kinds: Vec<&str> = patch
        .changes
        .iter()
        .map(|c| match c {
            Change::BalloonRemoved { .. } => "balloon_removed",
            Change::CharacteristicRemoved { .. } => "characteristic_removed",
            Change::CharacteristicChanged { .. } => "characteristic_changed",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "balloon_removed",
            "characteristic_removed",
            "characteristic_changed",
            "characteristic_changed"
        ]
    );
    assert_eq!(
        common::numbers(doc.project()),
        [1, 2].map(dimo_core::DisplayNumber::plain)
    );
}

#[test]
fn update_fields_on_several_characteristics() {
    let (mut doc, mut env) = doc_with(3);
    let ids = common::order(doc.project());
    let patch = doc
        .execute(
            Command::UpdateFields {
                ids: vec![ids[0], ids[2], ids[0]],
                values: vec![
                    FieldValue::Classification(dimo_core::Classification::Critical),
                    FieldValue::Quantity(4),
                ],
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(patch.changes.len(), 2, "duplicates are applied once");
    let p = doc.project();
    assert_eq!(p.characteristics[0].quantity, 4);
    assert_eq!(p.characteristics[1].quantity, 1);
    assert_eq!(p.characteristics[2].quantity, 4);

    // Setting the same values again changes nothing: no patch, no undo step.
    let before_undo = doc.undo_command().cloned();
    let same = doc
        .execute(
            Command::UpdateFields {
                ids: vec![ids[0]],
                values: vec![FieldValue::Quantity(4)],
            },
            &mut env,
        )
        .unwrap();
    assert!(same.is_empty());
    assert_eq!(doc.undo_command().cloned(), before_undo);
}

#[test]
fn unit_defaults_to_degrees_for_angles_and_to_the_sheet_unit() {
    let (mut doc, mut env) = doc_with(2);
    let sheet = common::sheet(doc.project(), 0);
    doc.execute(
        Command::UpdateSheet {
            sheet,
            rotation: None,
            unit: Some(Unit::In),
            scale: None,
        },
        &mut env,
    )
    .unwrap();
    let ids = common::order(doc.project());
    doc.execute(
        Command::UpdateFields {
            ids: vec![ids[0]],
            values: vec![FieldValue::Nominal(dec("0.250"))],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::UpdateFields {
            ids: vec![ids[1]],
            values: vec![
                FieldValue::Kind(CharacteristicKind::Angle),
                FieldValue::Nominal(dec("90.0")),
            ],
        },
        &mut env,
    )
    .unwrap();
    let p = doc.project();
    assert_eq!(p.characteristics[0].unit, Some(Unit::In));
    assert_eq!(p.characteristics[0].nominal.unwrap().to_string(), "0.250");
    assert_eq!(p.characteristics[1].unit, Some(Unit::Deg));

    // A kind change corrects a unit that no longer fits.
    doc.execute(
        Command::UpdateFields {
            ids: vec![ids[1]],
            values: vec![FieldValue::Kind(CharacteristicKind::Linear)],
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(doc.project().characteristics[1].unit, Some(Unit::In));
}

#[test]
fn move_balloons_keeps_anchor_unless_given() {
    let (mut doc, mut env) = doc_with(2);
    let b0 = doc.project().balloons[0].clone();
    let b1 = doc.project().balloons[1].clone();
    doc.execute(
        Command::MoveBalloons {
            moves: vec![
                BalloonMove {
                    id: b0.id,
                    position: Point { x: 1.0, y: 2.0 },
                    anchor: None,
                },
                BalloonMove {
                    id: b1.id,
                    position: Point { x: 3.0, y: 4.0 },
                    anchor: Some(Point { x: 5.0, y: 6.0 }),
                },
            ],
        },
        &mut env,
    )
    .unwrap();
    let p = doc.project();
    assert_eq!(p.balloons[0].position, Point { x: 1.0, y: 2.0 });
    assert_eq!(p.balloons[0].anchor, b0.anchor);
    assert_eq!(p.balloons[1].anchor, Point { x: 5.0, y: 6.0 });
    // One undo step for the whole group move (FR-BAL-12).
    doc.undo(&mut env).unwrap();
    assert_eq!(doc.project().balloons, vec![b0, b1]);
}

#[test]
fn restyle_merges_and_reset_clears() {
    let (mut doc, mut env) = doc_with(2);
    let ids: Vec<_> = doc.project().balloons.iter().map(|b| b.id).collect();
    let flag = BalloonStyleOverride {
        shape: Some(BalloonShape::Flag),
        ..Default::default()
    };
    let no_leader = BalloonStyleOverride {
        leader: Some(false),
        ..Default::default()
    };
    for style in [flag, no_leader] {
        doc.execute(
            Command::RestyleBalloons {
                ids: ids.clone(),
                style,
            },
            &mut env,
        )
        .unwrap();
    }
    let p = doc.project();
    let effective = p.settings.balloon_style.with_override(&p.balloons[1].style);
    assert_eq!(effective.shape, BalloonShape::Flag);
    assert!(!effective.leader);

    doc.execute(Command::ResetBalloonStyle { ids: vec![ids[0]] }, &mut env)
        .unwrap();
    assert!(doc.project().balloons[0].style.is_empty());
    assert!(!doc.project().balloons[1].style.is_empty());

    let bad = BalloonStyleOverride {
        size_mm: Some(f64::NAN),
        ..Default::default()
    };
    assert_eq!(
        doc.execute(Command::RestyleBalloons { ids, style: bad }, &mut env),
        Err(CommandError::InvalidStyle)
    );
}

#[test]
fn sheet_settings_are_validated() {
    let (mut doc, mut env) = doc_with(0);
    let sheet = common::sheet(doc.project(), 1);
    let patch = doc
        .execute(
            Command::UpdateSheet {
                sheet,
                rotation: Some(Rotation::Deg90),
                unit: None,
                scale: Some(Scale {
                    drawing: 1,
                    actual: 2,
                }),
            },
            &mut env,
        )
        .unwrap();
    let [Change::SheetChanged { before, after }] = patch.changes.as_slice() else {
        panic!("unexpected patch {patch:#?}");
    };
    assert_eq!(before.rotation, Rotation::Deg0);
    assert_eq!(after.rotation, Rotation::Deg90);
    assert_eq!(after.scale.to_string(), "1:2");
    assert_eq!(after.unit, Unit::Mm);

    let invalid = [
        (Some(Unit::Deg), None),
        (
            None,
            Some(Scale {
                drawing: 0,
                actual: 1,
            }),
        ),
    ];
    for (unit, scale) in invalid {
        let result = doc.execute(
            Command::UpdateSheet {
                sheet,
                rotation: None,
                unit,
                scale,
            },
            &mut env,
        );
        assert!(matches!(result, Err(CommandError::InvalidSheetSetting(_))));
    }
}

#[test]
fn refused_commands_change_nothing() {
    let (mut doc, mut env) = doc_with(2);
    let before = doc.project().clone();
    let audit_len = doc.audit().len();
    let unknown = dimo_core::CharId::from_uuid(uuid_like(999));
    let refused = [
        Command::DeleteCharacteristics {
            ids: vec![common::order(&before)[0], unknown],
        },
        Command::UpdateFields {
            ids: vec![unknown],
            values: vec![],
        },
        Command::UpdateFields {
            ids: vec![common::order(&before)[0]],
            values: vec![FieldValue::Quantity(0)],
        },
        common::add_at(&before, f64::INFINITY, 0.0, vec![]),
        Command::AddCharacteristic {
            sheet: dimo_core::SheetId::from_uuid(uuid_like(998)),
            position: Point { x: 0.0, y: 0.0 },
            anchor: Point { x: 0.0, y: 0.0 },
            region: None,
            values: vec![],
            insert_after: None,
        },
        // A batch whose second command fails rolls back the first.
        Command::Batch {
            commands: vec![
                common::add_at(&before, 1.0, 1.0, vec![]),
                Command::MoveCharacteristics {
                    ids: vec![unknown],
                    before: None,
                },
            ],
        },
    ];
    for command in refused {
        assert!(
            doc.execute(command.clone(), &mut env).is_err(),
            "{command:?}"
        );
        assert_eq!(*doc.project(), before, "{command:?}");
    }
    assert_eq!(doc.audit().len(), audit_len);
}

fn uuid_like(n: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(n)
}

#[test]
fn batch_is_one_undo_step_and_one_audit_entry() {
    let (mut doc, mut env) = doc_with(1);
    let start = doc.project().clone();
    let first = common::order(&start)[0];
    doc.execute(
        Command::Batch {
            commands: vec![
                common::add_at(&start, 300.0, 300.0, vec![]),
                common::add_at(&start, 400.0, 300.0, vec![]),
                Command::DeleteCharacteristics { ids: vec![first] },
            ],
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(
        common::numbers(doc.project()),
        [1, 2].map(dimo_core::DisplayNumber::plain)
    );
    let entry = doc.audit().last().unwrap();
    assert!(matches!(
        &entry.action,
        AuditAction::Command {
            command: Command::Batch { .. }
        }
    ));
    doc.undo(&mut env).unwrap();
    assert_eq!(*doc.project(), start);
}

#[test]
fn audit_records_user_time_and_changes_for_commands_undo_and_redo() {
    let (mut doc, mut env) = doc_with(0);
    env.set_user("Anna");
    env.set_time(dimo_core::Timestamp::from_unix_millis(1_791_554_601_250));
    doc.execute(common::add_at(doc.project(), 1.0, 1.0, vec![]), &mut env)
        .unwrap();
    doc.undo(&mut env).unwrap();
    doc.redo(&mut env).unwrap();
    let audit = doc.take_audit();
    assert_eq!(audit.len(), 3);
    assert_eq!(doc.audit(), []);
    for entry in &audit {
        assert_eq!(entry.user, "Anna");
        assert_eq!(entry.timestamp.as_str(), "2026-10-09T14:03:21.250Z");
        assert_eq!(entry.changes.len(), 2);
    }
    assert!(matches!(audit[0].action, AuditAction::Command { .. }));
    assert!(matches!(audit[1].action, AuditAction::Undo { .. }));
    assert!(matches!(audit[2].action, AuditAction::Redo { .. }));
    assert_eq!(
        audit[1].changes,
        dimo_core::patch::invert(&audit[0].changes)
    );
    assert_eq!(audit[2].changes, audit[0].changes);

    // One JSON line per entry, as in audit.jsonl.
    let line = serde_json::to_string(&audit[0]).unwrap();
    assert!(!line.contains('\n'));
    let back: dimo_core::AuditEntry = serde_json::from_str(&line).unwrap();
    assert_eq!(back, audit[0]);
}

#[test]
fn undo_redo_errors_and_modified_flag() {
    let (mut doc, mut env) = doc_with(0);
    assert_eq!(doc.undo(&mut env), Err(CommandError::NothingToUndo));
    assert_eq!(doc.redo(&mut env), Err(CommandError::NothingToRedo));
    assert!(!doc.is_modified());
    doc.execute(common::add_at(doc.project(), 1.0, 1.0, vec![]), &mut env)
        .unwrap();
    assert!(doc.is_modified());
    doc.mark_saved();
    assert!(!doc.is_modified());
    doc.undo(&mut env).unwrap();
    assert!(doc.is_modified());
    doc.redo(&mut env).unwrap();
    assert!(!doc.is_modified());
    // A new command clears the redo history.
    doc.undo(&mut env).unwrap();
    doc.execute(common::add_at(doc.project(), 2.0, 2.0, vec![]), &mut env)
        .unwrap();
    assert!(!doc.can_redo());
}

#[test]
fn settings_and_info_commands() {
    let (mut doc, mut env) = doc_with(0);
    let style = BalloonStyle {
        shape: BalloonShape::Rectangle,
        size_mm: 5.0,
        ..BalloonStyle::default()
    };
    doc.execute(
        Command::SetDefaultBalloonStyle {
            style: style.clone(),
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(doc.project().settings.balloon_style, style);
    let mut info = doc.project().info.clone();
    info.customer = "Customer A".into();
    doc.execute(Command::UpdateProjectInfo { info: info.clone() }, &mut env)
        .unwrap();
    assert_eq!(doc.project().info, info);
    let bad = BalloonStyle {
        outline_mm: 0.0,
        ..BalloonStyle::default()
    };
    assert_eq!(
        doc.execute(Command::SetDefaultBalloonStyle { style: bad }, &mut env),
        Err(CommandError::InvalidStyle)
    );
}

#[test]
fn project_json_round_trips_and_matches_its_schema() {
    let (mut doc, mut env) = doc_with(2);
    let id = common::order(doc.project())[0];
    doc.execute(
        Command::UpdateFields {
            ids: vec![id],
            values: vec![
                FieldValue::Nominal(dec("30.000")),
                FieldValue::UpperDev(dec("0.021")),
                FieldValue::LowerDev(dec("0")),
                FieldValue::Fit(Some("H7".into())),
            ],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::LockNumbering {
            reason: LockReason::IssuedReport,
        },
        &mut env,
    )
    .unwrap();
    let json = serde_json::to_value(doc.project()).unwrap();
    let c = &json["characteristics"][0];
    assert_eq!(c["nominal"], "30.000");
    assert_eq!(c["upper_limit"], "30.021");
    assert_eq!(c["lower_dev"], "0");
    assert!(c["comment"].is_string());
    assert!(json["characteristics"][1]["nominal"].is_null());
    let back: dimo_core::Project = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(back, *doc.project());

    let schema = dimo_core::project_schema();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:project.schema.json", schema)
        .unwrap();
    let index = compiler
        .compile("urn:dimo:project.schema.json", &mut schemas)
        .unwrap();
    schemas.validate(&json, index).unwrap();

    let mut float = json;
    float["characteristics"][0]["nominal"] = serde_json::json!(30.0);
    assert!(schemas.validate(&float, index).is_err());
    assert!(serde_json::from_value::<dimo_core::Project>(float).is_err());
}
