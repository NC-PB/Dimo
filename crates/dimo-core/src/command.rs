//! Commands: the only way to change a project (05 Architecture, principle 1, ADR 0001).
//!
//! A [`Command`] says what the user wants. [`execute`] checks it against the project and
//! compiles it into primitive [`Change`]s, applying each one as it goes. If any step fails,
//! the changes made so far are rolled back and the project is unchanged.
//!
//! Numbering rules (D-21, D-23) live here: see [`Numbering`](crate::project::Numbering).

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::{Balloon, BalloonStyle, BalloonStyleOverride};
use crate::characteristic::{
    Characteristic, CharacteristicKind, FieldError, FieldValue, SourceRegion, TextSource, Unit,
};
use crate::env::Environment;
use crate::geometry::{OrientedBox, Point};
use crate::id::{BalloonId, CharId, SheetId};
use crate::patch::{Change, ChangeError, invert};
use crate::project::{InsertPolicy, LockReason, Numbering, NumberingLock, Project, ProjectInfo};
use crate::sheet::{Rotation, Scale};

/// New position of one balloon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct BalloonMove {
    /// The balloon.
    pub id: BalloonId,
    /// New center.
    pub position: Point,
    /// New leader anchor; `null` keeps the current one.
    pub anchor: Option<Point>,
}

/// A change the user asks for. Every command is undoable as one step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Adds a characteristic with one balloon at the end of the placement order (FR-BAL-01).
    /// Numbered by placement order, or with the next free number when locked (D-21, D-23).
    AddCharacteristic {
        /// Sheet the balloon and region are on.
        sheet: SheetId,
        /// Balloon center.
        position: Point,
        /// Leader anchor on the feature.
        anchor: Point,
        /// Region the user drew around the callout, if any (FR-CHR-09).
        region: Option<OrientedBox>,
        /// Initial field values.
        values: Vec<FieldValue>,
    },
    /// Sets field values on one or more characteristics (FR-CHR-02).
    UpdateFields {
        /// Characteristics to change.
        ids: Vec<CharId>,
        /// Values, applied in order.
        values: Vec<FieldValue>,
    },
    /// Moves balloons and optionally their leader anchors (FR-BAL-12 group move).
    MoveBalloons {
        /// One entry per balloon.
        moves: Vec<BalloonMove>,
    },
    /// Deletes characteristics with their balloons (FR-BAL-12 group delete).
    DeleteCharacteristics {
        /// Characteristics to delete.
        ids: Vec<CharId>,
    },
    /// Sets style overrides on balloons; fields left `null` keep their current override
    /// (FR-BAL-12 group restyle).
    RestyleBalloons {
        /// Balloons to restyle.
        ids: Vec<BalloonId>,
        /// Overrides to set.
        style: BalloonStyleOverride,
    },
    /// Removes all style overrides, so the balloons use the project default again.
    ResetBalloonStyle {
        /// Balloons to reset.
        ids: Vec<BalloonId>,
    },
    /// Moves characteristics in the placement order and renumbers (renumber by drag, D-21).
    /// The moved characteristics keep their relative order. Refused while numbering is locked.
    MoveCharacteristics {
        /// Characteristics to move.
        ids: Vec<CharId>,
        /// Insert them before this characteristic; `null` moves them to the end.
        before: Option<CharId>,
    },
    /// Sets view rotation, unit or scale of a sheet; `null` keeps the current value (FR-DOC-05).
    UpdateSheet {
        /// The sheet.
        sheet: SheetId,
        /// New rotation.
        rotation: Option<Rotation>,
        /// New length unit, `mm` or `in`.
        unit: Option<Unit>,
        /// New scale.
        scale: Option<Scale>,
    },
    /// Locks numbering (FR-BAL-10). No change if already locked.
    LockNumbering {
        /// Why.
        reason: LockReason,
    },
    /// Unlocks numbering and renumbers by placement order. No change if not locked.
    UnlockNumbering,
    /// Sets the project default balloon style.
    SetDefaultBalloonStyle {
        /// New default.
        style: BalloonStyle,
    },
    /// Replaces part and order data.
    UpdateProjectInfo {
        /// New data.
        info: ProjectInfo,
    },
    /// Several commands as one undo step. If one fails, none is applied.
    Batch {
        /// Commands in order.
        commands: Vec<Command>,
    },
}

