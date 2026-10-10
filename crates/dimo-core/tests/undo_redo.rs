//! Property tests of the command engine (NFR-REL-02, rule 11).
//!
//! Random sequences of commands, undos and redos on a small project must
//! - restore the original project when everything is undone, and the final one when redone,
//! - match a model of snapshots after every undo and redo,
//! - leave the project unchanged when a command is refused,
//! - produce patches that turn the previous state into the next one,
//! - keep the numbering and reference invariants,
//! - give byte identical JSON when replayed with the same environment.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::characteristic::FieldValue;
use dimo_core::{
    BalloonId, BalloonMove, BalloonPlacement, BalloonShape, BalloonStyle, BalloonStyleOverride,
    CharId, CharacteristicKind, Command, DecimalPlaceRule, DerivationRule, Document,
    FixedEnvironment, InsertPolicy, LockReason, MultiInstance, NumberingSettings,
    NumberingStrategy, OrientedBox, Origin, ParseHint, Point, Project, Proposal, RangeBound, Rect,
    Rotation, Scale, SheetView, Size, SizeRange, SourceRegion, TableClass, TableLookup, TableRef,
    TextSource, ToleranceDerivation, ToleranceSettings, Unit, UnitRounding, ZoneGrid,
};
use proptest::prelude::*;
use rust_decimal::Decimal;

/// A command in terms of positions, resolved against the current project when it runs.
#[derive(Debug, Clone)]
enum Op {
    Add {
        sheet: usize,
        x: i16,
        y: i16,
        region: bool,
        value: Option<Value>,
        after: Option<usize>,
    },
    Accept {
        sheet: usize,
        count: usize,
        x: i16,
        after: Option<usize>,
        valid: bool,
    },
    ZoneGrid {
        sheet: usize,
        columns: Option<u8>,
    },
    Views {
        sheet: usize,
        count: u8,
    },
    Numbering {
        policy: u8,
        strategy: u8,
        sub_number: bool,
    },
    Tolerance {
        class: Option<u8>,
        places: Vec<u32>,
        rounding: u32,
    },
    Update {
        picks: Vec<usize>,
        value: Value,
    },
    MoveBalloons {
        picks: Vec<usize>,
        dx: i16,
        anchor: bool,
    },
    Delete {
        picks: Vec<usize>,
    },
    Restyle {
        picks: Vec<usize>,
        shape: u8,
    },
    ResetStyle {
        picks: Vec<usize>,
    },
    MoveChars {
        picks: Vec<usize>,
        before: Option<usize>,
    },
    Sheet {
        sheet: usize,
        rotation: u8,
        inch: bool,
        scale: (u32, u32),
    },
    /// Reorder and renumber by a strategy (FR-BAL-04).
    ApplyNumbering {
        strategy: u8,
    },
    Lock,
    Unlock,
    DefaultStyle {
        quarter_mm: u8,
    },
    Info {
        part: u8,
    },
    /// Refers to a characteristic that does not exist.
    Unknown,
    Batch(Vec<Op>),
}

#[derive(Debug, Clone)]
enum Value {
    Nominal(i32, u32),
    Deviations(i32, i32),
    Kind(u8),
    Quantity(u32),
    Comment(u8),
    /// A derivation set with explicit limits, as a re-interpretation would (FR-TOL-08).
    Derivation(Option<u8>),
}

#[derive(Debug, Clone)]
enum Action {
    Do(Op),
    Undo,
    Redo,
}

fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        (any::<i32>(), 0u32..5).prop_map(|(m, s)| Value::Nominal(m, s)),
        (-500i32..500, -500i32..500).prop_map(|(u, l)| Value::Deviations(u, l)),
        any::<u8>().prop_map(Value::Kind),
        (0u32..5).prop_map(Value::Quantity),
        any::<u8>().prop_map(Value::Comment),
        prop::option::of(0u8..4).prop_map(Value::Derivation),
    ]
}

fn picks() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0usize..8, 0..4)
}

