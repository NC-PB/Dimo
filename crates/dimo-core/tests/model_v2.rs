//! Data model version 2 (T2.4): proposals and their accept command (ADR 0006), tolerance
//! derivations (FR-TOL-08), project tolerance and numbering settings, zone grids and views
//! (M2 decisions 1 and 2), and the change history per characteristic (FR-CHR-10).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    BalloonPlacement, Change, ChangeSource, CharacteristicField, CharacteristicKind, Command,
    CommandError, DecimalPlaceRule, DerivationHint, DerivationRule, Document, FieldError,
    FixedEnvironment, HistoryAction, InsertPolicy, LockReason, NumberingSettings,
    NumberingStrategy, OrientedBox, Origin, ParseIssue, Point, Proposal, Rect, SheetView, Size,
    SourceRegion, TableClass, TableRef, TextSource, ToleranceDerivation, ToleranceSettings,
    ZoneGrid, characteristic_history,
};
use rust_decimal::Decimal;

fn dec(text: &str) -> Option<Decimal> {
    parse_decimal(text)
}

fn fresh() -> (Document, FixedEnvironment) {
    let mut env = common::env();
    let doc = Document::new(common::project(&mut env));
    (doc, env)
}

fn proposal(doc: &Document, text: &str, x: f64) -> Proposal {
    let at = Point { x, y: 300.0 };
    Proposal {
        kind: CharacteristicKind::Diameter,
        requirement_text: text.into(),
        nominal: dec("30"),
        unit: None,
        upper_dev: dec("0.0203"),
        lower_dev: dec("0"),
        upper_limit: dec("30.0203"),
        lower_limit: dec("30"),
        fit: Some(" H7 ".into()),
        derivation: Some(ToleranceDerivation {
            rule: DerivationRule::Explicit,
            draft: false,
            hints: vec![DerivationHint::FitDeviationsDiffer {
                fit: "H7".into(),
                table_upper_dev: dec("0.021"),
                table_lower_dev: dec("0"),
            }],
            conversion: None,
        }),
        quantity: 2,
        inspect: true,
        source: SourceRegion {
            sheet: common::sheet(doc.project(), 0),
            region: OrientedBox {
                center: at,
                size: Size {
                    width: 40.0,
                    height: 8.0,
                },
                angle: 0.0,
            },
            text_source: TextSource::PdfText,
            raw_text: Some(text.into()),
        },
        origin: Origin::BoxSelect,
        placement: BalloonPlacement {
            position: Point {
                x: x + 30.0,
                y: 280.0,
            },
            anchor: at,
        },
        parse_error: None,
        parse_hints: Vec::new(),
        job_id: Some(7),
        engines: vec![dimo_core::EngineVersion::new("dimo-notation", "0.1.0")],
    }
}

