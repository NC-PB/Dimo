//! Sample projects shared by the project file tests.
#![allow(dead_code, reason = "each test crate uses a different subset")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::path::PathBuf;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    BalloonMove, BalloonPlacement, BalloonStyleOverride, CharacteristicKind, Command, CustomTable,
    DecimalPlaceRule, DerivationHint, DerivationRule, Document, FixedEnvironment, InsertPolicy,
    LockReason, MultiInstance, NumberingSettings, NumberingStrategy, OrientedBox, Origin,
    ParseHint, Point, Project, ProjectInfo, Proposal, RangeBound, Rect, SheetKind, SheetView, Size,
    SizeRange, SourceRegion, TableClass, TableLookup, TableRef, TextSource, Timestamp,
    ToleranceDerivation, ToleranceSettings, Unit, UnitRounding, ZoneGrid,
};
use dimo_io::project::{ImportedDrawing, ProjectFile, SheetInfo, import_drawing, sha256};

/// Stand-in drawing file. `dimo-io` never parses it, it only stores and hashes it.
pub const DRAWING: &[u8] = b"%PDF-1.7\n% dimo-io test drawing, not a renderable PDF\n%%EOF\n";

pub fn env() -> FixedEnvironment {
    let mut env = FixedEnvironment::new();
    env.set_user("tester");
    env
}

pub fn at(env: &mut FixedEnvironment, time: &str) {
    env.set_time(Timestamp::parse(time).unwrap());
}

/// A drawing with an A3 and an A4 sheet, imported at 2026-03-01T08:00:00Z.
pub fn drawing(env: &mut FixedEnvironment) -> ImportedDrawing {
    at(env, "2026-03-01T08:00:00Z");
    let sheets = [
        SheetInfo {
            size: Size {
                width: 1190.55,
                height: 841.89,
            },
            kind: SheetKind::VectorText,
        },
        SheetInfo {
            size: Size {
                width: 595.276,
                height: 841.89,
            },
            kind: SheetKind::Raster,
        },
    ];
    import_drawing(
        DRAWING.to_vec(),
        "part-100.pdf",
        &sha256(DRAWING),
        &sheets,
        env,
    )
    .unwrap()
}

pub fn info() -> ProjectInfo {
    ProjectInfo {
        part_number: "P-100".into(),
        part_name: "Bracket".into(),
        customer: "Example customer".into(),
        order_reference: "PO 4711".into(),
    }
}

fn dec(text: &str) -> FieldValue {
    FieldValue::Nominal(parse_decimal(text).into())
}