fn simple_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0usize..2, any::<i16>(), any::<i16>(), any::<bool>(), prop::option::of(value()),
              prop::option::of(0usize..8))
            .prop_map(|(sheet, x, y, region, value, after)| Op::Add { sheet, x, y, region, value, after }),
        2 => (0usize..2, 0usize..3, any::<i16>(), prop::option::of(0usize..8), prop::bool::weighted(0.9))
            .prop_map(|(sheet, count, x, after, valid)| Op::Accept { sheet, count, x, after, valid }),
        1 => (0usize..2, prop::option::of(0u8..4))
            .prop_map(|(sheet, columns)| Op::ZoneGrid { sheet, columns }),
        1 => (0usize..2, 0u8..3).prop_map(|(sheet, count)| Op::Views { sheet, count }),
        1 => (0u8..3, 0u8..5, any::<bool>())
            .prop_map(|(policy, strategy, sub_number)| Op::Numbering { policy, strategy, sub_number }),
        1 => (prop::option::of(0u8..3), prop::collection::vec(0u32..4, 0..3), 0u32..12)
            .prop_map(|(class, places, rounding)| Op::Tolerance { class, places, rounding }),
        3 => (picks(), value()).prop_map(|(picks, value)| Op::Update { picks, value }),
        2 => (picks(), any::<i16>(), any::<bool>())
            .prop_map(|(picks, dx, anchor)| Op::MoveBalloons { picks, dx, anchor }),
        2 => picks().prop_map(|picks| Op::Delete { picks }),
        1 => (picks(), 0u8..4).prop_map(|(picks, shape)| Op::Restyle { picks, shape }),
        1 => picks().prop_map(|picks| Op::ResetStyle { picks }),
        2 => (picks(), prop::option::of(0usize..8))
            .prop_map(|(picks, before)| Op::MoveChars { picks, before }),
        1 => (0usize..2, 0u8..4, any::<bool>(), (0u32..3, 0u32..3))
            .prop_map(|(sheet, rotation, inch, scale)| Op::Sheet { sheet, rotation, inch, scale }),
        2 => (0u8..5).prop_map(|strategy| Op::ApplyNumbering { strategy }),
        1 => Just(Op::Lock),
        1 => Just(Op::Unlock),
        1 => (0u8..40).prop_map(|quarter_mm| Op::DefaultStyle { quarter_mm }),
        1 => any::<u8>().prop_map(|part| Op::Info { part }),
        1 => Just(Op::Unknown),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => simple_op(),
        1 => prop::collection::vec(simple_op(), 0..4).prop_map(Op::Batch),
    ]
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        6 => op().prop_map(Action::Do),
        2 => Just(Action::Undo),
        1 => Just(Action::Redo),
    ]
}

fn chars(project: &Project, picks: &[usize]) -> Vec<CharId> {
    let ids = common::order(project);
    if ids.is_empty() {
        return Vec::new();
    }
    picks.iter().map(|p| ids[p % ids.len()]).collect()
}

fn balloons(project: &Project, picks: &[usize]) -> Vec<BalloonId> {
    let ids: Vec<BalloonId> = project.balloons.iter().map(|b| b.id).collect();
    if ids.is_empty() {
        return Vec::new();
    }
    picks.iter().map(|p| ids[p % ids.len()]).collect()
}

fn field_values(value: &Value) -> Vec<FieldValue> {
    match value {
        Value::Nominal(mantissa, scale) => vec![FieldValue::Nominal(
            Decimal::new(i64::from(*mantissa), *scale).into(),
        )],
        Value::Deviations(upper, lower) => vec![
            FieldValue::UpperDev(Decimal::new(i64::from(*upper), 3).into()),
            FieldValue::LowerDev(Decimal::new(i64::from(*lower), 3).into()),
        ],
        Value::Kind(k) => vec![FieldValue::Kind(if k % 2 == 0 {
            CharacteristicKind::Angle
        } else {
            CharacteristicKind::Linear
        })],
        Value::Quantity(q) => vec![FieldValue::Quantity(*q)],
        Value::Comment(c) => vec![FieldValue::Comment(format!("c{c}"))],
        Value::Derivation(rule) => vec![
            FieldValue::UpperLimit(Decimal::new(101, 1).into()),
            FieldValue::LowerLimit(Decimal::new(99, 1).into()),
            FieldValue::Derivation(rule.map(|r| {
                let mut derivation = ToleranceDerivation::new(match r {
                    0 => DerivationRule::Explicit,
                    1 => DerivationRule::NoToleranceDefined,
                    2 => DerivationRule::DecimalRule {
                        places: 1,
                        tolerance: Decimal::new(1, 1),
                    },
                    _ => DerivationRule::General {
                        lookup: TableLookup {
                            table: table_ref("iso-2768-1"),
                            part: "linear".into(),
                            class: "m".into(),
                            range: SizeRange {
                                min: None,
                                max: Some(RangeBound {
                                    value: Decimal::new(3, 0),
                                    inclusive: true,
                                }),
                            },
                        },
                    },
                });
                derivation.draft = r % 2 == 0;
                derivation
            })),
        ],
    }
}

