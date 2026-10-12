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

use crate::balloon::{Balloon, BalloonStyle, BalloonStyleOverride, UNITS_PER_MM};
use crate::characteristic::{
    Characteristic, CharacteristicKind, FieldError, FieldValue, SourceRegion, TextSource, Unit,
};
use crate::env::Environment;
use crate::geometry::{OrientedBox, Point};
use crate::id::{BalloonId, CharId, RevisionId, SampleId, SheetId};
use crate::measurement::{
    MAX_SAMPLES, MAX_SAMPLES_PER_ADD, MeasurementResult, ResultCell, ResultEntry, ResultParseError,
    ResultSource, ResultType, Sample, SerialError, check_serial, is_date_or_empty, next_serials,
    parse_result,
};
use crate::number::DisplayNumber;
use crate::numbering::{numbers_for_order, strategy_order};
use crate::patch::{Change, ChangeError, invert};
use crate::project::{
    InsertPolicy, LockReason, MultiInstance, Numbering, NumberingLock, NumberingSettings,
    NumberingStrategy, Project, ProjectInfo, ProjectSettings, ToleranceSettings,
};
use crate::proposal::{BalloonPlacement, Proposal};
use crate::report::{IssuedReport, ReportFormat, ReportInfo, ReportKind, TemplateRef};
use crate::sheet::{Rotation, Scale, Sha256Hex, Sheet, SheetView, ZoneGrid};

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
    /// Sets field values on one or more characteristics (FR-CHR-02). While unlocked, a copy of a
    /// repeated feature whose kind or text changes leaves its sub-number group (D-22).
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
    /// (FR-BAL-04, FR-BAL-07, FR-BAL-11). The order does not change. While unlocked, a change of
    /// multi-instance numbering renumbers groups as sub-numbers or plain numbers (D-22).
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
    /// Reorders and renumbers all characteristics by a numbering strategy as one undo step, and
    /// makes it the strategy of the project (FR-BAL-04, D-21). The result equals
    /// [`numbering::preview`](crate::numbering::preview) (FR-BAL-05). Refused while numbering is
    /// locked (D-23).
    ApplyNumbering {
        /// The strategy.
        strategy: NumberingStrategy,
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
    /// Adds `count` samples (1 to 100) at the end of the sample order (FR-MEA-01, M3
    /// decision 6). Serials continue the highest plain number, the inspector is the audit user
    /// (D-27), other fields are empty. Refused above 500 samples per project.
    AddSamples {
        /// How many samples to add.
        count: u32,
    },
    /// Replaces serial, date, inspector, gauges and comment of the sample with the same ID. The
    /// serial is trimmed, must not be empty and must be unique; the date is `YYYY-MM-DD` or
    /// empty (M3 decision 6).
    UpdateSample {
        /// The sample with its new values.
        sample: Sample,
    },
    /// Deletes samples with their results (M3 decision 16).
    DeleteSamples {
        /// Samples to delete.
        ids: Vec<SampleId>,
    },
    /// Moves samples in the column order. The moved samples keep their relative order.
    MoveSamples {
        /// Samples to move.
        ids: Vec<SampleId>,
        /// Insert them before this sample; `null` moves them to the end.
        before: Option<SampleId>,
    },
    /// Sets the results of cells from typed or pasted text, as one undo step (FR-MEA-02, M3
    /// decisions 7 and 8). Each text is parsed by the effective result type of its
    /// characteristic; empty text clears the cell. Refused as a whole if one text is invalid.
    /// A result on an `auto` characteristic pins it to the effective type in the same step.
    SetResults {
        /// Cells in order; a later entry for the same cell wins.
        results: Vec<ResultEntry>,
    },
    /// Clears the results of cells. Cells without a result are left as they are.
    ClearResults {
        /// Cells to clear.
        cells: Vec<ResultCell>,
    },
    /// Sets how characteristics are measured (FR-MEA-06, M3 decision 4). Refused while a
    /// characteristic has results of the other type; nothing is converted. `auto` on a
    /// characteristic with results keeps its effective type pinned.
    SetResultType {
        /// Characteristics to change.
        ids: Vec<CharId>,
        /// New result type.
        result_type: ResultType,
    },
    /// Replaces the report header and approval data (M3 decision 1). The report date is
    /// `YYYY-MM-DD` or empty.
    UpdateReportInfo {
        /// New data.
        info: ReportInfo,
    },
    /// Sets the label of a drawing revision, trimmed (FR-RVC-04).
    UpdateRevisionLabel {
        /// The revision.
        revision: RevisionId,
        /// New label; empty if unknown.
        label: String,
    },
    /// Records a report exported as issued (D-23, M3 decision 17). Revision and drawing hash
    /// are those of the current revision, the time comes from the environment clock. Exports
    /// send it in a batch with [`Command::LockNumbering`], so lock and record are one undo
    /// step.
    RecordIssuedReport {
        /// Which document.
        kind: ReportKind,
        /// The template it was rendered with.
        template: TemplateRef,
        /// File format.
        format: ReportFormat,
        /// SHA-256 of the written file.
        output_sha256: Sha256Hex,
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
    /// No sample with this ID.
    #[error("unknown sample {0}")]
    UnknownSample(SampleId),
    /// No drawing revision with this ID.
    #[error("unknown drawing revision {0}")]
    UnknownRevision(RevisionId),
    /// A serial that is empty or used by another sample (M3 decision 6).
    #[error(transparent)]
    Serial(#[from] SerialError),
    /// The project would hold more than [`MAX_SAMPLES`] samples (M3 decision 6).
    #[error("a project holds at most {MAX_SAMPLES} samples")]
    TooManySamples,
    /// Samples are added 1 to [`MAX_SAMPLES_PER_ADD`] at once.
    #[error("add 1 to {MAX_SAMPLES_PER_ADD} samples at once")]
    InvalidSampleCount,
    /// A date that is neither empty nor a calendar date `YYYY-MM-DD`.
    #[error("invalid date {0:?}, expected YYYY-MM-DD")]
    InvalidDate(String),
    /// A typed result that does not fit the result type of its characteristic.
    #[error("invalid result for characteristic {characteristic}: {error}")]
    InvalidResult {
        /// The characteristic of the cell.
        characteristic: CharId,
        /// Why the text was not accepted.
        error: ResultParseError,
    },
    /// The result type cannot change while results of the other type exist (M3 decision 4).
    #[error("characteristic {0} has results of the other result type; clear them first")]
    ResultTypeConflict(CharId),
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

#[allow(clippy::too_many_lines, reason = "one arm per command, exhaustive")]
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
        } => accept_proposals(tx, proposals, *insert_after, env),
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
            change_settings(tx, |s| s.numbering = *settings)?;
            renumber_if_unlocked(tx)
        }
        Command::SetToleranceSettings { settings } => {
            settings
                .validate()
                .map_err(CommandError::InvalidProjectSetting)?;
            change_settings(tx, |s| s.tolerance.clone_from(settings))
        }
        Command::ApplyNumbering { strategy } => apply_numbering(tx, *strategy),
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
        command @ (Command::AddSamples { .. }
        | Command::UpdateSample { .. }
        | Command::DeleteSamples { .. }
        | Command::MoveSamples { .. }
        | Command::SetResults { .. }
        | Command::ClearResults { .. }
        | Command::SetResultType { .. }
        | Command::UpdateReportInfo { .. }
        | Command::UpdateRevisionLabel { .. }
        | Command::RecordIssuedReport { .. }) => run_measurement(tx, command, env),
        Command::Batch { commands } => commands.iter().try_for_each(|c| run(tx, c, env)),
    }
}

