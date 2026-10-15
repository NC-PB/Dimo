//! Commands: the only way to change a project (05 Architecture, principle 1, ADR 0001).
//!
//! A [`Command`] says what the user wants. [`execute`] checks it against the project and
//! compiles it into primitive [`Change`]s, applying each one as it goes. If any step fails,
//! the changes made so far are rolled back and the project is unchanged.
//!
//! Handlers live in one submodule per area (characteristics, balloons, sheets, numbering,
//! settings, measurement, reports). Numbering rules (D-21, D-23) live in
//! `command::numbering`: see [`Numbering`](crate::project::Numbering).

mod balloons;
mod characteristics;
mod measurement;
mod numbering;
mod reject;
mod reports;
mod settings;
mod sheets;

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::{BalloonStyle, BalloonStyleOverride};
use crate::characteristic::{FieldError, FieldValue, Unit};
use crate::env::Environment;
use crate::geometry::{OrientedBox, Point};
use crate::id::{BalloonId, CharId, RevisionId, SampleId, SheetId};
use crate::measurement::{
    MAX_SAMPLES, MAX_SAMPLES_PER_ADD, ResultCell, ResultEntry, ResultParseError, ResultType,
    Sample, SerialError,
};
use crate::patch::{Change, ChangeError, invert};
use crate::project::{
    LockReason, NumberingSettings, NumberingStrategy, Project, ProjectInfo, ToleranceSettings,
};
use crate::proposal::{BalloonPlacement, Proposal};
use crate::report::{ReportFormat, ReportInfo, ReportKind, TemplateRef};
use crate::sheet::{Rotation, Scale, Sha256Hex, SheetView, ZoneGrid};
use crate::source::ImageEdit;

pub use reject::RejectReason;

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
    /// characteristic has results of the other type, and for `auto` on a characteristic that
    /// has results (owner decision 2026-10-12); nothing is converted. Clear the results first.
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
    /// Replaces the image edit settings of one page of an image drawing revision: rotation,
    /// fine angle, corners, trim, paper format and clean up (R01 decisions 8 to 11, MF F1).
    /// Refused for a PDF revision. Sheets and balloons do not change yet; the sheet size and
    /// balloon positions that follow from an edit are derived in Rust when the image editor
    /// comes (R.20, rule 2), never in the frontend.
    SetImageEdit {
        /// The revision.
        revision: RevisionId,
        /// Zero based page, the index of its sheet.
        page: u32,
        /// New settings.
        edit: Box<ImageEdit>,
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
    /// The revision is not an image, so it has no image edit settings (MF F1).
    #[error("drawing revision {0} is not an image")]
    NotAnImage(RevisionId),
    /// Image edit settings out of range.
    #[error("invalid image edit: {0}")]
    InvalidImageEdit(&'static str),
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

/// Sends each command to its handler. Handlers live in one submodule per area.
fn run(tx: &mut Tx<'_>, command: &Command, env: &mut dyn Environment) -> Result<(), CommandError> {
    use Command as C;
    match command {
        C::AddCharacteristic {
            sheet,
            position,
            anchor,
            region,
            values,
            insert_after,
        } => {
            let placement = BalloonPlacement {
                position: *position,
                anchor: *anchor,
            };
            let region = region.as_ref();
            characteristics::add(tx, *sheet, placement, region, values, *insert_after, env)
        }
        C::AcceptProposals {
            proposals,
            insert_after,
        } => characteristics::accept_proposals(tx, proposals, *insert_after, env),
        C::UpdateFields { ids, values } => characteristics::update_fields(tx, ids, values),
        C::DeleteCharacteristics { ids } => characteristics::delete(tx, ids),
        C::MoveCharacteristics { ids, before } => characteristics::move_to(tx, ids, *before),
        C::MoveBalloons { moves } => balloons::move_balloons(tx, moves),
        C::RestyleBalloons { ids, style } => balloons::restyle(tx, ids, style),
        C::ResetBalloonStyle { ids } => balloons::reset_style(tx, ids),
        C::UpdateSheet {
            sheet,
            rotation,
            unit,
            scale,
        } => sheets::update(tx, *sheet, *rotation, *unit, *scale),
        C::SetZoneGrid { sheet, grid } => sheets::set_zone_grid(tx, *sheet, grid.as_ref()),
        C::SetViews { sheet, views } => sheets::set_views(tx, *sheet, views),
        C::SetNumberingSettings { settings } => numbering::set_settings(tx, *settings),
        C::ApplyNumbering { strategy } => numbering::apply(tx, *strategy),
        C::LockNumbering { reason } => numbering::lock(tx, *reason, env),
        C::UnlockNumbering => numbering::unlock(tx),
        C::SetToleranceSettings { settings } => settings::set_tolerance(tx, settings),
        C::SetDefaultBalloonStyle { style } => settings::set_default_balloon_style(tx, style),
        C::UpdateProjectInfo { info } => settings::update_project_info(tx, info),
        C::AddSamples { count } => measurement::add_samples(tx, *count, env),
        C::UpdateSample { sample } => measurement::update_sample(tx, sample),
        C::DeleteSamples { ids } => measurement::delete_samples(tx, ids),
        C::MoveSamples { ids, before } => measurement::move_samples(tx, ids, *before),
        C::SetResults { results } => measurement::set_results(tx, results),
        C::ClearResults { cells } => measurement::clear_results(tx, cells),
        C::SetResultType { ids, result_type } => {
            measurement::set_result_type(tx, ids, *result_type)
        }
        C::UpdateReportInfo { info } => reports::update_report_info(tx, info),
        C::UpdateRevisionLabel { revision, label } => {
            reports::update_revision_label(tx, *revision, label)
        }
        C::SetImageEdit {
            revision,
            page,
            edit,
        } => reports::set_image_edit(tx, *revision, *page, edit),
        C::RecordIssuedReport {
            kind,
            template,
            format,
            output_sha256,
        } => reports::record_issued_report(tx, *kind, template, *format, output_sha256, env),
        C::Batch { commands } => commands.iter().try_for_each(|c| run(tx, c, env)),
    }
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn index_u32(index: usize) -> u32 {
    // Projects never hold more than u32::MAX items; saturate instead of panicking.
    u32::try_from(index).unwrap_or(u32::MAX)
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