fn table_ref(id: &str) -> TableRef {
    TableRef {
        id: id.into(),
        version: 1,
    }
}

fn rect(x: f64, width: f64) -> Rect {
    Rect {
        origin: Point { x, y: 10.0 },
        size: Size {
            width,
            height: 100.0,
        },
    }
}

/// `count` proposals read from boxes on `sheet`; with `valid` false the last one has quantity 0
/// and the whole accept is refused.
fn proposals(project: &Project, sheet: usize, count: usize, x: i16, valid: bool) -> Vec<Proposal> {
    let sheet = common::sheet(project, sheet);
    (0..count)
        .map(|i| {
            let at = Point {
                x: f64::from(x) / 4.0 + 30.0 * f64::from(u32::try_from(i).unwrap()),
                y: 50.0,
            };
            Proposal {
                kind: CharacteristicKind::Diameter,
                requirement_text: format!(" Ø{i} H7 "),
                nominal: Some(Decimal::new(i64::try_from(i).unwrap() + 8, 0)),
                unit: None,
                upper_dev: Some(Decimal::new(15, 3)),
                lower_dev: Some(Decimal::ZERO),
                upper_limit: Some(Decimal::new(8015, 3)),
                lower_limit: Some(Decimal::new(8, 0)),
                fit: Some("H7".into()),
                derivation: Some(ToleranceDerivation::new(DerivationRule::Fit {
                    table: table_ref("iso-286"),
                    fit: "H7".into(),
                    range: None,
                })),
                // Quantities 1 to 3; with sub-numbers a quantity above 1 is split (D-22).
                quantity: if valid || i + 1 < count {
                    u32::try_from(i % 3 + 1).unwrap()
                } else {
                    0
                },
                inspect: i % 2 == 0,
                source: SourceRegion {
                    sheet,
                    region: OrientedBox {
                        center: at,
                        size: Size {
                            width: 20.0,
                            height: 5.0,
                        },
                        angle: 90.0,
                    },
                    text_source: TextSource::PdfText,
                    raw_text: Some(format!("Ø{i} H7")),
                },
                origin: Origin::BoxSelect,
                placement: BalloonPlacement {
                    position: Point {
                        x: at.x + 10.0,
                        y: at.y - 10.0,
                    },
                    anchor: at,
                },
                parse_error: None,
                parse_hints: vec![ParseHint::StackedLinesJoined],
            }
        })
        .collect()
}

fn strategy_of(index: u8) -> NumberingStrategy {
    [
        NumberingStrategy::SheetZone,
        NumberingStrategy::View,
        NumberingStrategy::ViewClockwise,
        NumberingStrategy::Kind,
        NumberingStrategy::Manual,
    ][usize::from(index)]
}