#[test]
fn accepting_proposals_is_one_undo_step() {
    let (mut doc, mut env) = fresh();
    doc.execute(common::add_at(doc.project(), 50.0, 50.0, vec![]), &mut env)
        .unwrap();
    let before = doc.project().clone();
    let proposals = vec![
        proposal(&doc, "2X Ø30 H7 +0.0203 0", 100.0),
        proposal(&doc, "Ø30 H7", 200.0),
    ];
    let patch = doc
        .execute(
            Command::AcceptProposals {
                proposals: proposals.clone(),
                insert_after: None,
            },
            &mut env,
        )
        .unwrap();
    assert_eq!(
        patch.changes.len(),
        4,
        "a characteristic and a balloon each"
    );
    let project = doc.project();
    assert_eq!(
        common::numbers(project)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
    let accepted = &project.characteristics[1];
    assert_eq!(accepted.origin, Origin::BoxSelect);
    assert_eq!(accepted.requirement_text, "2X Ø30 H7 +0.0203 0");
    assert_eq!(accepted.fit.as_deref(), Some("H7"));
    assert_eq!(accepted.upper_limit, dec("30.0203"));
    assert_eq!(accepted.quantity, 2);
    assert_eq!(accepted.unit, Some(dimo_core::Unit::Mm), "sheet unit");
    assert_eq!(accepted.derivation, proposals[0].derivation);
    assert_eq!(accepted.sources, vec![proposals[0].source.clone()]);
    let balloon = project.balloons_of(accepted.id).next().unwrap();
    assert_eq!(balloon.position, proposals[0].placement.position);
    assert_eq!(balloon.anchor, proposals[0].placement.anchor);

    doc.undo(&mut env).unwrap();
    assert_eq!(doc.project(), &before, "one undo removes both");
    doc.redo(&mut env).unwrap();
    assert_eq!(doc.project().characteristics.len(), 3);
}

#[test]
fn accepting_while_locked_uses_the_insert_policy() {
    let (mut doc, mut env) = fresh();
    for x in [10.0, 20.0] {
        doc.execute(common::add_at(doc.project(), x, 50.0, vec![]), &mut env)
            .unwrap();
    }
    let first = doc.project().characteristics[0].id;
    doc.execute(
        Command::SetNumberingSettings {
            settings: NumberingSettings {
                insert_when_locked: InsertPolicy::LetterSuffix,
                ..NumberingSettings::default()
            },
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::LockNumbering {
            reason: LockReason::Manual,
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::AcceptProposals {
            proposals: vec![proposal(&doc, "a", 1.0), proposal(&doc, "b", 2.0)],
            insert_after: Some(first),
        },
        &mut env,
    )
    .unwrap();
    let numbers: Vec<String> = common::numbers(doc.project())
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(numbers, ["1", "1A", "1B", "2"]);
}

#[test]
fn invalid_proposals_are_refused_as_a_whole() {
    let (mut doc, mut env) = fresh();
    let before = doc.project().clone();
    let good = proposal(&doc, "Ø30", 100.0);
    let mut zero = proposal(&doc, "Ø30", 200.0);
    zero.quantity = 0;
    let mut no_size = proposal(&doc, "Ø30", 200.0);
    no_size.source.region.size.width = 0.0;
    let mut far = proposal(&doc, "Ø30", 200.0);
    far.placement.position.x = f64::NAN;
    let mut unknown_sheet = proposal(&doc, "Ø30", 200.0);
    unknown_sheet.source.sheet = dimo_core::SheetId::from_uuid(uuid::Uuid::from_u128(999));
    for (bad, error) in [
        (zero, CommandError::Field(FieldError::ZeroQuantity)),
        (no_size, CommandError::InvalidGeometry("region")),
        (
            far,
            CommandError::InvalidGeometry("balloon position or anchor"),
        ),
        (
            unknown_sheet.clone(),
            CommandError::UnknownSheet(unknown_sheet.source.sheet),
        ),
    ] {
        let result = doc.execute(
            Command::AcceptProposals {
                proposals: vec![good.clone(), bad],
                insert_after: None,
            },
            &mut env,
        );
        assert_eq!(result, Err(error));
        assert_eq!(doc.project(), &before);
    }
}

#[test]
fn a_proposal_with_a_parse_error_keeps_its_raw_text_and_has_no_limits() {
    let (mut doc, mut env) = fresh();
    let mut p = proposal(&doc, "Ø3O H7", 100.0);
    p.nominal = None;
    p.upper_dev = None;
    p.lower_dev = None;
    p.upper_limit = None;
    p.lower_limit = None;
    p.derivation = None;
    p.parse_error = Some(ParseIssue {
        position: 3,
        expected: "digit".into(),
    });
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["parse_error"]["position"], 3);
    assert_eq!(serde_json::from_value::<Proposal>(json).unwrap(), p);
    doc.execute(
        Command::AcceptProposals {
            proposals: vec![p],
            insert_after: None,
        },
        &mut env,
    )
    .unwrap();
    let c = &doc.project().characteristics[0];
    assert_eq!(c.requirement_text, "Ø3O H7");
    assert_eq!((c.upper_limit, c.lower_limit, c.unit), (None, None, None));
}

/// T2.6: proposals carry job ID and engine versions (data model `Proposal`); audit entries
/// written before T2.6 have neither and still read.
#[test]
fn proposals_without_job_and_engines_still_read() {
    let (doc, _) = fresh();
    let p = proposal(&doc, "Ø30 H7", 100.0);
    let mut json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["job_id"], 7);
    assert_eq!(json["engines"][0]["name"], "dimo-notation");
    let object = json.as_object_mut().unwrap();
    object.remove("job_id");
    object.remove("engines");
    let old: Proposal = serde_json::from_value(json).unwrap();
    assert_eq!(old.job_id, None);
    assert_eq!(old.engines, Vec::new());
}

#[test]
fn hand_edits_of_limits_make_the_derivation_manual() {
    let (mut doc, mut env) = fresh();
    doc.execute(
        Command::AcceptProposals {
            proposals: vec![proposal(&doc, "Ø30 H7", 100.0)],
            insert_after: None,
        },
        &mut env,
    )
    .unwrap();
    let id = doc.project().characteristics[0].id;
    let derivation = |doc: &Document| doc.project().characteristics[0].derivation.clone();
    let update = |values| Command::UpdateFields {
        ids: vec![id],
        values,
    };

    // Text and comment edits keep the rule.
    doc.execute(update(vec![FieldValue::Comment("x".into())]), &mut env)
        .unwrap();
    assert_eq!(
        derivation(&doc).unwrap().rule,
        DerivationRule::Explicit,
        "FR-TOL-08"
    );
    // A new nominal moves the limits: they no longer come from the rule.
    doc.execute(
        update(vec![FieldValue::Nominal(dec("31").into())]),
        &mut env,
    )
    .unwrap();
    assert_eq!(derivation(&doc), Some(ToleranceDerivation::manual()));
    // A rule sets limits and derivation together.
    let general = ToleranceDerivation::new(DerivationRule::NoToleranceDefined);
    doc.execute(
        update(vec![
            FieldValue::UpperLimit(None.into()),
            FieldValue::LowerLimit(None.into()),
            FieldValue::Derivation(Some(general.clone())),
        ]),
        &mut env,
    )
    .unwrap();
    assert_eq!(derivation(&doc), Some(general));
    // Clearing limits by hand clears the derivation.
    doc.execute(
        update(vec![FieldValue::UpperLimit(dec("1").into())]),
        &mut env,
    )
    .unwrap();
    assert_eq!(derivation(&doc), Some(ToleranceDerivation::manual()));
    doc.execute(update(vec![FieldValue::UpperLimit(None.into())]), &mut env)
        .unwrap();
    assert_eq!(derivation(&doc), None);
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        origin: Point { x, y },
        size: Size { width, height },
    }
}

