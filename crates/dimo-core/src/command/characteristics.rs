//! Characteristics: add, accept proposals, edit fields, delete and reorder (FR-BAL-01,
//! FR-CHR-02, ADR 0006).

use std::collections::BTreeSet;

use super::numbering::{insert_slot, renumber_if_unlocked};
use super::{CommandError, Tx, finite, index_u32, known_chars};
use crate::balloon::{Balloon, BalloonStyleOverride, UNITS_PER_MM};
use crate::characteristic::{
    Characteristic, CharacteristicKind, FieldError, FieldValue, SourceRegion, TextSource, Unit,
};
use crate::env::Environment;
use crate::geometry::OrientedBox;
use crate::id::{BalloonId, CharId, SheetId};
use crate::measurement::{MeasurementResult, ResultCell};
use crate::patch::Change;
use crate::project::{MultiInstance, Project};
use crate::proposal::{BalloonPlacement, Proposal};

/// Unit for a characteristic with a nominal: degrees for angles, otherwise the unit of the
/// sheet the characteristic sits on. Filled when missing, and corrected when it contradicts the
/// kind (an angle in mm, a size in degrees), for example after a kind change.
fn fill_unit(project: &Project, c: &mut Characteristic, sheet: Option<SheetId>) {
    if c.nominal.is_none() {
        return;
    }
    let length_kind = matches!(
        c.kind,
        CharacteristicKind::Linear
            | CharacteristicKind::Diameter
            | CharacteristicKind::Radius
            | CharacteristicKind::SphericalRadius
            | CharacteristicKind::Depth
    );
    let fits = match c.unit {
        None => false,
        Some(unit) if c.kind == CharacteristicKind::Angle => unit == Unit::Deg,
        Some(unit) => !length_kind || unit.is_length(),
    };
    if fits {
        return;
    }
    c.unit = Some(if c.kind == CharacteristicKind::Angle {
        Unit::Deg
    } else {
        sheet
            .and_then(|id| project.sheet(id))
            .map_or(Unit::Mm, |s| s.unit)
    });
}

fn sheet_of(project: &Project, c: &Characteristic) -> Option<SheetId> {
    project
        .balloons_of(c.id)
        .map(|b| b.sheet)
        .next()
        .or_else(|| c.sources.first().map(|s| s.sheet))
}

fn valid_region(r: &OrientedBox) -> bool {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    finite(r.center) && r.angle.is_finite() && positive(r.size.width) && positive(r.size.height)
}

/// Checks sheet and geometry of a new characteristic with its balloon.
fn check_new(
    project: &Project,
    sheet: SheetId,
    placement: BalloonPlacement,
    region: Option<&OrientedBox>,
) -> Result<(), CommandError> {
    if project.sheet(sheet).is_none() {
        return Err(CommandError::UnknownSheet(sheet));
    }
    if !finite(placement.position) || !finite(placement.anchor) {
        return Err(CommandError::InvalidGeometry("balloon position or anchor"));
    }
    if region.is_some_and(|r| !valid_region(r)) {
        return Err(CommandError::InvalidGeometry("region"));
    }
    Ok(())
}

/// Inserts a characteristic built by `build` from its ID and number, with one balloon.
fn insert_new(
    tx: &mut Tx<'_>,
    sheet: SheetId,
    placement: BalloonPlacement,
    insert_after: Option<CharId>,
    env: &mut dyn Environment,
    build: impl FnOnce(&mut Characteristic) -> Result<(), CommandError>,
) -> Result<(), CommandError> {
    let id = CharId::from_uuid(env.new_uuid());
    let balloon_id = BalloonId::from_uuid(env.new_uuid());
    let (number, index) = insert_slot(tx, insert_after)?;
    let mut characteristic = Characteristic::manual(id, number);
    build(&mut characteristic)?;
    fill_unit(tx.project, &mut characteristic, Some(sheet));
    tx.push(Change::CharacteristicInserted {
        index: index_u32(index),
        characteristic: Box::new(characteristic),
    })?;
    tx.push(Change::BalloonInserted {
        index: index_u32(tx.project.balloons.len()),
        balloon: Box::new(Balloon {
            id: balloon_id,
            characteristic: id,
            sheet,
            position: placement.position,
            anchor: placement.anchor,
            style: BalloonStyleOverride::default(),
        }),
    })
}

