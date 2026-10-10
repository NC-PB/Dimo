//! Numbering strategies on a synthetic layout with known expected orders (FR-BAL-04, FR-BAL-05,
//! FR-BAL-07, FR-BAL-11, D-21, D-22, D-23).
//!
//! Layout of sheet 0 (A3 landscape, 1190 x 842 units). Points are leader anchors:
//!
//! ```text
//!        x: 100   250  300  400       700  800  900  1000
//!  y 100   b    j(112) c(105)          d                       zone A1 | A2
//!    300                    e                                  V1 = (50,50)-(450,350)
//!    500                                     h
//!    600     f                              a                  zone B1 | B2
//!    700                                          g            V2 = (650,450)-(1050,800)
//! ```
//!
//! Zone grid: frame (10,10) 1160 x 820, columns `1`, `2` (split at x 590), rows `A`, `B` (split
//! at y 420). Sheet 1 holds `i` at (100,100), without grid and views.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use std::collections::BTreeMap;

use dimo_core::characteristic::FieldValue;
use dimo_core::numbering::{preview, strategy_order};
use dimo_core::{
    BalloonPlacement, CharId, CharacteristicKind, Command, CommandError, Document,
    FixedEnvironment, InsertPolicy, LockReason, MultiInstance, NumberingSettings,
    NumberingStrategy, OrientedBox, Origin, Point, Proposal, Rect, Rotation, SheetView, Size,
    SourceRegion, TextSource, ZoneGrid,
};

use CharacteristicKind as K;
use NumberingStrategy as S;

/// Label, sheet, anchor and kind of each characteristic, in the order they are added.
const LAYOUT: [(char, usize, f64, f64, CharacteristicKind); 10] = [
    ('a', 0, 800.0, 600.0, K::Diameter),
    ('b', 0, 100.0, 100.0, K::Linear),
    ('c', 0, 300.0, 105.0, K::Diameter),
    ('d', 0, 700.0, 100.0, K::Linear),
    ('e', 0, 400.0, 300.0, K::Radius),
    ('f', 0, 200.0, 600.0, K::Other),
    ('g', 0, 1000.0, 700.0, K::Linear),
    ('h', 0, 900.0, 500.0, K::Diameter),
    ('i', 1, 100.0, 100.0, K::Linear),
    ('j', 0, 250.0, 112.0, K::Angle),
];

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        origin: Point { x, y },
        size: Size { width, height },
    }
}

fn grid() -> ZoneGrid {
    ZoneGrid {
        frame: rect(10.0, 10.0, 1160.0, 820.0),
        column_labels: vec!["1".into(), "2".into()],
        row_labels: vec!["A".into(), "B".into()],
    }
}

fn views() -> Vec<SheetView> {
    vec![
        SheetView {
            label: "V1".into(),
            rect: rect(50.0, 50.0, 400.0, 300.0),
        },
        SheetView {
            label: "V2".into(),
            rect: rect(650.0, 450.0, 400.0, 350.0),
        },
    ]
}

struct Synth {
    doc: Document,
    env: FixedEnvironment,
    labels: BTreeMap<CharId, char>,
}

impl Synth {
    /// The layout with grid and views on sheet 0, in the order of [`LAYOUT`].
    fn new() -> Self {
        let mut env = common::env();
        let mut doc = Document::new(common::project(&mut env));
        let sheet0 = common::sheet(doc.project(), 0);
        let mut commands = vec![
            Command::SetZoneGrid {
                sheet: sheet0,
                grid: Some(grid()),
            },
            Command::SetViews {
                sheet: sheet0,
                views: views(),
            },
        ];
        for (_, sheet, x, y, kind) in LAYOUT {
            commands.push(Command::AddCharacteristic {
                sheet: common::sheet(doc.project(), sheet),
                position: Point {
                    x: x + 30.0,
                    y: y - 30.0,
                },
                anchor: Point { x, y },
                region: None,
                values: vec![FieldValue::Kind(kind)],
                insert_after: None,
            });
        }
        for command in commands {
            doc.execute(command, &mut env).unwrap();
        }
        let labels = doc
            .project()
            .characteristics
            .iter()
            .zip(LAYOUT)
            .map(|(c, (label, ..))| (c.id, label))
            .collect();
        Self { doc, env, labels }
    }

