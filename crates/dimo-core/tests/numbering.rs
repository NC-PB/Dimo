//! Numbering rules as a table (D-21 placement order, D-23 locked numbering, FR-BAL-10).
//!
//! Each case runs steps on a fresh project and lists the expected characteristics in placement
//! order as `(label, number)`, where the label is the letter of the add step that created it
//! (`a` for the first add, `b` for the second, ...).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

mod common;

use std::collections::BTreeMap;

use dimo_core::{CharId, Command, CommandError, Document, LockReason};

#[derive(Debug, Clone, Copy)]
enum Step {
    /// Add one characteristic.
    Add,
    /// Delete characteristics at these positions of the current order.
    Delete(&'static [usize]),
    /// Move characteristics at these positions before the one at the given position
    /// (`None`: to the end).
    Move(&'static [usize], Option<usize>),
    Lock,
    Unlock,
    Undo,
    Redo,
}

use Step::{Add, Delete, Lock, Move, Redo, Undo, Unlock};

struct Case {
    name: &'static str,
    steps: &'static [Step],
    expected: &'static [(char, u32)],
    /// Error expected from the last step.
    error: Option<CommandError>,
}

fn run(case: &Case) {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    let mut labels: BTreeMap<CharId, char> = BTreeMap::new();
    let mut last = Ok(());
    for step in case.steps {
        let ids = common::order(doc.project());
        let pick = |positions: &[usize]| positions.iter().map(|p| ids[*p]).collect::<Vec<_>>();
        let command = match *step {
            Add => {
                let n = ids.len();
                Some(common::add_at(
                    doc.project(),
                    100.0 + 10.0 * f64::from(u32::try_from(n).unwrap()),
                    100.0,
                    vec![],
                ))
            }
            Delete(positions) => Some(Command::DeleteCharacteristics {
                ids: pick(positions),
            }),
            Move(positions, before) => Some(Command::MoveCharacteristics {
                ids: pick(positions),
                before: before.map(|p| ids[p]),
            }),
            Lock => Some(Command::LockNumbering {
                reason: LockReason::IssuedReport,
            }),
            Unlock => Some(Command::UnlockNumbering),
            Undo => {
                last = doc.undo(&mut env).map(|_| ());
                None
            }
            Redo => {
                last = doc.redo(&mut env).map(|_| ());
                None
            }
        };
        if let Some(command) = command {
            last = doc.execute(command, &mut env).map(|_| ());
        }
        for c in &doc.project().characteristics {
            let next = char::from(b'a' + u8::try_from(labels.len()).unwrap());
            labels.entry(c.id).or_insert(next);
        }
        common::check_invariants(doc.project());
    }
    let actual: Vec<(char, u32)> = doc
        .project()
        .characteristics
        .iter()
        .map(|c| (labels[&c.id], c.number))
        .collect();
    assert_eq!(actual, case.expected, "{}", case.name);
    assert_eq!(last.err(), case.error, "{}", case.name);
}

#[test]
#[allow(clippy::too_many_lines, reason = "one table of cases")]
fn numbering_rules() {
    let cases = [
        Case {
            name: "numbers follow placement order",
            steps: &[Add, Add, Add],
            expected: &[('a', 1), ('b', 2), ('c', 3)],
            error: None,
        },
        Case {
            name: "unlocked delete renumbers",
            steps: &[Add, Add, Add, Add, Delete(&[1])],
            expected: &[('a', 1), ('c', 2), ('d', 3)],
            error: None,
        },
        Case {
            name: "unlocked group delete renumbers",
            steps: &[Add, Add, Add, Add, Delete(&[0, 2])],
            expected: &[('b', 1), ('d', 2)],
            error: None,
        },
        Case {
            name: "move last to the front renumbers",
            steps: &[Add, Add, Add, Move(&[2], Some(0))],
            expected: &[('c', 1), ('a', 2), ('b', 3)],
            error: None,
        },
        Case {
            name: "moving several keeps their relative order",
            steps: &[Add, Add, Add, Add, Move(&[2, 0], None)],
            expected: &[('b', 1), ('d', 2), ('a', 3), ('c', 4)],
            error: None,
        },
        Case {
            name: "move to the current place changes nothing",
            steps: &[Add, Add, Move(&[0], Some(1))],
            expected: &[('a', 1), ('b', 2)],
            error: None,
        },
        Case {
            name: "move before itself is refused",
            steps: &[Add, Add, Move(&[0, 1], Some(1))],
            expected: &[('a', 1), ('b', 2)],
            error: Some(CommandError::InvalidMoveTarget),
        },
        Case {
            name: "locked delete leaves a gap",
            steps: &[Add, Add, Add, Lock, Delete(&[1])],
            expected: &[('a', 1), ('c', 3)],
            error: None,
        },
        Case {
            name: "locked add takes the next free number",
            steps: &[Add, Add, Add, Lock, Add],
            expected: &[('a', 1), ('b', 2), ('c', 3), ('d', 4)],
            error: None,
        },
        Case {
            name: "locked add does not fill a gap",
            steps: &[Add, Add, Add, Lock, Delete(&[1]), Add],
            expected: &[('a', 1), ('c', 3), ('d', 4)],
            error: None,
        },
        Case {
            name: "locked add never reuses a deleted highest number",
            steps: &[Add, Add, Add, Lock, Delete(&[2]), Add],
            expected: &[('a', 1), ('b', 2), ('d', 4)],
            error: None,
        },
        Case {
            name: "locked move is refused",
            steps: &[Add, Add, Lock, Move(&[1], Some(0))],
            expected: &[('a', 1), ('b', 2)],
            error: Some(CommandError::NumberingLocked),
        },
        Case {
            name: "lock on an empty project, then add",
            steps: &[Lock, Add, Add],
            expected: &[('a', 1), ('b', 2)],
            error: None,
        },
        Case {
            name: "unlock renumbers by placement order",
            steps: &[Add, Add, Add, Lock, Delete(&[0]), Add, Unlock],
            expected: &[('b', 1), ('c', 2), ('d', 3)],
            error: None,
        },
        Case {
            name: "undo of a locked add gives the number back",
            steps: &[Add, Add, Lock, Add, Undo, Add],
            expected: &[('a', 1), ('b', 2), ('d', 3)],
            error: None,
        },
        Case {
            name: "undo of unlock restores the gaps and the lock",
            steps: &[Add, Add, Add, Lock, Delete(&[1]), Unlock, Undo],
            expected: &[('a', 1), ('c', 3)],
            error: None,
        },
        Case {
            name: "redo of a move renumbers again",
            steps: &[Add, Add, Move(&[1], Some(0)), Undo, Redo],
            expected: &[('b', 1), ('a', 2)],
            error: None,
        },
        Case {
            name: "lock twice and unlock twice change nothing",
            steps: &[Add, Lock, Lock, Unlock, Unlock],
            expected: &[('a', 1)],
            error: None,
        },
    ];
    for case in &cases {
        run(case);
    }
}

#[test]
fn lock_records_reason_time_user_and_highest_number() {
    let mut env = common::env();
    let mut doc = Document::new(common::project(&mut env));
    for _ in 0..3 {
        doc.execute(common::add_at(doc.project(), 50.0, 50.0, vec![]), &mut env)
            .unwrap();
    }
    env.set_time(dimo_core::Timestamp::parse("2026-03-04T05:06:07Z").unwrap());
    doc.execute(
        Command::LockNumbering {
            reason: LockReason::Manual,
        },
        &mut env,
    )
    .unwrap();
    let lock = doc.project().numbering.lock.clone().unwrap();
    assert_eq!(lock.reason, LockReason::Manual);
    assert_eq!(lock.locked_at.as_str(), "2026-03-04T05:06:07Z");
    assert_eq!(lock.locked_by, "inspector");
    assert_eq!(lock.highest_number, 3);
}