#[allow(clippy::too_many_lines, reason = "one arm per operation")]
fn command(project: &Project, op: &Op) -> Command {
    match op {
        Op::Add {
            sheet,
            x,
            y,
            region,
            value,
            after,
        } => {
            let at = Point {
                x: f64::from(*x) / 4.0,
                y: f64::from(*y) / 4.0,
            };
            Command::AddCharacteristic {
                sheet: common::sheet(project, *sheet),
                position: at,
                anchor: Point {
                    x: at.x - 8.0,
                    y: at.y,
                },
                region: region.then_some(OrientedBox {
                    center: at,
                    size: Size {
                        width: 12.0,
                        height: 4.0,
                    },
                    angle: 0.0,
                }),
                values: value.as_ref().map(field_values).unwrap_or_default(),
                insert_after: after.and_then(|a| chars(project, &[a]).first().copied()),
            }
        }
        Op::Accept {
            sheet,
            count,
            x,
            after,
            valid,
        } => Command::AcceptProposals {
            proposals: proposals(project, *sheet, *count, *x, *valid),
            insert_after: after.and_then(|a| chars(project, &[a]).first().copied()),
        },
        Op::ZoneGrid { sheet, columns } => Command::SetZoneGrid {
            sheet: common::sheet(project, *sheet),
            // Zero columns is invalid and refused.
            grid: columns.map(|n| ZoneGrid {
                frame: rect(20.0, 800.0),
                column_labels: (1..=n).map(|i| i.to_string()).collect(),
                row_labels: vec!["A".into(), "B".into()],
            }),
        },
        Op::Views { sheet, count } => Command::SetViews {
            sheet: common::sheet(project, *sheet),
            views: (0..*count)
                .map(|i| SheetView {
                    label: format!("V{i}"),
                    rect: rect(f64::from(i) * 200.0, 150.0),
                })
                .collect(),
        },
        Op::Numbering {
            policy,
            strategy,
            sub_number,
        } => Command::SetNumberingSettings {
            settings: NumberingSettings {
                strategy: strategy_of(*strategy),
                multi_instance: if *sub_number {
                    MultiInstance::SubNumber
                } else {
                    MultiInstance::Quantity
                },
                insert_when_locked: [
                    InsertPolicy::NextFree,
                    InsertPolicy::SubNumber,
                    InsertPolicy::LetterSuffix,
                ][usize::from(*policy)],
            },
        },
        // Unsorted or duplicate places and rounding above 10 are refused.
        Op::Tolerance {
            class,
            places,
            rounding,
        } => Command::SetToleranceSettings {
            settings: ToleranceSettings {
                general: class.map(|c| TableClass {
                    table: table_ref("iso-2768-1"),
                    class: ["f", "m", "c"][usize::from(c)].into(),
                }),
                drawing_rule: None,
                decimal_rules: places
                    .iter()
                    .map(|p| DecimalPlaceRule {
                        places: *p,
                        tolerance: Decimal::new(5, *p + 1),
                    })
                    .collect(),
                unit_rounding: UnitRounding {
                    mm_places: *rounding,
                    inch_places: 4,
                },
                custom_tables: Vec::new(),
            },
        },
        Op::Update { picks, value } => Command::UpdateFields {
            ids: chars(project, picks),
            values: field_values(value),
        },
        Op::MoveBalloons { picks, dx, anchor } => Command::MoveBalloons {
            moves: balloons(project, picks)
                .into_iter()
                .map(|id| BalloonMove {
                    id,
                    position: Point {
                        x: f64::from(*dx) / 4.0,
                        y: 10.0,
                    },
                    anchor: anchor.then_some(Point { x: 0.5, y: 0.25 }),
                })
                .collect(),
        },
        Op::Delete { picks } => Command::DeleteCharacteristics {
            ids: chars(project, picks),
        },
        Op::Restyle { picks, shape } => Command::RestyleBalloons {
            ids: balloons(project, picks),
            style: BalloonStyleOverride {
                shape: match shape {
                    0 => Some(BalloonShape::Circle),
                    1 => Some(BalloonShape::Flag),
                    2 => Some(BalloonShape::Rectangle),
                    _ => None,
                },
                leader: Some(shape % 2 == 0),
                ..Default::default()
            },
        },
        Op::ResetStyle { picks } => Command::ResetBalloonStyle {
            ids: balloons(project, picks),
        },
        Op::MoveChars { picks, before } => Command::MoveCharacteristics {
            ids: chars(project, picks),
            before: before.and_then(|b| chars(project, &[b]).first().copied()),
        },
        Op::Sheet {
            sheet,
            rotation,
            inch,
            scale,
        } => Command::UpdateSheet {
            sheet: common::sheet(project, *sheet),
            rotation: Some(
                [
                    Rotation::Deg0,
                    Rotation::Deg90,
                    Rotation::Deg180,
                    Rotation::Deg270,
                ][usize::from(*rotation)],
            ),
            unit: Some(if *inch { Unit::In } else { Unit::Mm }),
            scale: Some(Scale {
                drawing: scale.0,
                actual: scale.1,
            }),
        },
        Op::ApplyNumbering { strategy } => Command::ApplyNumbering {
            strategy: strategy_of(*strategy),
        },
        Op::Lock => Command::LockNumbering {
            reason: LockReason::Manual,
        },
        Op::Unlock => Command::UnlockNumbering,
        Op::DefaultStyle { quarter_mm } => Command::SetDefaultBalloonStyle {
            style: BalloonStyle {
                size_mm: f64::from(*quarter_mm) / 4.0,
                ..BalloonStyle::default()
            },
        },
        Op::Info { part } => Command::UpdateProjectInfo {
            info: dimo_core::ProjectInfo {
                part_number: format!("P-{part}"),
                ..dimo_core::ProjectInfo::default()
            },
        },
        Op::Unknown => Command::DeleteCharacteristics {
            ids: vec![CharId::from_uuid(uuid::Uuid::from_u128(u128::MAX))],
        },
        // Batch members are resolved against the state before the batch, so later members may
        // refer to characteristics that earlier members deleted. That exercises rollback.
        Op::Batch(ops) => Command::Batch {
            commands: ops.iter().map(|op| command(project, op)).collect(),
        },
    }
}

