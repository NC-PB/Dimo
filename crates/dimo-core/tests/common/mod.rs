//! Fixtures shared by the command engine tests.
#![allow(dead_code, reason = "each test crate uses a different subset")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use dimo_core::{
    CharId, Command, DrawingRevision, Environment, FieldValue, FixedEnvironment, Point, Project,
    ProjectInfo, RevisionId, Sha256Hex, Sheet, SheetId, SheetKind, Size, Timestamp,
};

/// A project with one revision of two sheets (A3 landscape, A4 portrait), IDs from `env`.
pub fn project(env: &mut FixedEnvironment) -> Project {
    let a3 = Size {
        width: 1190.0,
        height: 842.0,
    };
    let a4 = Size {
        width: 595.0,
        height: 842.0,
    };
    let sheets = vec![
        Sheet::new(
            SheetId::from_uuid(env.new_uuid()),
            0,
            a3,
            SheetKind::VectorText,
        ),
        Sheet::new(SheetId::from_uuid(env.new_uuid()), 1, a4, SheetKind::Raster),
    ];
    let revision = DrawingRevision {
        id: RevisionId::from_uuid(env.new_uuid()),
        label: "A".into(),
        file_name: "part.pdf".into(),
        sha256: Sha256Hex::from_bytes(&[7; 32]),
        imported_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
        sheets,
    };
    Project::new(
        ProjectInfo {
            part_number: "P-100".into(),
            ..ProjectInfo::default()
        },
        revision,
    )
}

/// Sheet `index` of the current revision.
pub fn sheet(project: &Project, index: usize) -> SheetId {
    project.revisions[0].sheets[index].id
}

/// Adds a characteristic on sheet 0 at `(x, y)` with the anchor 20 units to the left.
pub fn add_at(project: &Project, x: f64, y: f64, values: Vec<FieldValue>) -> Command {
    Command::AddCharacteristic {
        sheet: sheet(project, 0),
        position: Point { x, y },
        anchor: Point { x: x - 20.0, y },
        region: None,
        values,
    }
}

/// Characteristic IDs in placement order.
pub fn order(project: &Project) -> Vec<CharId> {
    project.characteristics.iter().map(|c| c.id).collect()
}

/// Display numbers in placement order.
pub fn numbers(project: &Project) -> Vec<u32> {
    project.characteristics.iter().map(|c| c.number).collect()
}

/// Structural invariants that hold after every command, undo and redo.
pub fn check_invariants(project: &Project) {
    let numbers = numbers(project);
    if project.is_numbering_locked() {
        let mut sorted = numbers.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), numbers.len(), "duplicate numbers {numbers:?}");
        let highest = project
            .numbering
            .lock
            .as_ref()
            .map_or(0, |l| l.highest_number);
        assert!(numbers.iter().all(|n| *n >= 1 && *n <= highest));
    } else {
        let expected: Vec<u32> = (1..).take(numbers.len()).collect();
        assert_eq!(numbers, expected, "unlocked numbers follow placement order");
    }
    for c in &project.characteristics {
        assert_eq!(project.balloons_of(c.id).count(), 1, "one balloon per char");
    }
    for b in &project.balloons {
        assert!(project.characteristic(b.characteristic).is_some());
        assert!(project.sheet(b.sheet).is_some());
    }
}

/// A deterministic environment, after the fixture consumed its IDs.
pub fn env() -> FixedEnvironment {
    let mut env = FixedEnvironment::new();
    env.set_user("inspector");
    env
}
