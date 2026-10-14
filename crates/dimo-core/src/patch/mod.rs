//! Patches: what a command changed (05 Architecture, principle 1).
//!
//! Every command is compiled into a list of primitive [`Change`]s. Each change carries the
//! state before and after, so
//!
//! - the frontend applies a [`Patch`] to its view store (inserted, removed or replaced items),
//! - undo is the inverted changes in reverse order ([`Change::inverse`]),
//! - an audit entry records the changes as "before and after" (data model `AuditEntry`),
//! - a journal of changes can be replayed onto a saved project ([`Project::apply_change`]).
//!
//! Indexes are positions in the project lists at the moment the change is applied.

mod characteristics;
mod measurement;
mod revisions;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::Balloon;
use crate::characteristic::Characteristic;
use crate::id::{CharId, RevisionId, SampleId};
use crate::measurement::{MeasurementResult, ResultCell, Sample};
use crate::project::{Numbering, Project, ProjectInfo, ProjectSettings};
use crate::report::{IssuedReport, ReportInfo};
use crate::sheet::Sheet;

/// One primitive change of a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    /// A characteristic was inserted at `index` of `Project::characteristics`.
    CharacteristicInserted {
        /// Position in the placement order.
        index: u32,
        /// The new characteristic.
        characteristic: Box<Characteristic>,
    },
    /// The characteristic at `index` was removed.
    CharacteristicRemoved {
        /// Position before removal.
        index: u32,
        /// The removed characteristic.
        characteristic: Box<Characteristic>,
    },
    /// Fields or the number of a characteristic changed. The ID stays the same.
    CharacteristicChanged {
        /// State before.
        before: Box<Characteristic>,
        /// State after.
        after: Box<Characteristic>,
    },
    /// A balloon was inserted at `index` of `Project::balloons`.
    BalloonInserted {
        /// Position in the balloon list.
        index: u32,
        /// The new balloon.
        balloon: Box<Balloon>,
    },
    /// The balloon at `index` was removed.
    BalloonRemoved {
        /// Position before removal.
        index: u32,
        /// The removed balloon.
        balloon: Box<Balloon>,
    },
    /// Position, anchor or style of a balloon changed. The ID stays the same.
    BalloonChanged {
        /// State before.
        before: Box<Balloon>,
        /// State after.
        after: Box<Balloon>,
    },
    /// The placement order of characteristics changed (same IDs, new order).
    OrderChanged {
        /// Characteristic IDs in the order before.
        before: Vec<CharId>,
        /// Characteristic IDs in the order after.
        after: Vec<CharId>,
    },
    /// Rotation, unit or scale of a sheet changed.
    SheetChanged {
        /// State before.
        before: Box<Sheet>,
        /// State after.
        after: Box<Sheet>,
    },
    /// The numbering lock changed.
    NumberingChanged {
        /// State before.
        before: Numbering,
        /// State after.
        after: Numbering,
    },
    /// Project settings changed.
    SettingsChanged {
        /// State before.
        before: Box<ProjectSettings>,
        /// State after.
        after: Box<ProjectSettings>,
    },
    /// Part and order data changed.
    InfoChanged {
        /// State before.
        before: ProjectInfo,
        /// State after.
        after: ProjectInfo,
    },
    /// A sample was inserted at `index` of `Project::samples`.
    SampleInserted {
        /// Position in the sample order.
        index: u32,
        /// The new sample.
        sample: Sample,
    },
    /// The sample at `index` was removed. It has no results left.
    SampleRemoved {
        /// Position before removal.
        index: u32,
        /// The removed sample.
        sample: Sample,
    },
    /// Serial, date, inspector, gauges or comment of a sample changed. The ID stays the same.
    SampleChanged {
        /// State before.
        before: Sample,
        /// State after.
        after: Sample,
    },
    /// The sample order changed (same IDs, new order). Results follow the new order.
    SampleOrderChanged {
        /// Sample IDs in the order before.
        before: Vec<SampleId>,
        /// Sample IDs in the order after.
        after: Vec<SampleId>,
    },
    /// The result of one cell was set, changed or cleared. `Project::results` stays sorted by
    /// sample and placement order, so the position follows from the cell.
    ResultChanged {
        /// The cell.
        cell: ResultCell,
        /// State before; `null` if the cell had no result.
        before: Option<MeasurementResult>,
        /// State after; `null` if the result was cleared.
        after: Option<MeasurementResult>,
    },
    /// Report header and approval data changed.
    ReportInfoChanged {
        /// State before.
        before: ReportInfo,
        /// State after.
        after: ReportInfo,
    },
    /// The label of a drawing revision changed.
    RevisionLabelChanged {
        /// The revision.
        revision: RevisionId,
        /// Label before.
        before: String,
        /// Label after.
        after: String,
    },
    /// An issued report record was inserted at `index` of `Project::issued_reports`.
    IssuedReportInserted {
        /// Position in the list.
        index: u32,
        /// The record.
        report: IssuedReport,
    },
    /// The issued report record at `index` was removed.
    IssuedReportRemoved {
        /// Position before removal.
        index: u32,
        /// The record.
        report: IssuedReport,
    },
}