#[test]
fn zone_grid_and_views_are_sheet_changes() {
    let (mut doc, mut env) = fresh();
    let sheet = common::sheet(doc.project(), 0);
    let grid = ZoneGrid {
        frame: rect(20.0, 20.0, 1150.0, 800.0),
        column_labels: (1..=8).map(|i| i.to_string()).collect(),
        row_labels: ["A", "B", "C", "D", "E", "F"].map(String::from).to_vec(),
    };
    let patch = doc
        .execute(
            Command::SetZoneGrid {
                sheet,
                grid: Some(grid.clone()),
            },
            &mut env,
        )
        .unwrap();
    assert!(matches!(
        patch.changes.as_slice(),
        [Change::SheetChanged { after, .. }] if after.zone_grid.as_ref() == Some(&grid)
    ));
    let views = vec![
        SheetView {
            label: "A-A".into(),
            rect: rect(100.0, 100.0, 300.0, 200.0),
        },
        SheetView {
            label: String::new(),
            rect: rect(500.0, 100.0, 300.0, 200.0),
        },
    ];
    doc.execute(
        Command::SetViews {
            sheet,
            views: views.clone(),
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(doc.project().sheet(sheet).unwrap().views, views);
    // Same value again: no change, no undo step.
    assert!(
        doc.execute(
            Command::SetViews {
                sheet,
                views: views.clone()
            },
            &mut env
        )
        .unwrap()
        .is_empty()
    );
    doc.undo(&mut env).unwrap();
    assert_eq!(doc.project().sheet(sheet).unwrap().views, []);

    let invalid_grids = [
        ZoneGrid {
            column_labels: Vec::new(),
            ..grid.clone()
        },
        ZoneGrid {
            row_labels: vec!["A".into(), "A".into()],
            ..grid.clone()
        },
        ZoneGrid {
            column_labels: vec![" ".into()],
            ..grid.clone()
        },
        ZoneGrid {
            frame: rect(0.0, 0.0, -1.0, 10.0),
            ..grid.clone()
        },
    ];
    for bad in invalid_grids {
        assert!(matches!(
            doc.execute(
                Command::SetZoneGrid {
                    sheet,
                    grid: Some(bad)
                },
                &mut env
            ),
            Err(CommandError::InvalidSheetSetting(_))
        ));
    }
    assert!(matches!(
        doc.execute(
            Command::SetViews {
                sheet,
                views: vec![SheetView {
                    label: String::new(),
                    rect: rect(f64::INFINITY, 0.0, 1.0, 1.0),
                }],
            },
            &mut env
        ),
        Err(CommandError::InvalidSheetSetting(_))
    ));
}

#[test]
fn project_settings_commands() {
    let (mut doc, mut env) = fresh();
    assert_eq!(
        doc.project().settings.numbering.strategy,
        NumberingStrategy::SheetZone,
        "D-21 default for new projects"
    );
    assert_eq!(
        doc.project().settings.tolerance.general,
        None,
        "M2 decision 2: no general tolerance until the user sets one"
    );
    let tolerance = ToleranceSettings {
        general: Some(TableClass {
            table: TableRef {
                id: "iso-2768-1".into(),
                version: 1,
            },
            class: "m".into(),
        }),
        decimal_rules: vec![DecimalPlaceRule {
            places: 1,
            tolerance: Decimal::new(1, 1),
        }],
        ..ToleranceSettings::default()
    };
    doc.execute(
        Command::SetToleranceSettings {
            settings: tolerance.clone(),
        },
        &mut env,
    )
    .unwrap();
    assert_eq!(doc.project().settings.tolerance, tolerance);
    let mut broken = tolerance.clone();
    broken.decimal_rules[0].tolerance = Decimal::ZERO;
    assert!(matches!(
        doc.execute(Command::SetToleranceSettings { settings: broken }, &mut env),
        Err(CommandError::InvalidProjectSetting(_))
    ));
    doc.undo(&mut env).unwrap();
    assert_eq!(
        doc.project().settings.tolerance,
        ToleranceSettings::default()
    );
}

#[test]
fn history_lists_who_when_what_and_source() {
    use CharacteristicField as F;

    let (mut doc, mut env) = fresh();
    env.set_time(dimo_core::Timestamp::parse("2026-05-01T10:00:00Z").unwrap());
    doc.execute(common::add_at(doc.project(), 10.0, 10.0, vec![]), &mut env)
        .unwrap();
    doc.execute(
        Command::AcceptProposals {
            proposals: vec![proposal(&doc, "Ø30 H7", 100.0)],
            insert_after: None,
        },
        &mut env,
    )
    .unwrap();
    let [first, second] = [0, 1].map(|i| doc.project().characteristics[i].id);
    env.set_user("checker");
    env.set_time(dimo_core::Timestamp::parse("2026-05-01T11:00:00Z").unwrap());
    doc.execute(
        Command::UpdateFields {
            ids: vec![second],
            values: vec![FieldValue::Comment("ok".into())],
        },
        &mut env,
    )
    .unwrap();
    doc.execute(
        Command::UpdateFields {
            ids: vec![second],
            values: vec![FieldValue::Derivation(Some(ToleranceDerivation::new(
                DerivationRule::NoToleranceDefined,
            )))],
        },
        &mut env,
    )
    .unwrap();
    // Deleting the first renumbers the second: a rule, not the user.
    doc.execute(
        Command::DeleteCharacteristics { ids: vec![first] },
        &mut env,
    )
    .unwrap();
    doc.undo(&mut env).unwrap();

    let history = characteristic_history(doc.audit(), second);
    let summary: Vec<_> = history
        .iter()
        .map(|h| (h.action, h.source, h.user.as_str(), h.fields.clone()))
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                HistoryAction::Command,
                ChangeSource::Recognition,
                "inspector",
                vec![]
            ),
            (
                HistoryAction::Command,
                ChangeSource::Manual,
                "checker",
                vec![F::Comment]
            ),
            (
                HistoryAction::Command,
                ChangeSource::Rule,
                "checker",
                vec![F::Derivation]
            ),
            (
                HistoryAction::Command,
                ChangeSource::Rule,
                "checker",
                vec![F::Number]
            ),
            (
                HistoryAction::Undo,
                ChangeSource::Rule,
                "checker",
                vec![F::Number]
            ),
        ]
    );
    assert!(history[0].before.is_none() && history[0].after.is_some());
    assert_eq!(history[0].timestamp.as_str(), "2026-05-01T10:00:00Z");
    assert_eq!(history[3].before.as_ref().unwrap().number.to_string(), "2");
    assert_eq!(history[3].after.as_ref().unwrap().number.to_string(), "1");

    let first_history = characteristic_history(doc.audit(), first);
    let sources: Vec<_> = first_history
        .iter()
        .map(|h| (h.action, h.source, h.after.is_some()))
        .collect();
    assert_eq!(
        sources,
        [
            (HistoryAction::Command, ChangeSource::Manual, true),
            (HistoryAction::Command, ChangeSource::Manual, false),
            (HistoryAction::Undo, ChangeSource::Manual, true),
        ]
    );
}

/// Regression (T2.6): typing `30.0` over a nominal of `30` keeps the typed digits. Decimal
/// equality ignores the scale, so the edit was dropped as "no change" before.
#[test]
fn typed_digits_replace_an_equal_value() {
    let (mut doc, mut env) = fresh();
    let p = proposal(&doc, "Ø30 H7", 100.0);
    doc.execute(
        Command::AcceptProposals {
            proposals: vec![p],
            insert_after: None,
        },
        &mut env,
    )
    .unwrap();
    let id = doc.project().characteristics[0].id;
    doc.execute(
        Command::UpdateFields {
            ids: vec![id],
            values: vec![FieldValue::Nominal(dec("30.0").into())],
        },
        &mut env,
    )
    .unwrap();
    let nominal = doc.project().characteristics[0].nominal.unwrap();
    assert_eq!(nominal.to_string(), "30.0");
    doc.undo(&mut env).unwrap();
    let nominal = doc.project().characteristics[0].nominal.unwrap();
    assert_eq!(nominal.to_string(), "30");
}