pub(super) fn add(
    tx: &mut Tx<'_>,
    sheet: SheetId,
    placement: BalloonPlacement,
    region: Option<&OrientedBox>,
    values: &[FieldValue],
    insert_after: Option<CharId>,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    check_new(tx.project, sheet, placement, region)?;
    insert_new(tx, sheet, placement, insert_after, env, |c| {
        c.sources = region
            .map(|region| SourceRegion {
                sheet,
                region: *region,
                text_source: TextSource::Manual,
                raw_text: None,
            })
            .into_iter()
            .collect();
        c.set_values(values)?;
        Ok(())
    })?;
    renumber_if_unlocked(tx)
}

/// Accepts proposals in order, then renumbers once (ADR 0006).
pub(super) fn accept_proposals(
    tx: &mut Tx<'_>,
    proposals: &[Proposal],
    insert_after: Option<CharId>,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    for proposal in proposals {
        accept_proposal(tx, proposal, insert_after, env)?;
    }
    renumber_if_unlocked(tx)
}

/// One accepted proposal becomes a characteristic with the proposed values (ADR 0006).
fn accept_proposal(
    tx: &mut Tx<'_>,
    proposal: &Proposal,
    insert_after: Option<CharId>,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    let sheet = proposal.source.sheet;
    check_new(
        tx.project,
        sheet,
        proposal.placement,
        Some(&proposal.source.region),
    )?;
    if proposal.quantity == 0 {
        return Err(FieldError::ZeroQuantity.into());
    }
    // D-22, FR-BAL-07: with sub-numbers, a callout for several features becomes one
    // characteristic per feature with the same source region; their balloons are stacked
    // downwards, a little more than one balloon size apart, and numbering gives them
    // sub-numbers. While locked, and above `MAX_SPLIT` features, it stays one characteristic
    // with the quantity: locked numbers follow the insert policy only (D-23).
    let split = tx.project.settings.numbering.multi_instance == MultiInstance::SubNumber
        && !tx.project.is_numbering_locked()
        && (2..=MAX_SPLIT).contains(&proposal.quantity);
    let (copies, quantity) = if split {
        (proposal.quantity, 1)
    } else {
        (1, proposal.quantity)
    };
    // Whole sheet units, so positions stay exact in `project.json`.
    let step = (tx.project.settings.balloon_style.size_mm * UNITS_PER_MM * 1.2).round();
    for copy in 0..copies {
        let mut placement = proposal.placement;
        placement.position.y += step * f64::from(copy);
        accept_one(tx, proposal, placement, quantity, insert_after, env)?;
    }
    Ok(())
}

/// Most characteristics one multi-instance callout is split into (D-22).
const MAX_SPLIT: u32 = 100;

/// Inserts one characteristic with the values of `proposal`.
fn accept_one(
    tx: &mut Tx<'_>,
    proposal: &Proposal,
    placement: BalloonPlacement,
    quantity: u32,
    insert_after: Option<CharId>,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    let sheet = proposal.source.sheet;
    insert_new(tx, sheet, placement, insert_after, env, |c| {
        c.kind = proposal.kind;
        proposal
            .requirement_text
            .trim()
            .clone_into(&mut c.requirement_text);
        c.nominal = proposal.nominal;
        c.unit = proposal.unit;
        c.upper_dev = proposal.upper_dev;
        c.lower_dev = proposal.lower_dev;
        c.upper_limit = proposal.upper_limit;
        c.lower_limit = proposal.lower_limit;
        c.fit = proposal
            .fit
            .as_deref()
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .map(str::to_owned);
        c.derivation.clone_from(&proposal.derivation);
        c.quantity = quantity;
        c.inspect = proposal.inspect;
        c.origin = proposal.origin;
        c.sources = vec![proposal.source.clone()];
        c.shorter_leg = proposal.shorter_leg;
        Ok(())
    })
}