/// Patches must be complete: replaying them on the old state gives the new state.
fn replay(before: &Project, changes: &[dimo_core::Change]) -> Project {
    let mut project = before.clone();
    for change in changes {
        project.apply_change(change).unwrap();
    }
    project
}

fn fresh() -> (Document, FixedEnvironment) {
    let mut env = common::env();
    let project = common::project(&mut env);
    (Document::new(project), env)
}

/// Runs the commands, returns the document and the number of undo steps they created.
fn run_commands(ops: &[Op]) -> (Document, FixedEnvironment, usize) {
    let (mut doc, mut env) = fresh();
    let mut steps = 0;
    for op in ops {
        let before = doc.project().clone();
        let cmd = command(&before, op);
        match doc.execute(cmd, &mut env) {
            Ok(patch) => {
                assert_eq!(replay(&before, &patch.changes), *doc.project());
                steps += usize::from(!patch.is_empty());
            }
            Err(_) => assert_eq!(
                *doc.project(),
                before,
                "refused command changed the project"
            ),
        }
        common::check_invariants(doc.project());
    }
    (doc, env, steps)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn undo_all_restores_original_and_redo_all_restores_final(
        ops in prop::collection::vec(op(), 0..40)
    ) {
        let original = fresh().0.project().clone();
        let (mut doc, mut env, steps) = run_commands(&ops);
        let last = doc.project().clone();

        let mut undone = 0;
        while doc.can_undo() {
            let before = doc.project().clone();
            let patch = doc.undo(&mut env).unwrap();
            prop_assert_eq!(&replay(&before, &patch.changes), doc.project());
            common::check_invariants(doc.project());
            undone += 1;
        }
        prop_assert_eq!(undone, steps);
        prop_assert_eq!(doc.project(), &original);

        while doc.can_redo() {
            doc.redo(&mut env).unwrap();
        }
        prop_assert_eq!(doc.project(), &last);

        // Rule 11: same commands and environment give byte identical JSON.
        let (again, _, _) = run_commands(&ops);
        prop_assert_eq!(
            serde_json::to_string(again.project()).unwrap(),
            serde_json::to_string(&last).unwrap()
        );
        let back: Project = serde_json::from_str(&serde_json::to_string(&last).unwrap()).unwrap();
        prop_assert_eq!(back, last);
    }

    #[test]
    fn undo_and_redo_match_snapshots(actions in prop::collection::vec(action(), 0..60)) {
        let (mut doc, mut env) = fresh();
        // State before each undoable step, and state after each redoable step.
        let mut undo_model: Vec<Project> = Vec::new();
        let mut redo_model: Vec<Project> = Vec::new();
        for action in &actions {
            let before = doc.project().clone();
            match action {
                Action::Do(op) => {
                    let cmd = command(&before, op);
                    match doc.execute(cmd, &mut env) {
                        Ok(patch) if !patch.is_empty() => {
                            undo_model.push(before);
                            redo_model.clear();
                        }
                        Ok(_) => prop_assert_eq!(doc.project(), &before),
                        Err(_) => prop_assert_eq!(doc.project(), &before),
                    }
                }
                Action::Undo => match undo_model.pop() {
                    Some(expected) => {
                        doc.undo(&mut env).unwrap();
                        prop_assert_eq!(doc.project(), &expected);
                        redo_model.push(before);
                    }
                    None => prop_assert!(doc.undo(&mut env).is_err()),
                },
                Action::Redo => match redo_model.pop() {
                    Some(expected) => {
                        doc.redo(&mut env).unwrap();
                        prop_assert_eq!(doc.project(), &expected);
                        undo_model.push(before);
                    }
                    None => prop_assert!(doc.redo(&mut env).is_err()),
                },
            }
            prop_assert_eq!(doc.can_undo(), !undo_model.is_empty());
            prop_assert_eq!(doc.can_redo(), !redo_model.is_empty());
            common::check_invariants(doc.project());
        }
    }
}