/// [`run`] for samples, results and report data (data model version 3, M3 decision 1).
fn run_measurement(
    tx: &mut Tx<'_>,
    command: &Command,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    match command {
        Command::AddSamples { count } => add_samples(tx, *count, env),
        Command::UpdateSample { sample } => update_sample(tx, sample),
        Command::DeleteSamples { ids } => delete_samples(tx, ids),
        Command::MoveSamples { ids, before } => move_samples(tx, ids, *before),
        Command::SetResults { results } => set_results(tx, results),
        Command::ClearResults { cells } => cells.iter().try_for_each(|cell| {
            check_cell(tx.project, *cell)?;
            set_result(tx, *cell, None)
        }),
        Command::SetResultType { ids, result_type } => set_result_type(tx, ids, *result_type),
        Command::UpdateReportInfo { info } => update_report_info(tx, info),
        Command::UpdateRevisionLabel { revision, label } => {
            update_revision_label(tx, *revision, label)
        }
        Command::RecordIssuedReport {
            kind,
            template,
            format,
            output_sha256,
        } => record_issued_report(tx, *kind, template, *format, output_sha256, env),
        // `run` sends only the commands above; anything else is a bug.
        _ => Err(ChangeError("not a sample, result or report command").into()),
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
fn renumber_if_unlocked(tx: &mut Tx<'_>) -> Result<(), CommandError> {
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
    })?;
    renumber_if_unlocked(tx)
}