/// Why a command was refused. The project is unchanged.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CommandError {
    /// No characteristic with this ID.
    #[error("unknown characteristic {0}")]
    UnknownCharacteristic(CharId),
    /// No balloon with this ID.
    #[error("unknown balloon {0}")]
    UnknownBalloon(BalloonId),
    /// No sheet with this ID.
    #[error("unknown sheet {0}")]
    UnknownSheet(SheetId),
    /// The command would change locked numbers (D-23).
    #[error("numbering is locked")]
    NumberingLocked,
    /// A position or region is not finite, or a region has no positive size.
    #[error("invalid geometry: {0}")]
    InvalidGeometry(&'static str),
    /// A style size is not finite and positive.
    #[error("balloon sizes must be finite and positive")]
    InvalidStyle,
    /// A sheet unit that is not a length, or a scale with a zero part.
    #[error("invalid sheet setting: {0}")]
    InvalidSheetSetting(&'static str),
    /// `MoveCharacteristics` target is one of the moved characteristics.
    #[error("cannot move characteristics before one of themselves")]
    InvalidMoveTarget,
    /// A field value cannot be stored.
    #[error(transparent)]
    Field(#[from] FieldError),
    /// Undo with an empty undo history.
    #[error("nothing to undo")]
    NothingToUndo,
    /// Redo with an empty redo history.
    #[error("nothing to redo")]
    NothingToRedo,
    /// Internal inconsistency; a bug if it ever happens.
    #[error(transparent)]
    Change(#[from] ChangeError),
}

/// Changes applied to a project so far, rolled back on error.
struct Tx<'a> {
    project: &'a mut Project,
    changes: Vec<Change>,
}

impl Tx<'_> {
    fn push(&mut self, change: Change) -> Result<(), CommandError> {
        self.project.apply_change(&change)?;
        self.changes.push(change);
        Ok(())
    }

    fn rollback(self) {
        for change in invert(&self.changes) {
            // Cannot fail: each inverse fits the state its forward change produced.
            let _ = self.project.apply_change(&change);
        }
    }
}

/// Applies `command` to `project` and returns the changes in application order.
/// On error the project is unchanged. An empty list means the command changed nothing.
pub fn execute(
    project: &mut Project,
    command: &Command,
    env: &mut dyn Environment,
) -> Result<Vec<Change>, CommandError> {
    let mut tx = Tx {
        project,
        changes: Vec::new(),
    };
    match run(&mut tx, command, env) {
        Ok(()) => Ok(tx.changes),
        Err(err) => {
            tx.rollback();
            Err(err)
        }
    }
}

fn run(tx: &mut Tx<'_>, command: &Command, env: &mut dyn Environment) -> Result<(), CommandError> {
    match command {
        Command::AddCharacteristic {
            sheet,
            position,
            anchor,
            region,
            values,
        } => add_characteristic(tx, *sheet, *position, *anchor, region.as_ref(), values, env),
        Command::UpdateFields { ids, values } => update_fields(tx, ids, values),
        Command::MoveBalloons { moves } => move_balloons(tx, moves),
        Command::DeleteCharacteristics { ids } => delete_characteristics(tx, ids),
        Command::RestyleBalloons { ids, style } => {
            if !style.is_valid() {
                return Err(CommandError::InvalidStyle);
            }
            update_balloons(tx, ids, |b| b.style = b.style.merged(style))
        }
        Command::ResetBalloonStyle { ids } => {
            update_balloons(tx, ids, |b| b.style = BalloonStyleOverride::default())
        }
        Command::MoveCharacteristics { ids, before } => move_characteristics(tx, ids, *before),
        Command::UpdateSheet {
            sheet,
            rotation,
            unit,
            scale,
        } => update_sheet(tx, *sheet, *rotation, *unit, *scale),
        Command::LockNumbering { reason } => lock_numbering(tx, *reason, env),
        Command::UnlockNumbering => {
            if tx.project.numbering.lock.is_some() {
                tx.push(Change::NumberingChanged {
                    before: tx.project.numbering.clone(),
                    after: Numbering::default(),
                })?;
                renumber_if_unlocked(tx)?;
            }
            Ok(())
        }
        Command::SetDefaultBalloonStyle { style } => {
            if !style.is_valid() {
                return Err(CommandError::InvalidStyle);
            }
            if tx.project.settings.balloon_style != *style {
                let before = Box::new(tx.project.settings.clone());
                let mut after = before.clone();
                after.balloon_style = style.clone();
                tx.push(Change::SettingsChanged { before, after })?;
            }
            Ok(())
        }
        Command::UpdateProjectInfo { info } => {
            if tx.project.info != *info {
                tx.push(Change::InfoChanged {
                    before: tx.project.info.clone(),
                    after: info.clone(),
                })?;
            }
            Ok(())
        }
        Command::Batch { commands } => commands.iter().try_for_each(|c| run(tx, c, env)),
    }
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn index_u32(index: usize) -> u32 {
    // Projects never hold more than u32::MAX items; saturate instead of panicking.
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// Unit for a characteristic that has a nominal but no unit: degrees for angles, otherwise the
/// unit of the sheet the characteristic sits on.
fn fill_unit(project: &Project, c: &mut Characteristic, sheet: Option<SheetId>) {
    if c.nominal.is_none() || c.unit.is_some() {
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

/// Number for a characteristic appended now. Locked: one more than the highest number ever
/// given, and the lock remembers it (D-23).
fn next_number(tx: &mut Tx<'_>) -> Result<u32, CommandError> {
    let highest_present = tx
        .project
        .characteristics
        .iter()
        .map(|c| c.number)
        .max()
        .unwrap_or(0);
    let Some(lock) = &tx.project.numbering.lock else {
        return Ok(index_u32(tx.project.characteristics.len()).saturating_add(1));
    };
    match lock.insert_policy {
        InsertPolicy::NextFree => {
            let number = lock.highest_number.max(highest_present).saturating_add(1);
            let before = tx.project.numbering.clone();
            let mut after = before.clone();
            if let Some(lock) = &mut after.lock {
                lock.highest_number = number;
            }
            tx.push(Change::NumberingChanged { before, after })?;
            Ok(number)
        }
    }
}

/// While unlocked, numbers are `1..=n` in placement order (D-21).
fn renumber_if_unlocked(tx: &mut Tx<'_>) -> Result<(), CommandError> {
    if tx.project.numbering.lock.is_some() {
        return Ok(());
    }
    for i in 0..tx.project.characteristics.len() {
        let number = index_u32(i).saturating_add(1);
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

fn add_characteristic(
    tx: &mut Tx<'_>,
    sheet: SheetId,
    position: Point,
    anchor: Point,
    region: Option<&OrientedBox>,
    values: &[FieldValue],
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    if tx.project.sheet(sheet).is_none() {
        return Err(CommandError::UnknownSheet(sheet));
    }
    if !finite(position) || !finite(anchor) {
        return Err(CommandError::InvalidGeometry("balloon position or anchor"));
    }
    if let Some(r) = region {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !finite(r.center)
            || !r.angle.is_finite()
            || !positive(r.size.width)
            || !positive(r.size.height)
        {
            return Err(CommandError::InvalidGeometry("region"));
        }
    }
    let id = CharId::from_uuid(env.new_uuid());
    let balloon_id = BalloonId::from_uuid(env.new_uuid());
    let number = next_number(tx)?;
    let mut characteristic = Characteristic::manual(id, number);
    characteristic.sources = region
        .map(|region| SourceRegion {
            sheet,
            region: *region,
            text_source: TextSource::Manual,
            raw_text: None,
        })
        .into_iter()
        .collect();
    characteristic.set_values(values)?;
    fill_unit(tx.project, &mut characteristic, Some(sheet));
    tx.push(Change::CharacteristicInserted {
        index: index_u32(tx.project.characteristics.len()),
        characteristic: Box::new(characteristic),
    })?;
    tx.push(Change::BalloonInserted {
        index: index_u32(tx.project.balloons.len()),
        balloon: Box::new(Balloon {
            id: balloon_id,
            characteristic: id,
            sheet,
            position,
            anchor,
            style: BalloonStyleOverride::default(),
        }),
    })
}

/// Distinct IDs in first seen order, all checked to exist.
fn known_chars(project: &Project, ids: &[CharId]) -> Result<Vec<CharId>, CommandError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &id in ids {
        if project.characteristic(id).is_none() {
            return Err(CommandError::UnknownCharacteristic(id));
        }
        if seen.insert(id) {
            out.push(id);
        }
    }
    Ok(out)
}

fn update_fields(
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
        if after != before {
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
    }
    Ok(())
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

fn move_balloons(tx: &mut Tx<'_>, moves: &[BalloonMove]) -> Result<(), CommandError> {
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

fn delete_characteristics(tx: &mut Tx<'_>, ids: &[CharId]) -> Result<(), CommandError> {
    for id in known_chars(tx.project, ids)? {
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

fn move_characteristics(
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

fn update_sheet(
    tx: &mut Tx<'_>,
    id: SheetId,
    rotation: Option<Rotation>,
    unit: Option<Unit>,
    scale: Option<Scale>,
) -> Result<(), CommandError> {
    if unit.is_some_and(|u| !u.is_length()) {
        return Err(CommandError::InvalidSheetSetting("unit must be mm or in"));
    }
    if scale.is_some_and(|s| !s.is_valid()) {
        return Err(CommandError::InvalidSheetSetting(
            "scale parts must be at least 1",
        ));
    }
    let current = tx.project.sheet(id).ok_or(CommandError::UnknownSheet(id))?;
    let before = Box::new(current.clone());
    let mut after = before.clone();
    after.rotation = rotation.unwrap_or(after.rotation);
    after.unit = unit.unwrap_or(after.unit);
    after.scale = scale.unwrap_or(after.scale);
    if after != before {
        tx.push(Change::SheetChanged { before, after })?;
    }
    Ok(())
}

fn lock_numbering(
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
        .map(|c| c.number)
        .max()
        .unwrap_or(0);
    let after = Numbering {
        lock: Some(NumberingLock {
            reason,
            locked_at: env.now(),
            locked_by: env.user_name(),
            insert_policy: InsertPolicy::NextFree,
            highest_number,
        }),
    };
    tx.push(Change::NumberingChanged {
        before: tx.project.numbering.clone(),
        after,
    })
}