pub(super) fn update_fields(
    tx: &mut Tx<'_>,
    ids: &[CharId],
    values: &[FieldValue],
) -> Result<(), CommandError> {
    for id in known_chars(tx.project, ids)? {
        let Some(current) = tx.project.characteristic(id) else {
            continue;
        };
        let before = Box::new(current.clone());
        let mut after = before.clone();
        after.set_values(values)?;
        let sheet = sheet_of(tx.project, &after);
        fill_unit(tx.project, &mut after, sheet);
        if after != before || digits_differ(&before, &after) {
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
    }
    // A copy of a repeated feature whose kind or text is edited leaves its group (D-22).
    renumber_if_unlocked(tx)
}

/// True if a decimal field is equal in value but written with other digits, e.g. `8` and
/// `8.0`. `Decimal` equality ignores the scale, but the typed digits are kept and exported
/// (FR-EXP-11), so such an edit is a change.
fn digits_differ(a: &Characteristic, b: &Characteristic) -> bool {
    let digits = |c: &Characteristic| {
        [
            c.nominal,
            c.upper_dev,
            c.lower_dev,
            c.upper_limit,
            c.lower_limit,
        ]
        .map(|d| d.map(|d| d.scale()))
    };
    digits(a) != digits(b)
}

pub(super) fn delete(tx: &mut Tx<'_>, ids: &[CharId]) -> Result<(), CommandError> {
    for id in known_chars(tx.project, ids)? {
        // M3 decision 16: results go with their characteristic, in the same undo step.
        let cells: Vec<ResultCell> = tx
            .project
            .results_of(id)
            .map(MeasurementResult::cell)
            .collect();
        for cell in cells {
            super::measurement::set_result(tx, cell, None)?;
        }
        while let Some(index) = tx
            .project
            .balloons
            .iter()
            .position(|b| b.characteristic == id)
        {
            let balloon = Box::new(tx.project.balloons[index].clone());
            tx.push(Change::BalloonRemoved {
                index: index_u32(index),
                balloon,
            })?;
        }
        if let Some(index) = tx.project.characteristics.iter().position(|c| c.id == id) {
            let characteristic = Box::new(tx.project.characteristics[index].clone());
            tx.push(Change::CharacteristicRemoved {
                index: index_u32(index),
                characteristic,
            })?;
        }
    }
    renumber_if_unlocked(tx)
}

pub(super) fn move_to(
    tx: &mut Tx<'_>,
    ids: &[CharId],
    before: Option<CharId>,
) -> Result<(), CommandError> {
    if tx.project.is_numbering_locked() {
        return Err(CommandError::NumberingLocked);
    }
    let moved: BTreeSet<CharId> = known_chars(tx.project, ids)?.into_iter().collect();
    if let Some(target) = before {
        if tx.project.characteristic(target).is_none() {
            return Err(CommandError::UnknownCharacteristic(target));
        }
        if moved.contains(&target) {
            return Err(CommandError::InvalidMoveTarget);
        }
    }
    let old: Vec<CharId> = tx.project.characteristics.iter().map(|c| c.id).collect();
    let (picked, mut new): (Vec<CharId>, Vec<CharId>) =
        old.iter().copied().partition(|id| moved.contains(id));
    let at = before
        .and_then(|target| new.iter().position(|id| *id == target))
        .unwrap_or(new.len());
    new.splice(at..at, picked);
    if new != old {
        tx.push(Change::OrderChanged {
            before: old,
            after: new,
        })?;
        renumber_if_unlocked(tx)?;
    }
    Ok(())
}
