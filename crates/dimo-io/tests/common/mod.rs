//! Sample projects shared by the project file tests.
#![allow(dead_code, reason = "each test crate uses a different subset")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::path::PathBuf;

use dimo_core::characteristic::FieldValue;
use dimo_core::decimal::parse_decimal;
use dimo_core::{
    BalloonMove, BalloonStyleOverride, CharacteristicKind, Command, Document, FixedEnvironment,
    LockReason, OrientedBox, Point, Project, ProjectInfo, SheetKind, Size, Timestamp, Unit,
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

pub fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("../..")
}

pub fn crate_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
}

/// A project file after [`edit`], with its audit log and drawing.
pub fn sample() -> ProjectFile {
    let mut env = env();
    let drawing = drawing(&mut env);
    let mut document = Document::new(Project::new(info(), drawing.revision.clone()));
    edit(&mut document, &mut env);
    let mut file = ProjectFile::new(document.project().clone(), drawing.revision.imported_at);
    file.audit = document.take_audit();
    file.insert_drawing(drawing.bytes);
    file
}