/// Accepts proposals in order, then renumbers once (ADR 0006).
fn accept_proposals(
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
        // M3 decision 16: results go with their characteristic, in the same undo step.
        let cells: Vec<ResultCell> = tx
            .project
            .results_of(id)
            .map(MeasurementResult::cell)
            .collect();
        for cell in cells {
            set_result(tx, cell, None)?;
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

/// Reorders by `strategy` and renumbers, one undo step (FR-BAL-04, D-21, D-23).
fn apply_numbering(tx: &mut Tx<'_>, strategy: NumberingStrategy) -> Result<(), CommandError> {
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

/// Distinct sample IDs in first seen order, all checked to exist.
fn known_samples(project: &Project, ids: &[SampleId]) -> Result<Vec<SampleId>, CommandError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &id in ids {
        if project.sample(id).is_none() {
            return Err(CommandError::UnknownSample(id));
        }
        if seen.insert(id) {
            out.push(id);
        }
    }
    Ok(out)
}

/// Checks that sample and characteristic of a cell exist.
fn check_cell(project: &Project, cell: ResultCell) -> Result<(), CommandError> {
    if project.sample(cell.sample).is_none() {
        return Err(CommandError::UnknownSample(cell.sample));
    }
    if project.characteristic(cell.characteristic).is_none() {
        return Err(CommandError::UnknownCharacteristic(cell.characteristic));
    }
    Ok(())
}

/// Sets or clears the result of one cell; records a change only if value, digits, source or
/// comment differ.
fn set_result(
    tx: &mut Tx<'_>,
    cell: ResultCell,
    after: Option<MeasurementResult>,
) -> Result<(), CommandError> {
    let before = tx.project.result(cell).cloned();
    let same = match (&before, &after) {
        (Some(a), Some(b)) => {
            a.value.same_as(&b.value) && a.source == b.source && a.comment == b.comment
        }
        (None, None) => true,
        _ => false,
    };
    if !same {
        tx.push(Change::ResultChanged {
            cell,
            before,
            after,
        })?;
    }
    Ok(())
}

fn add_samples(tx: &mut Tx<'_>, count: u32, env: &mut dyn Environment) -> Result<(), CommandError> {
    if !(1..=MAX_SAMPLES_PER_ADD).contains(&count) {
        return Err(CommandError::InvalidSampleCount);
    }
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if tx.project.samples.len().saturating_add(count) > MAX_SAMPLES {
        return Err(CommandError::TooManySamples);
    }
    for serial in next_serials(&tx.project.samples, count) {
        let sample = Sample {
            id: SampleId::from_uuid(env.new_uuid()),
            serial,
            date: String::new(),
            inspector: env.user_name(),
            gauges: String::new(),
            comment: String::new(),
        };
        tx.push(Change::SampleInserted {
            index: index_u32(tx.project.samples.len()),
            sample,
        })?;
    }
    Ok(())
}

fn update_sample(tx: &mut Tx<'_>, sample: &Sample) -> Result<(), CommandError> {
    let current = tx
        .project
        .sample(sample.id)
        .ok_or(CommandError::UnknownSample(sample.id))?;
    let serial = check_serial(&tx.project.samples, &sample.serial, Some(sample.id))?;
    if !is_date_or_empty(&sample.date) {
        return Err(CommandError::InvalidDate(sample.date.clone()));
    }
    let after = Sample {
        serial,
        ..sample.clone()
    };
    if *current != after {
        tx.push(Change::SampleChanged {
            before: current.clone(),
            after,
        })?;
    }
    Ok(())
}

fn delete_samples(tx: &mut Tx<'_>, ids: &[SampleId]) -> Result<(), CommandError> {
    for id in known_samples(tx.project, ids)? {
        // M3 decision 16: results go with their sample, in the same undo step.
        let cells: Vec<ResultCell> = tx
            .project
            .results
            .iter()
            .filter(|r| r.sample == id)
            .map(MeasurementResult::cell)
            .collect();
        for cell in cells {
            set_result(tx, cell, None)?;
        }
        if let Some(index) = tx.project.samples.iter().position(|s| s.id == id) {
            let sample = tx.project.samples[index].clone();
            tx.push(Change::SampleRemoved {
                index: index_u32(index),
                sample,
            })?;
        }
    }
    Ok(())
}

fn move_samples(
    tx: &mut Tx<'_>,
    ids: &[SampleId],
    before: Option<SampleId>,
) -> Result<(), CommandError> {
    let moved: BTreeSet<SampleId> = known_samples(tx.project, ids)?.into_iter().collect();
    if let Some(target) = before {
        if tx.project.sample(target).is_none() {
            return Err(CommandError::UnknownSample(target));
        }
        if moved.contains(&target) {
            return Err(CommandError::InvalidMoveTarget);
        }
    }
    let old: Vec<SampleId> = tx.project.samples.iter().map(|s| s.id).collect();
    let (picked, mut new): (Vec<SampleId>, Vec<SampleId>) =
        old.iter().copied().partition(|id| moved.contains(id));
    let at = before
        .and_then(|target| new.iter().position(|id| *id == target))
        .unwrap_or(new.len());
    new.splice(at..at, picked);
    if new != old {
        tx.push(Change::SampleOrderChanged {
            before: old,
            after: new,
        })?;
    }
    Ok(())
}

fn set_results(tx: &mut Tx<'_>, entries: &[ResultEntry]) -> Result<(), CommandError> {
    for entry in entries {
        let cell = ResultCell {
            sample: entry.sample,
            characteristic: entry.characteristic,
        };
        check_cell(tx.project, cell)?;
        if entry.text.trim().is_empty() {
            set_result(tx, cell, None)?;
            continue;
        }
        let Some(characteristic) = tx.project.characteristic(cell.characteristic) else {
            return Err(CommandError::UnknownCharacteristic(cell.characteristic));
        };
        let effective = characteristic.effective_result_type();
        let value =
            parse_result(&entry.text, effective).map_err(|error| CommandError::InvalidResult {
                characteristic: cell.characteristic,
                error,
            })?;
        // M3 decision 4 (owner addition): a result pins an `auto` characteristic to its
        // effective type in the same undo step, so a later limit change never flips it.
        if characteristic.result_type == ResultType::Auto {
            // Only a project file written before pinning existed can hold results on an
            // `auto` characteristic; never mix types in one row.
            if tx
                .project
                .results_of(cell.characteristic)
                .any(|r| r.value.result_type() != effective)
            {
                return Err(CommandError::ResultTypeConflict(cell.characteristic));
            }
            let before = Box::new(characteristic.clone());
            let mut after = before.clone();
            after.result_type = effective;
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
        let comment = match &entry.comment {
            Some(comment) => comment.clone(),
            None => tx
                .project
                .result(cell)
                .map(|r| r.comment.clone())
                .unwrap_or_default(),
        };
        let result = MeasurementResult {
            sample: cell.sample,
            characteristic: cell.characteristic,
            value,
            source: ResultSource::Manual,
            comment,
        };
        set_result(tx, cell, Some(result))?;
    }
    Ok(())
}

fn set_result_type(
    tx: &mut Tx<'_>,
    ids: &[CharId],
    result_type: ResultType,
) -> Result<(), CommandError> {
    for id in known_chars(tx.project, ids)? {
        let Some(current) = tx.project.characteristic(id) else {
            continue;
        };
        let before = Box::new(current.clone());
        let mut after = before.clone();
        after.result_type = result_type;
        let effective = after.effective_result_type();
        let has_results = tx.project.results_of(id).next().is_some();
        if tx
            .project
            .results_of(id)
            .any(|r| r.value.result_type() != effective)
        {
            return Err(CommandError::ResultTypeConflict(id));
        }
        // M3 decision 4: `auto` never holds while results exist, else a later limit change
        // would flip the type under them. Back to `auto` keeps the pinned effective type.
        if has_results {
            after.result_type = effective;
        }
        if after != before {
            tx.push(Change::CharacteristicChanged { before, after })?;
        }
    }
    Ok(())
}

fn update_report_info(tx: &mut Tx<'_>, info: &ReportInfo) -> Result<(), CommandError> {
    if !is_date_or_empty(&info.report_date) {
        return Err(CommandError::InvalidDate(info.report_date.clone()));
    }
    if tx.project.report_info != *info {
        tx.push(Change::ReportInfoChanged {
            before: tx.project.report_info.clone(),
            after: info.clone(),
        })?;
    }
    Ok(())
}

fn update_revision_label(
    tx: &mut Tx<'_>,
    revision: RevisionId,
    label: &str,
) -> Result<(), CommandError> {
    let current = tx
        .project
        .revisions
        .iter()
        .find(|r| r.id == revision)
        .ok_or(CommandError::UnknownRevision(revision))?;
    let label = label.trim();
    if current.label != label {
        tx.push(Change::RevisionLabelChanged {
            revision,
            before: current.label.clone(),
            after: label.to_owned(),
        })?;
    }
    Ok(())
}

fn record_issued_report(
    tx: &mut Tx<'_>,
    kind: ReportKind,
    template: &TemplateRef,
    format: ReportFormat,
    output_sha256: &Sha256Hex,
    env: &mut dyn Environment,
) -> Result<(), CommandError> {
    let revision = tx
        .project
        .current_revision()
        .ok_or(CommandError::UnknownRevision(tx.project.current_revision))?;
    let report = IssuedReport {
        kind,
        template: template.clone(),
        revision: revision.id,
        drawing_sha256: revision.sha256.clone(),
        exported_at: env.now(),
        output_sha256: output_sha256.clone(),
        format,
    };
    tx.push(Change::IssuedReportInserted {
        index: index_u32(tx.project.issued_reports.len()),
        report,
    })
}