/// Builds [`Change::inverse`]: insert and remove variants swap into each other with the same
/// fields; before and after variants swap `before` and `after` and keep the other fields. Every
/// variant must be listed, the match stays exhaustive.
macro_rules! inverse_of {
    (
        $value:expr;
        insert_remove: [$($insert:ident <=> $remove:ident { $($field:ident),+ }),+ $(,)?];
        before_after: [$($changed:ident $({ $($keep:ident),+ })?),+ $(,)?];
    ) => {
        match $value {
            $(
                Change::$insert { $($field),+ } => Change::$remove { $($field),+ },
                Change::$remove { $($field),+ } => Change::$insert { $($field),+ },
            )+
            $(
                Change::$changed { $($($keep,)+)? before, after } => Change::$changed {
                    $($($keep,)+)?
                    before: after,
                    after: before,
                },
            )+
        }
    };
}

impl Change {
    /// The change that undoes this one.
    #[must_use]
    pub fn inverse(&self) -> Self {
        inverse_of! {
            self.clone();
            insert_remove: [
                CharacteristicInserted <=> CharacteristicRemoved { index, characteristic },
                BalloonInserted <=> BalloonRemoved { index, balloon },
                SampleInserted <=> SampleRemoved { index, sample },
                IssuedReportInserted <=> IssuedReportRemoved { index, report },
            ];
            before_after: [
                CharacteristicChanged,
                BalloonChanged,
                OrderChanged,
                SheetChanged,
                NumberingChanged,
                SettingsChanged,
                InfoChanged,
                SampleChanged,
                SampleOrderChanged,
                ResultChanged { cell },
                ReportInfoChanged,
                RevisionLabelChanged { revision },
            ];
        }
    }
}

/// Inverse of a list of changes: each change inverted, in reverse order.
pub fn invert(changes: &[Change]) -> Vec<Change> {
    changes.iter().rev().map(Change::inverse).collect()
}

/// What one command, undo or redo changed. Sent to the frontend after every command.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Patch {
    /// Changes in the order they were applied. Empty if nothing changed.
    pub changes: Vec<Change>,
}

impl Patch {
    /// True if nothing changed.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// A change that does not fit the current project state.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("change does not fit the project: {0}")]
pub struct ChangeError(pub &'static str);

fn ensure(ok: bool, message: &'static str) -> Result<(), ChangeError> {
    if ok {
        Ok(())
    } else {
        Err(ChangeError(message))
    }
}

fn is_permutation<T: Ord + Clone>(a: &[T], b: &[T]) -> bool {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

fn position(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

impl Project {
    /// Applies one change after checking that it fits: the `before` state must match, IDs must
    /// be unique, balloons must refer to existing characteristics and sheets, results to
    /// existing samples and characteristics, and serials must be unique.
    /// On error the project is unchanged.
    pub fn apply_change(&mut self, change: &Change) -> Result<(), ChangeError> {
        use Change as C;
        match change {
            C::CharacteristicInserted {
                index,
                characteristic,
            } => self.insert_characteristic(position(*index), characteristic),
            C::CharacteristicRemoved {
                index,
                characteristic,
            } => self.remove_characteristic(position(*index), characteristic),
            C::CharacteristicChanged { before, after } => self.change_characteristic(before, after),
            C::BalloonInserted { index, balloon } => self.insert_balloon(position(*index), balloon),
            C::BalloonRemoved { index, balloon } => self.remove_balloon(position(*index), balloon),
            C::BalloonChanged { before, after } => self.change_balloon(before, after),
            C::OrderChanged { before, after } => self.reorder(before, after),
            C::SheetChanged { before, after } => self.change_sheet(before, after),
            C::NumberingChanged { before, after } => replace(
                &mut self.numbering,
                before,
                after,
                "numbering differs from before",
            ),
            C::SettingsChanged { before, after } => replace(
                &mut self.settings,
                before,
                after,
                "settings differ from before",
            ),
            C::InfoChanged { before, after } => {
                replace(&mut self.info, before, after, "info differs from before")
            }
            C::SampleInserted { index, sample } => self.insert_sample(position(*index), sample),
            C::SampleRemoved { index, sample } => self.remove_sample(position(*index), sample),
            C::SampleChanged { before, after } => self.change_sample(before, after),
            C::SampleOrderChanged { before, after } => self.reorder_samples(before, after),
            C::ResultChanged {
                cell,
                before,
                after,
            } => self.change_result(*cell, before.as_ref(), after.as_ref()),
            C::ReportInfoChanged { before, after } => replace(
                &mut self.report_info,
                before,
                after,
                "report info differs from before",
            ),
            C::RevisionLabelChanged {
                revision,
                before,
                after,
            } => self.change_revision_label(*revision, before, after),
            C::IssuedReportInserted { index, report } => {
                self.insert_issued_report(position(*index), report)
            }
            C::IssuedReportRemoved { index, report } => {
                self.remove_issued_report(position(*index), report)
            }
        }
    }
}

/// Replaces a whole value after checking that it equals `before`.
fn replace<T: PartialEq + Clone>(
    slot: &mut T,
    before: &T,
    after: &T,
    message: &'static str,
) -> Result<(), ChangeError> {
    ensure(*slot == *before, message)?;
    slot.clone_from(after);
    Ok(())
}
