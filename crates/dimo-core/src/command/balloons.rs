//! Balloons: move, restyle and reset style (FR-BAL-12).

use super::{BalloonMove, CommandError, Tx, finite};
use crate::balloon::{Balloon, BalloonStyleOverride};
use crate::id::BalloonId;
use crate::patch::Change;

/// Sets style overrides; fields left `null` keep their current override.
pub(super) fn restyle(
    tx: &mut Tx<'_>,
    ids: &[BalloonId],
    style: &BalloonStyleOverride,
) -> Result<(), CommandError> {
    if !style.is_valid() {
        return Err(CommandError::InvalidStyle);
    }
    update_balloons(tx, ids, |b| b.style = b.style.merged(style))
}

/// Removes all style overrides.
pub(super) fn reset_style(tx: &mut Tx<'_>, ids: &[BalloonId]) -> Result<(), CommandError> {
    update_balloons(tx, ids, |b| b.style = BalloonStyleOverride::default())
}

fn update_balloons(
    tx: &mut Tx<'_>,
    ids: &[BalloonId],
    edit: impl Fn(&mut Balloon),
) -> Result<(), CommandError> {
    for &id in ids {
        let current = tx
            .project
            .balloon(id)
            .ok_or(CommandError::UnknownBalloon(id))?;
        let before = Box::new(current.clone());
        let mut after = before.clone();
        edit(&mut after);
        if after != before {
            tx.push(Change::BalloonChanged { before, after })?;
        }
    }
    Ok(())
}

pub(super) fn move_balloons(tx: &mut Tx<'_>, moves: &[BalloonMove]) -> Result<(), CommandError> {
    for m in moves {
        if !finite(m.position) || m.anchor.is_some_and(|a| !finite(a)) {
            return Err(CommandError::InvalidGeometry("balloon position or anchor"));
        }
        update_balloons(tx, &[m.id], |b| {
            b.position = m.position;
            if let Some(anchor) = m.anchor {
                b.anchor = anchor;
            }
        })?;
    }
    Ok(())
}