    fn run(&mut self, command: Command) -> Result<(), CommandError> {
        self.doc.execute(command, &mut self.env).map(|_| ())
    }

    fn sheet(&self, index: usize) -> dimo_core::SheetId {
        common::sheet(self.doc.project(), index)
    }

    fn label(&self, id: CharId) -> char {
        self.labels.get(&id).copied().unwrap_or('?')
    }

    /// Labels in the order `strategy` gives.
    fn order(&self, strategy: NumberingStrategy) -> String {
        strategy_order(self.doc.project(), strategy)
            .into_iter()
            .map(|id| self.label(id))
            .collect()
    }

    /// `label:number` in placement order.
    fn numbers(&self) -> String {
        self.doc
            .project()
            .characteristics
            .iter()
            .map(|c| format!("{}:{}", self.label(c.id), c.number))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[test]
#[allow(clippy::too_many_lines, reason = "one table of cases")]
fn each_strategy_gives_the_expected_order() {
    // FR-BAL-04, D-21: one row per strategy and layout variant.
    struct Case {
        name: &'static str,
        setup: fn(&mut Synth),
        strategy: NumberingStrategy,
        expected: &'static str,
    }
    let no_grid: fn(&mut Synth) = |s| {
        let sheet = s.sheet(0);
        s.run(Command::SetZoneGrid { sheet, grid: None }).unwrap();
    };
    let cases = [
        Case {
            name: "sheet, zone, reading order (default, D-21)",
            setup: |_| {},
            strategy: S::SheetZone,
            expected: "bjcedfhagi",
        },
        Case {
            name: "no zone grid falls back to reading order on the sheet",
            setup: no_grid,
            strategy: S::SheetZone,
            expected: "bjcdehfagi",
        },
        Case {
            name: "reading order as shown on a sheet turned 90 degrees",
            setup: |s| {
                let sheet = s.sheet(0);
                s.run(Command::SetZoneGrid { sheet, grid: None }).unwrap();
                s.run(Command::UpdateSheet {
                    sheet,
                    rotation: Some(Rotation::Deg90),
                    unit: None,
                    scale: None,
                })
                .unwrap();
            },
            strategy: S::SheetZone,
            expected: "bfjcedahgi",
        },
        Case {
            name: "zones are read as shown on a sheet turned 180 degrees",
            setup: |s| {
                let sheet = s.sheet(0);
                s.run(Command::UpdateSheet {
                    sheet,
                    rotation: Some(Rotation::Deg180),
                    unit: None,
                    scale: None,
                })
                .unwrap();
            },
            strategy: S::SheetZone,
            // Shown top left is zone B2 (g a h as shown), then B1, A2 and A1 (e c j b).
            expected: "gahfdecjbi",
        },
        Case {
            name: "per view, outside views last",
            setup: |_| {},
            strategy: S::View,
            expected: "bjcehagdfi",
        },
        Case {
            name: "clockwise per view from 12 o'clock",
            setup: |_| {},
            strategy: S::ViewClockwise,
            expected: "jcebhgadfi",
        },
        Case {
            name: "a point in two views belongs to the smaller one",
            setup: |s| {
                let sheet = s.sheet(0);
                let mut all = views();
                // A big view around everything, drawn first, and a detail view around j and c.
                all.insert(
                    0,
                    SheetView {
                        label: "all".into(),
                        rect: rect(0.0, 0.0, 1190.0, 842.0),
                    },
                );
                all.push(SheetView {
                    label: "detail".into(),
                    rect: rect(240.0, 90.0, 80.0, 40.0),
                });
                s.run(Command::SetViews { sheet, views: all }).unwrap();
            },
            strategy: S::View,
            // all: d f; V1: b e; V2: h a g; detail: j c; sheet 1: i.
            expected: "dfbehagjci",
        },
        Case {
            name: "by kind, each kind in sheet and zone order",
            setup: |_| {},
            strategy: S::Kind,
            expected: "bdgichaejf",
        },
        Case {
            name: "manual keeps the current order",
            setup: |_| {},
            strategy: S::Manual,
            expected: "abcdefghij",
        },
    ];
    for case in cases {
        let mut synth = Synth::new();
        (case.setup)(&mut synth);
        assert_eq!(synth.order(case.strategy), case.expected, "{}", case.name);
        // Applying gives the same order with numbers 1..n (FR-BAL-05: preview equals apply).
        let preview = preview(synth.doc.project(), case.strategy).unwrap();
        synth
            .run(Command::ApplyNumbering {
                strategy: case.strategy,
            })
            .unwrap();
        let applied: Vec<(CharId, String)> = synth
            .doc
            .project()
            .characteristics
            .iter()
            .map(|c| (c.id, c.number.to_string()))
            .collect();
        let previewed: Vec<(CharId, String)> = preview
            .entries
            .iter()
            .map(|e| (e.id, e.number.to_string()))
            .collect();
        assert_eq!(applied, previewed, "{}", case.name);
        let expected: Vec<String> = (1..=10).map(|n: u32| n.to_string()).collect();
        let numbers: Vec<String> = applied.into_iter().map(|(_, n)| n).collect();
        assert_eq!(numbers, expected, "{}", case.name);
        assert_eq!(
            synth.doc.project().settings.numbering.strategy,
            case.strategy,
            "{}",
            case.name
        );
        common::check_invariants(synth.doc.project());
    }
}

#[test]
fn reading_rows_use_one_balloon_diameter_as_tolerance() {
    // b (y 100), c (y 105) and j (y 112) are one row with the 7 mm default balloon (19.8 units).
    // With a 2 mm balloon (5.7 units) c joins b's row but j starts its own.
    let mut synth = Synth::new();
    let sheet = synth.sheet(0);
    synth
        .run(Command::SetZoneGrid { sheet, grid: None })
        .unwrap();
    assert_eq!(synth.order(S::SheetZone), "bjcdehfagi");
    let mut style = synth.doc.project().settings.balloon_style.clone();
    style.size_mm = 2.0;
    synth
        .run(Command::SetDefaultBalloonStyle { style })
        .unwrap();
    assert_eq!(synth.order(S::SheetZone), "bcdjehfagi");
}

#[test]
fn apply_is_one_undo_step_and_reports_the_changed_numbers() {
    let mut synth = Synth::new();
    let before = synth.doc.project().clone();
    let preview = preview(synth.doc.project(), S::ViewClockwise).unwrap();
    // Every number changes, even `i` on sheet 1 moves from 9 to 10.
    assert_eq!(preview.changed, 10);
    assert_eq!(preview.strategy, S::ViewClockwise);
    synth
        .run(Command::ApplyNumbering {
            strategy: S::ViewClockwise,
        })
        .unwrap();
    assert_eq!(synth.numbers(), "j:1 c:2 e:3 b:4 h:5 g:6 a:7 d:8 f:9 i:10");
    synth.doc.undo(&mut synth.env).unwrap();
    assert_eq!(*synth.doc.project(), before);
    // Applying the order the project already has changes nothing and adds no undo step.
    synth
        .run(Command::ApplyNumbering {
            strategy: S::Manual,
        })
        .unwrap();
    synth
        .run(Command::ApplyNumbering {
            strategy: S::Manual,
        })
        .unwrap();
    assert!(!synth.doc.can_redo());
    assert_eq!(preview_of(&synth, S::Manual).changed, 0);
}

fn preview_of(synth: &Synth, strategy: NumberingStrategy) -> dimo_core::NumberingPreview {
    preview(synth.doc.project(), strategy).unwrap()
}

#[test]
fn locked_numbering_refuses_renumbering_and_adds_by_the_insert_policy() {
    // D-23, FR-BAL-11: a locked project keeps its numbers; added characteristics follow the
    // insert policy, not the strategy.
    let mut synth = Synth::new();
    synth
        .run(Command::SetNumberingSettings {
            settings: NumberingSettings {
                insert_when_locked: InsertPolicy::SubNumber,
                ..synth.doc.project().settings.numbering
            },
        })
        .unwrap();
    synth
        .run(Command::LockNumbering {
            reason: LockReason::IssuedReport,
        })
        .unwrap();
    let before = synth.doc.project().clone();
    assert_eq!(
        synth.run(Command::ApplyNumbering {
            strategy: S::SheetZone
        }),
        Err(CommandError::NumberingLocked)
    );
    assert_eq!(
        preview(synth.doc.project(), S::SheetZone),
        Err(CommandError::NumberingLocked)
    );
    assert_eq!(*synth.doc.project(), before);
    let after = synth.doc.project().characteristics[1].id;
    let sheet = synth.sheet(0);
    synth
        .run(Command::AddCharacteristic {
            sheet,
            position: Point { x: 50.0, y: 50.0 },
            anchor: Point { x: 60.0, y: 60.0 },
            region: None,
            values: vec![],
            insert_after: Some(after),
        })
        .unwrap();
    let numbers: Vec<String> = synth
        .doc
        .project()
        .characteristics
        .iter()
        .map(|c| c.number.to_string())
        .collect();
    assert_eq!(
        numbers,
        ["1", "2", "2.1", "3", "4", "5", "6", "7", "8", "9", "10"]
    );
    common::check_invariants(synth.doc.project());
}

/// A proposal for `text` with `quantity` features, read at `at` on sheet 0.
fn multi(synth: &Synth, text: &str, quantity: u32, at: Point) -> Proposal {
    Proposal {
        kind: K::Diameter,
        requirement_text: text.into(),
        nominal: None,
        unit: None,
        upper_dev: None,
        lower_dev: None,
        upper_limit: None,
        lower_limit: None,
        fit: None,
        derivation: None,
        quantity,
        inspect: true,
        source: SourceRegion {
            sheet: synth.sheet(0),
            region: OrientedBox {
                center: at,
                size: Size {
                    width: 30.0,
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
                x: at.x + 30.0,
                y: at.y - 20.0,
            },
            anchor: at,
        },
        parse_error: None,
        parse_hints: Vec::new(),
        job_id: None,
        engines: Vec::new(),
    }
}

#[test]
fn multi_instance_callouts_get_sub_numbers_when_set() {
    // D-22, FR-BAL-07: with sub-numbers, `3X` becomes three characteristics 11.1 to 11.3 that
    // every strategy keeps together.
    let mut synth = Synth::new();
    let settings = synth.doc.project().settings.numbering;
    synth
        .run(Command::SetNumberingSettings {
            settings: NumberingSettings {
                multi_instance: MultiInstance::SubNumber,
                ..settings
            },
        })
        .unwrap();
    let proposal = multi(&synth, "3X Ø5", 3, Point { x: 150.0, y: 200.0 });
    synth
        .run(Command::AcceptProposals {
            proposals: vec![proposal],
            insert_after: None,
        })
        .unwrap();
    let project = synth.doc.project();
    let added: Vec<_> = project.characteristics[10..].iter().collect();
    assert_eq!(added.len(), 3);
    assert!(added.iter().all(|c| c.quantity == 1));
    let texts: Vec<String> = added.iter().map(|c| c.number.to_string()).collect();
    assert_eq!(texts, ["11.1", "11.2", "11.3"]);
    // Balloons stacked downwards, anchors on the callout.
    let ys: Vec<f64> = added
        .iter()
        .map(|c| project.balloons_of(c.id).next().unwrap().position.y)
        .collect();
    assert!(ys.windows(2).all(|w| w[1] > w[0]), "{ys:?}");
    common::check_invariants(project);
    for (id, label) in added.iter().map(|c| c.id).zip(['x', 'y', 'z']) {
        synth.labels.insert(id, label);
    }
    // The group sits in zone A1, after j and c (rows at y 100 to 112), before e (y 300).
    synth
        .run(Command::ApplyNumbering {
            strategy: S::SheetZone,
        })
        .unwrap();
    assert_eq!(
        synth.numbers(),
        "b:1 j:2 c:3 x:4.1 y:4.2 z:4.3 e:5 d:6 f:7 h:8 a:9 g:10 i:11"
    );
    common::check_invariants(synth.doc.project());
    // Clockwise in V1: the group's anchor (150,200) is at about 270 degrees, before b.
    synth
        .run(Command::ApplyNumbering {
            strategy: S::ViewClockwise,
        })
        .unwrap();
    assert_eq!(
        synth.numbers(),
        "j:1 c:2 e:3 x:4.1 y:4.2 z:4.3 b:5 h:6 g:7 a:8 d:9 f:10 i:11"
    );
    // Back to one balloon with a quantity: the group members get plain numbers again.
    synth
        .run(Command::SetNumberingSettings {
            settings: NumberingSettings {
                multi_instance: MultiInstance::Quantity,
                ..settings
            },
        })
        .unwrap();
    assert_eq!(
        synth.numbers(),
        "j:1 c:2 e:3 x:4 y:5 z:6 b:7 h:8 g:9 a:10 d:11 f:12 i:13"
    );
    common::check_invariants(synth.doc.project());
}

#[test]
fn without_sub_numbers_a_multi_instance_callout_stays_one_characteristic() {
    // D-22 default: one balloon with the quantity.
    let mut synth = Synth::new();
    let proposal = multi(&synth, "4X Ø5", 4, Point { x: 150.0, y: 200.0 });
    synth
        .run(Command::AcceptProposals {
            proposals: vec![proposal],
            insert_after: None,
        })
        .unwrap();
    let project = synth.doc.project();
    assert_eq!(project.characteristics.len(), 11);
    assert_eq!(project.characteristics[10].quantity, 4);
    assert_eq!(project.characteristics[10].number.to_string(), "11");
}

#[test]
fn characteristics_without_a_point_go_last_in_their_order() {
    // A characteristic without balloon and source region has no point (only possible in a
    // hand edited project); it goes last.
    let synth = Synth::new();
    let mut project = synth.doc.project().clone();
    let a = project.characteristics[0].id;
    project.balloons.retain(|b| b.characteristic != a);
    let order: String = strategy_order(&project, S::SheetZone)
        .into_iter()
        .map(|id| synth.label(id))
        .collect();
    assert_eq!(order, "bjcedfhgia");
}

#[test]
fn different_callouts_from_one_box_are_not_a_group() {
    // A diameter and its depth read from one box share the region but are no repeated feature
    // (D-22): with sub-numbers on they keep plain numbers.
    let mut synth = Synth::new();
    let settings = synth.doc.project().settings.numbering;
    synth
        .run(Command::SetNumberingSettings {
            settings: NumberingSettings {
                multi_instance: MultiInstance::SubNumber,
                ..settings
            },
        })
        .unwrap();
    let at = Point { x: 150.0, y: 200.0 };
    let diameter = multi(&synth, "Ø5 ↧8", 1, at);
    let mut depth = diameter.clone();
    depth.kind = K::Depth;
    synth
        .run(Command::AcceptProposals {
            proposals: vec![diameter, depth],
            insert_after: None,
        })
        .unwrap();
    synth
        .run(Command::ApplyNumbering {
            strategy: S::SheetZone,
        })
        .unwrap();
    let numbers: Vec<String> = synth
        .doc
        .project()
        .characteristics
        .iter()
        .map(|c| c.number.to_string())
        .collect();
    assert_eq!(
        numbers,
        (1..=12).map(|n: u32| n.to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn no_split_while_locked_or_above_one_hundred_features() {
    // D-23: locked numbers follow the insert policy only, so a repeated callout stays one
    // characteristic with its quantity; so does one with more than 100 features.
    for (locked, quantity) in [(true, 4), (false, 101)] {
        let mut synth = Synth::new();
        let settings = synth.doc.project().settings.numbering;
        synth
            .run(Command::SetNumberingSettings {
                settings: NumberingSettings {
                    multi_instance: MultiInstance::SubNumber,
                    ..settings
                },
            })
            .unwrap();
        if locked {
            synth
                .run(Command::LockNumbering {
                    reason: LockReason::Manual,
                })
                .unwrap();
        }
        let proposal = multi(&synth, "nX Ø5", quantity, Point { x: 150.0, y: 200.0 });
        synth
            .run(Command::AcceptProposals {
                proposals: vec![proposal],
                insert_after: None,
            })
            .unwrap();
        let project = synth.doc.project();
        assert_eq!(project.characteristics.len(), 11, "locked {locked}");
        let added = &project.characteristics[10];
        assert_eq!(added.quantity, quantity);
        assert_eq!(added.number.to_string(), "11");
        common::check_invariants(project);
    }
}

#[test]
fn reading_order_on_a_sheet_turned_270_degrees() {
    let mut synth = Synth::new();
    let sheet = synth.sheet(0);
    synth
        .run(Command::SetZoneGrid { sheet, grid: None })
        .unwrap();
    synth
        .run(Command::UpdateSheet {
            sheet,
            rotation: Some(Rotation::Deg270),
            unit: None,
            scale: None,
        })
        .unwrap();
    assert_eq!(synth.order(S::SheetZone), "ghadecjfbi");
}
