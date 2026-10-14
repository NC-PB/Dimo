//! Numbering: insert slots, renumbering, strategies, lock and unlock (D-21, D-22, D-23,
//! FR-BAL-04, FR-BAL-10, FR-BAL-11).

use super::settings::change_settings;
use super::{CommandError, Tx};
use crate::env::Environment;
use crate::id::CharId;
use crate::number::DisplayNumber;
use crate::numbering::{numbers_for_order, strategy_order};
use crate::patch::Change;
use crate::project::{
    InsertPolicy, LockReason, Numbering, NumberingLock, NumberingSettings, NumberingStrategy,
};

/// Sets the numbering settings; while unlocked, renumbers (D-22).
pub(super) fn set_settings(
    tx: &mut Tx<'_>,
    settings: NumberingSettings,
) -> Result<(), CommandError> {
    change_settings(tx, |s| s.numbering = settings)?;
    renumber_if_unlocked(tx)
}

/// Unlocks numbering and renumbers by placement order. No change if not locked.
pub(super) fn unlock(tx: &mut Tx<'_>) -> Result<(), CommandError> {
    if tx.project.numbering.lock.is_some() {
        tx.push(Change::NumberingChanged {
            before: tx.project.numbering.clone(),
            after: Numbering::default(),
        })?;
        renumber_if_unlocked(tx)?;
    }
    Ok(())
}

/// Number and placement index for a characteristic added now (D-21, D-23, FR-BAL-11).
///
/// Unlocked: the next number at the end of the placement order. Locked: the number of the
/// insert policy, placed in number order; the lock remembers it, so it is never given again.
pub(super) fn insert_slot(
    tx: &mut Tx<'_>,
    after: Option<CharId>,
) -> Result<(DisplayNumber, usize), CommandError> {
    let project = &*tx.project;
    let len = project.characteristics.len();
    let anchor = match after {
        Some(id) => Some(
            project
                .characteristic(id)
                .ok_or(CommandError::UnknownCharacteristic(id))?
                .number,
        ),
        None => project.characteristics.iter().map(|c| c.number).max(),
    };
    let Some(lock) = &project.numbering.lock else {
        // Unlocked numbers ascend in placement order, so the last one has the highest base.
        let last = project
            .characteristics
            .last()
            .map_or(0, |c| c.number.base());
        return Ok((DisplayNumber::plain(last.saturating_add(1)), len));
    };
    let present = project.characteristics.iter().map(|c| c.number);
    let known: Vec<DisplayNumber> = present.chain(lock.given.iter().copied()).collect();
    let mut new_lock = lock.clone();
    let number = match (project.settings.numbering.insert_when_locked, anchor) {
        (InsertPolicy::SubNumber, Some(anchor)) => {
            let highest = known
                .iter()
                .filter(|n| n.base() == anchor.base())
                .filter_map(|n| n.sub())
                .max()
                .unwrap_or(0);
            DisplayNumber::plain(anchor.base()).with_sub(highest.saturating_add(1))
        }
        (InsertPolicy::LetterSuffix, Some(anchor)) => {
            let highest = known
                .iter()
                .filter(|n| n.base() == anchor.base() && n.sub() == anchor.sub())
                .filter_map(|n| n.letter())
                .max()
                .unwrap_or(0);
            anchor.with_letter(highest.saturating_add(1))
        }
        // Next free, or nothing to follow yet.
        _ => {
            let highest_present = known.iter().map(|n| n.base()).max().unwrap_or(0);
            let base = lock.highest_number.max(highest_present).saturating_add(1);
            new_lock.highest_number = base;
            DisplayNumber::plain(base)
        }
    };
    if number.as_plain().is_none()
        && let Err(at) = new_lock.given.binary_search(&number)
    {
        new_lock.given.insert(at, number);
    }
    let index = project
        .characteristics
        .iter()
        .position(|c| c.number > number)
        .unwrap_or(len);
    let before = project.numbering.clone();
    tx.push(Change::NumberingChanged {
        before,
        after: Numbering {
            lock: Some(new_lock),
        },
    })?;
    Ok((number, index))
}

/// While unlocked, numbers follow the placement order: `1..=n`, with sub-numbers for
/// multi-instance groups when the project uses them (D-21, D-22, [`numbers_for_order`]).
pub(super) fn renumber_if_unlocked(tx: &mut Tx<'_>) -> Result<(), CommandError> {
    if tx.project.numbering.lock.is_some() {
        return Ok(());
    }
    let order: Vec<CharId> = tx.project.characteristics.iter().map(|c| c.id).collect();
    let numbers = numbers_for_order(tx.project, &order);
    for (i, number) in numbers.into_iter().enumerate() {
        let current = &tx.project.characteristics[i];
        if current.number != number {
            let before = Box::new(current.clone());
            let mut after = before.clone();
            after.number = number;
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
    }
    Ok(())
}

/// Reorders by `strategy` and renumbers, one undo step (FR-BAL-04, D-21, D-23).
pub(super) fn apply(tx: &mut Tx<'_>, strategy: NumberingStrategy) -> Result<(), CommandError> {
    if tx.project.is_numbering_locked() {
        return Err(CommandError::NumberingLocked);
    }
    change_settings(tx, |s| s.numbering.strategy = strategy)?;
    let old: Vec<CharId> = tx.project.characteristics.iter().map(|c| c.id).collect();
    let new = strategy_order(tx.project, strategy);
    if new != old {
        tx.push(Change::OrderChanged {
            before: old,
            after: new,
        })?;
    }
    renumber_if_unlocked(tx)
}

pub(super) fn lock(
    tx: &mut Tx<'_>,
    reason: LockReason,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    if tx.project.is_numbering_locked() {
        return Ok(());
    }
    let highest_number = tx
        .project
        .characteristics
        .iter()
        .map(|c| c.number.base())
        .max()
        .unwrap_or(0);
    let mut given: Vec<DisplayNumber> = tx
        .project
        .characteristics
        .iter()
        .map(|c| c.number)
        .filter(|n| n.as_plain().is_none())
        .collect();
    given.sort();
    given.dedup();
    let after = Numbering {
        lock: Some(NumberingLock {
            reason,
            locked_at: env.now(),
            locked_by: env.user_name(),
            highest_number,
            // Sub-numbers present when locking (D-22) count as given, so a deleted one is
            // never given again.
            given,
        }),
    };
    tx.push(Change::NumberingChanged {
        before: tx.project.numbering.clone(),
        after,
    })
}
