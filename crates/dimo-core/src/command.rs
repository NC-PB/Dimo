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
use crate::number::DisplayNumber;
use crate::patch::{Change, ChangeError, invert};
use crate::project::{
    InsertPolicy, LockReason, Numbering, NumberingLock, NumberingSettings, Project, ProjectInfo,
    ProjectSettings, ToleranceSettings,
};
use crate::proposal::{BalloonPlacement, Proposal};
use crate::sheet::{Rotation, Scale, Sheet, SheetView, ZoneGrid};

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
    /// Adds a characteristic with one balloon (FR-BAL-01). Unlocked: at the end of the
    /// placement order, numbered by it (D-21). Locked: numbered by the insert policy and placed
    /// in number order (D-23, FR-BAL-11).
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
        /// Anchor of the sub-number and letter insert policies while locked: the new number
        /// follows this characteristic. `null` uses the highest number. Not used while unlocked
        /// and by the next free policy, but it must name an existing characteristic.
        insert_after: Option<CharId>,
    },
    /// Turns proposals into accepted characteristics with one balloon each, as one undo step
    /// (ADR 0006, FR-REC-02). Numbered like [`Command::AddCharacteristic`], in the given order.
    AcceptProposals {
        /// Proposals to accept, possibly edited by the user.
        proposals: Vec<Proposal>,
        /// Anchor of the locked insert policies, as for [`Command::AddCharacteristic`].
        insert_after: Option<CharId>,
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
    /// Sets or removes the zone grid of a sheet (M2 decision 1, D-21).
    SetZoneGrid {
        /// The sheet.
        sheet: SheetId,
        /// New grid; `null` removes it.
        grid: Option<ZoneGrid>,
    },
    /// Replaces the view rectangles of a sheet (M2 decision 1).
    SetViews {
        /// The sheet.
        sheet: SheetId,
        /// All views of the sheet, in order.
        views: Vec<SheetView>,
    },
    /// Sets the numbering strategy, multi-instance numbering and locked insert policy
    /// (FR-BAL-04, FR-BAL-07, FR-BAL-11). Numbers do not change.
    SetNumberingSettings {
        /// New settings.
        settings: NumberingSettings,
    },
    /// Sets the tolerance rules of the project (M2 decision 2, FR-TOL-06, FR-TOL-09). Limits
    /// of existing characteristics do not change.
    SetToleranceSettings {
        /// New settings.
        settings: ToleranceSettings,
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
    /// A sheet unit that is not a length, a scale with a zero part, or an invalid zone grid or
    /// view.
    #[error("invalid sheet setting: {0}")]
    InvalidSheetSetting(&'static str),
    /// Project settings that break their rules, e.g. unsorted decimal rules.
    #[error("invalid project setting: {0}")]
    InvalidProjectSetting(&'static str),
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
            insert_after,
        } => add_characteristic(
            tx,
            *sheet,
            BalloonPlacement {
                position: *position,
                anchor: *anchor,
            },
            region.as_ref(),
            values,
            *insert_after,
            env,
        ),
        Command::AcceptProposals {
            proposals,
            insert_after,
        } => proposals
            .iter()
            .try_for_each(|p| accept_proposal(tx, p, *insert_after, env)),
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
        Command::SetZoneGrid { sheet, grid } => {
            if let Some(grid) = grid {
                grid.validate().map_err(CommandError::InvalidSheetSetting)?;
            }
            change_sheet(tx, *sheet, |s| s.zone_grid.clone_from(grid))
        }
        Command::SetViews { sheet, views } => {
            if views.len() > MAX_VIEWS {
                return Err(CommandError::InvalidSheetSetting(
                    "a sheet has at most 1000 views",
                ));
            }
            for view in views {
                view.validate().map_err(CommandError::InvalidSheetSetting)?;
            }
            change_sheet(tx, *sheet, |s| s.views.clone_from(views))
        }
        Command::SetNumberingSettings { settings } => {
            change_settings(tx, |s| s.numbering = *settings)
        }
        Command::SetToleranceSettings { settings } => {
            settings
                .validate()
                .map_err(CommandError::InvalidProjectSetting)?;
            change_settings(tx, |s| s.tolerance.clone_from(settings))
        }
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
            change_settings(tx, |s| s.balloon_style = style.clone())
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

/// Number and placement index for a characteristic added now (D-21, D-23, FR-BAL-11).
///
/// Unlocked: the next number at the end of the placement order. Locked: the number of the
/// insert policy, placed in number order; the lock remembers it, so it is never given again.
fn insert_slot(
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
        return Ok((DisplayNumber::plain(index_u32(len).saturating_add(1)), len));
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

/// While unlocked, numbers are `1..=n` in placement order (D-21).
fn renumber_if_unlocked(tx: &mut Tx<'_>) -> Result<(), CommandError> {
    if tx.project.numbering.lock.is_some() {
        return Ok(());
    }
    for i in 0..tx.project.characteristics.len() {
        let number = DisplayNumber::plain(index_u32(i).saturating_add(1));
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

fn add_characteristic(
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
    })
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
    insert_new(tx, sheet, proposal.placement, insert_after, env, |c| {
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
        c.quantity = proposal.quantity;
        c.inspect = proposal.inspect;
        c.origin = proposal.origin;
        c.sources = vec![proposal.source.clone()];
        Ok(())
    })
}

/// Most view rectangles per sheet.
const MAX_VIEWS: usize = 1000;

/// Edits one sheet; records a change only if it differs.
fn change_sheet(
    tx: &mut Tx<'_>,
    id: SheetId,
    edit: impl FnOnce(&mut Sheet),
) -> Result<(), CommandError> {
    let current = tx.project.sheet(id).ok_or(CommandError::UnknownSheet(id))?;
    let before = Box::new(current.clone());
    let mut after = before.clone();
    edit(&mut after);
    if after != before {
        tx.push(Change::SheetChanged { before, after })?;
    }
    Ok(())
}

/// Edits the project settings; records a change only if they differ.
fn change_settings(
    tx: &mut Tx<'_>,
    edit: impl FnOnce(&mut ProjectSettings),
) -> Result<(), CommandError> {
    let before = Box::new(tx.project.settings.clone());
    let mut after = before.clone();
    edit(&mut after);
    if after != before {
        tx.push(Change::SettingsChanged { before, after })?;
    }
    Ok(())
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
    change_sheet(tx, id, |s| {
        s.rotation = rotation.unwrap_or(s.rotation);
        s.unit = unit.unwrap_or(s.unit);
        s.scale = scale.unwrap_or(s.scale);
    })
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
        .map(|c| c.number.base())
        .max()
        .unwrap_or(0);
    let after = Numbering {
        lock: Some(NumberingLock {
            reason,
            locked_at: env.now(),
            locked_by: env.user_name(),
            highest_number,
            given: Vec::new(),
        }),
    };
    tx.push(Change::NumberingChanged {
        before: tx.project.numbering.clone(),
        after,
    })
}