/// Commands that touch every kind of change: adds with decimals and regions, field updates,
/// balloon moves and styles, deletion and a numbering lock.
#[allow(clippy::too_many_lines, reason = "one linear script of test commands")]
pub fn edit(document: &mut Document, env: &mut FixedEnvironment) {
    let sheet = document.project().revisions[0].sheets[0].id;
    let add = |x: f64, y: f64, values: Vec<FieldValue>| Command::AddCharacteristic {
        sheet,
        position: Point { x, y },
        anchor: Point {
            x: x - 21.3,
            y: y + 0.1,
        },
        region: Some(OrientedBox {
            center: Point { x: x - 40.0, y },
            size: Size {
                width: 33.3,
                height: 7.1,
            },
            angle: 0.1,
        }),
        values,
        insert_after: None,
    };
    at(env, "2026-03-01T08:05:00.120Z");
    document
        .execute(
            add(
                100.25,
                200.5,
                vec![
                    FieldValue::Kind(CharacteristicKind::Diameter),
                    FieldValue::RequirementText("Ø8 ±0.1".into()),
                    dec("8.000"),
                    FieldValue::UpperDev(parse_decimal("0.1").into()),
                    FieldValue::LowerDev(parse_decimal("-0.1").into()),
                ],
            ),
            env,
        )
        .unwrap();
    at(env, "2026-03-01T08:06:10.000Z");
    document
        .execute(
            add(
                300.0,
                1.0 / 3.0,
                vec![
                    FieldValue::Kind(CharacteristicKind::Linear),
                    dec("30.0203"),
                    FieldValue::Unit(Some(Unit::Mm)),
                ],
            ),
            env,
        )
        .unwrap();
    document
        .execute(add(450.0, 600.0, vec![dec("12.5")]), env)
        .unwrap();
    let ids: Vec<_> = document
        .project()
        .characteristics
        .iter()
        .map(|c| c.id)
        .collect();
    let balloons: Vec<_> = document.project().balloons.iter().map(|b| b.id).collect();
    at(env, "2026-03-01T08:07:00.000Z");
    document
        .execute(
            Command::UpdateFields {
                ids: vec![ids[1]],
                values: vec![FieldValue::Comment("check twice".into())],
            },
            env,
        )
        .unwrap();
    document
        .execute(
            Command::MoveBalloons {
                moves: vec![BalloonMove {
                    id: balloons[0],
                    position: Point {
                        x: 0.1 + 0.2,
                        y: 1e-7,
                    },
                    anchor: None,
                }],
            },
            env,
        )
        .unwrap();
    document
        .execute(
            Command::RestyleBalloons {
                ids: vec![balloons[2]],
                style: BalloonStyleOverride {
                    size_mm: Some(7.5),
                    ..BalloonStyleOverride::default()
                },
            },
            env,
        )
        .unwrap();
    document
        .execute(Command::DeleteCharacteristics { ids: vec![ids[2]] }, env)
        .unwrap();
    at(env, "2026-03-01T08:09:59.999Z");
    document
        .execute(
            Command::LockNumbering {
                reason: LockReason::Manual,
            },
            env,
        )
        .unwrap();
}

/// A custom tolerance table stored in the sample project (M2 decision 4). `dimo-io` stores
/// the bytes as they are; this text follows `data/tolerances/README.md`.
pub const TABLE: &[u8] = br#"[table]
id = "shop-table"
title = "Shop general tolerances"
kind = "custom"
unit = "mm"
version = 1
status = "draft"
source = "Example shop standard"

[[part]]
id = "linear"
kind = "symmetric"
applies_to = "linear"
value_unit = "mm"
columns = ["a"]
rows = [
  { min_inclusive = "0.5", max_inclusive = "30", values = ["0.1"] },
  { min_exclusive = "30", values = ["0.2"] },
]
"#;

fn table_ref(id: &str) -> TableRef {
    TableRef {
        id: id.into(),
        version: 1,
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        origin: Point { x, y },
        size: Size { width, height },
    }
}

/// Commands of data model version 2, after [`edit`] at the same time: tolerance and numbering
/// settings with a custom table, zone grid and views, an accepted proposal inserted with a
/// letter suffix while locked, and a rule derivation (T2.4).
#[allow(clippy::too_many_lines, reason = "one linear script of test commands")]
pub fn edit_v2(document: &mut Document, env: &mut FixedEnvironment) {
    let sheet = document.project().revisions[0].sheets[0].id;
    let first = document.project().characteristics[0].id;
    let second = document.project().characteristics[1].id;
    let commands = vec![
        Command::SetToleranceSettings {
            settings: ToleranceSettings {
                general: Some(TableClass {
                    table: table_ref("iso-2768-1"),
                    class: "m".into(),
                }),
                drawing_rule: Some(TableClass {
                    table: table_ref("shop-table"),
                    class: "a".into(),
                }),
                decimal_rules: vec![
                    DecimalPlaceRule {
                        places: 1,
                        tolerance: parse_decimal("0.1").unwrap(),
                    },
                    DecimalPlaceRule {
                        places: 2,
                        tolerance: parse_decimal("0.05").unwrap(),
                    },
                ],
                unit_rounding: UnitRounding::default(),
                custom_tables: vec![CustomTable {
                    table: table_ref("shop-table"),
                    sha256: sha256(TABLE),
                }],
            },
        },
        Command::SetZoneGrid {
            sheet,
            grid: Some(ZoneGrid {
                frame: rect(20.0, 20.0, 1150.55, 801.89),
                column_labels: (1..=8).map(|i| i.to_string()).collect(),
                row_labels: ["A", "B", "C", "D", "E", "F"].map(String::from).to_vec(),
            }),
        },
        Command::SetViews {
            sheet,
            views: vec![
                SheetView {
                    label: "A-A".into(),
                    rect: rect(40.0, 40.0, 400.0, 300.25),
                },
                SheetView {
                    label: String::new(),
                    rect: rect(500.0, 40.0, 1.0 / 3.0, 300.0),
                },
            ],
        },
        Command::SetNumberingSettings {
            settings: NumberingSettings {
                strategy: NumberingStrategy::SheetZone,
                multi_instance: MultiInstance::SubNumber,
                insert_when_locked: InsertPolicy::LetterSuffix,
            },
        },
        Command::AcceptProposals {
            proposals: vec![Proposal {
                kind: CharacteristicKind::Diameter,
                requirement_text: "Ø30 H7 +0.0203 0".into(),
                nominal: parse_decimal("30"),
                unit: Some(Unit::Mm),
                upper_dev: parse_decimal("0.0203"),
                lower_dev: parse_decimal("0"),
                upper_limit: parse_decimal("30.0203"),
                lower_limit: parse_decimal("30"),
                fit: Some("H7".into()),
                derivation: Some(ToleranceDerivation {
                    rule: DerivationRule::Explicit,
                    draft: false,
                    hints: vec![DerivationHint::FitDeviationsDiffer {
                        fit: "H7".into(),
                        table_upper_dev: parse_decimal("0.021"),
                        table_lower_dev: parse_decimal("0"),
                    }],
                    conversion: None,
                }),
                quantity: 2,
                inspect: true,
                source: SourceRegion {
                    sheet,
                    region: OrientedBox {
                        center: Point { x: 700.5, y: 400.0 },
                        size: Size {
                            width: 60.0,
                            height: 9.5,
                        },
                        angle: 90.0,
                    },
                    text_source: TextSource::PdfText,
                    raw_text: Some("Ø30 H7 +0.0203 0".into()),
                },
                origin: Origin::BoxSelect,
                placement: BalloonPlacement {
                    position: Point { x: 720.0, y: 380.0 },
                    anchor: Point { x: 700.5, y: 400.0 },
                },
                parse_error: None,
                parse_hints: vec![ParseHint::StackedLinesJoined],
                job_id: None,
                engines: Vec::new(),
            }],
            insert_after: Some(first),
        },
        Command::UpdateFields {
            ids: vec![second],
            values: vec![
                FieldValue::UpperLimit(parse_decimal("30.3203").into()),
                FieldValue::LowerLimit(parse_decimal("29.7203").into()),
                FieldValue::Derivation(Some(ToleranceDerivation {
                    rule: DerivationRule::General {
                        lookup: TableLookup {
                            table: table_ref("iso-2768-1"),
                            part: "linear".into(),
                            class: "m".into(),
                            range: SizeRange {
                                min: Some(RangeBound {
                                    value: parse_decimal("6").unwrap(),
                                    inclusive: false,
                                }),
                                max: Some(RangeBound {
                                    value: parse_decimal("30").unwrap(),
                                    inclusive: true,
                                }),
                            },
                        },
                    },
                    draft: true,
                    hints: Vec::new(),
                    conversion: None,
                })),
            ],
        },
    ];
    for command in commands {
        document.execute(command, env).unwrap();
    }
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
}

pub fn crate_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
}

/// A project file after [`edit`] and [`edit_v2`], with its audit log, drawing and custom table.
pub fn sample() -> ProjectFile {
    let mut env = env();
    let drawing = drawing(&mut env);
    let mut document = Document::new(Project::new(info(), drawing.revision.clone()));
    edit(&mut document, &mut env);
    edit_v2(&mut document, &mut env);
    let mut file = ProjectFile::new(document.project().clone(), drawing.revision.imported_at);
    file.audit = document.take_audit();
    file.insert_drawing(drawing.bytes);
    file.insert_table(TABLE.to_vec());
    file
}
