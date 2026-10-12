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

use std::collections::HashMap;

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

impl Change {
    /// The change that undoes this one.
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self.clone() {
            Self::CharacteristicInserted {
                index,
                characteristic,
            } => Self::CharacteristicRemoved {
                index,
                characteristic,
            },
            Self::CharacteristicRemoved {
                index,
                characteristic,
            } => Self::CharacteristicInserted {
                index,
                characteristic,
            },
            Self::CharacteristicChanged { before, after } => Self::CharacteristicChanged {
                before: after,
                after: before,
            },
            Self::BalloonInserted { index, balloon } => Self::BalloonRemoved { index, balloon },
            Self::BalloonRemoved { index, balloon } => Self::BalloonInserted { index, balloon },
            Self::BalloonChanged { before, after } => Self::BalloonChanged {
                before: after,
                after: before,
            },
            Self::OrderChanged { before, after } => Self::OrderChanged {
                before: after,
                after: before,
            },
            Self::SheetChanged { before, after } => Self::SheetChanged {
                before: after,
                after: before,
            },
            Self::NumberingChanged { before, after } => Self::NumberingChanged {
                before: after,
                after: before,
            },
            Self::SettingsChanged { before, after } => Self::SettingsChanged {
                before: after,
                after: before,
            },
            Self::InfoChanged { before, after } => Self::InfoChanged {
                before: after,
                after: before,
            },
            Self::SampleInserted { index, sample } => Self::SampleRemoved { index, sample },
            Self::SampleRemoved { index, sample } => Self::SampleInserted { index, sample },
            Self::SampleChanged { before, after } => Self::SampleChanged {
                before: after,
                after: before,
            },
            Self::SampleOrderChanged { before, after } => Self::SampleOrderChanged {
                before: after,
                after: before,
            },
            Self::ResultChanged {
                cell,
                before,
                after,
            } => Self::ResultChanged {
                cell,
                before: after,
                after: before,
            },
            Self::ReportInfoChanged { before, after } => Self::ReportInfoChanged {
                before: after,
                after: before,
            },
            Self::RevisionLabelChanged {
                revision,
                before,
                after,
            } => Self::RevisionLabelChanged {
                revision,
                before: after,
                after: before,
            },
            Self::IssuedReportInserted { index, report } => {
                Self::IssuedReportRemoved { index, report }
            }
            Self::IssuedReportRemoved { index, report } => {
                Self::IssuedReportInserted { index, report }
            }
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
    #[allow(clippy::too_many_lines, reason = "one arm per change, exhaustive")]
    pub fn apply_change(&mut self, change: &Change) -> Result<(), ChangeError> {
        match change {
            Change::CharacteristicInserted {
                index,
                characteristic,
            } => {
                let index = position(*index);
                ensure(
                    index <= self.characteristics.len(),
                    "insert index out of range",
                )?;
                ensure(
                    self.characteristic(characteristic.id).is_none(),
                    "characteristic ID exists",
                )?;
                self.characteristics
                    .insert(index, (**characteristic).clone());
            }
            Change::CharacteristicRemoved {
                index,
                characteristic,
            } => {
                let index = position(*index);
                ensure(
                    self.characteristics.get(index) == Some(characteristic),
                    "removed characteristic differs",
                )?;
                ensure(
                    self.balloons_of(characteristic.id).next().is_none(),
                    "characteristic still has balloons",
                )?;
                ensure(
                    self.results_of(characteristic.id).next().is_none(),
                    "characteristic still has results",
                )?;
                self.characteristics.remove(index);
            }
            Change::CharacteristicChanged { before, after } => {
                ensure(before.id == after.id, "characteristic ID changed")?;
                let slot = self
                    .characteristics
                    .iter_mut()
                    .find(|c| c.id == before.id)
                    .ok_or(ChangeError("unknown characteristic"))?;
                ensure(*slot == **before, "characteristic differs from before")?;
                *slot = (**after).clone();
            }
            Change::BalloonInserted { index, balloon } => {
                let index = position(*index);
                ensure(index <= self.balloons.len(), "insert index out of range")?;
                ensure(self.balloon(balloon.id).is_none(), "balloon ID exists")?;
                self.check_balloon_refs(balloon)?;
                self.balloons.insert(index, (**balloon).clone());
            }
            Change::BalloonRemoved { index, balloon } => {
                let index = position(*index);
                ensure(
                    self.balloons.get(index) == Some(balloon),
                    "removed balloon differs",
                )?;
                self.balloons.remove(index);
            }
            Change::BalloonChanged { before, after } => {
                ensure(before.id == after.id, "balloon ID changed")?;
                self.check_balloon_refs(after)?;
                let slot = self
                    .balloons
                    .iter_mut()
                    .find(|b| b.id == before.id)
                    .ok_or(ChangeError("unknown balloon"))?;
                ensure(*slot == **before, "balloon differs from before")?;
                *slot = (**after).clone();
            }
            Change::OrderChanged { before, after } => self.reorder(before, after)?,
            Change::SheetChanged { before, after } => {
                ensure(before.id == after.id, "sheet ID changed")?;
                let slot = self
                    .revisions
                    .iter_mut()
                    .flat_map(|r| &mut r.sheets)
                    .find(|s| s.id == before.id)
                    .ok_or(ChangeError("unknown sheet"))?;
                ensure(*slot == **before, "sheet differs from before")?;
                *slot = (**after).clone();
            }
            Change::NumberingChanged { before, after } => {
                ensure(self.numbering == *before, "numbering differs from before")?;
                self.numbering = after.clone();
            }
            Change::SettingsChanged { before, after } => {
                ensure(self.settings == **before, "settings differ from before")?;
                self.settings = (**after).clone();
            }
            Change::InfoChanged { before, after } => {
                ensure(self.info == *before, "info differs from before")?;
                self.info = after.clone();
            }
            change @ (Change::SampleInserted { .. }
            | Change::SampleRemoved { .. }
            | Change::SampleChanged { .. }
            | Change::SampleOrderChanged { .. }
            | Change::ResultChanged { .. }
            | Change::ReportInfoChanged { .. }
            | Change::RevisionLabelChanged { .. }
            | Change::IssuedReportInserted { .. }
            | Change::IssuedReportRemoved { .. }) => self.apply_measurement_change(change)?,
        }
        Ok(())
    }

    /// [`Project::apply_change`] for samples, results and report data (data model version 3).
    fn apply_measurement_change(&mut self, change: &Change) -> Result<(), ChangeError> {
        match change {
            Change::SampleInserted { index, sample } => {
                let index = position(*index);
                ensure(index <= self.samples.len(), "insert index out of range")?;
                ensure(self.sample(sample.id).is_none(), "sample ID exists")?;
                self.check_serial(sample)?;
                self.samples.insert(index, sample.clone());
            }
            Change::SampleRemoved { index, sample } => {
                let index = position(*index);
                ensure(
                    self.samples.get(index) == Some(sample),
                    "removed sample differs",
                )?;
                ensure(
                    !self.results.iter().any(|r| r.sample == sample.id),
                    "sample still has results",
                )?;
                self.samples.remove(index);
            }
            Change::SampleChanged { before, after } => {
                ensure(before.id == after.id, "sample ID changed")?;
                self.check_serial(after)?;
                let slot = self
                    .samples
                    .iter_mut()
                    .find(|s| s.id == before.id)
                    .ok_or(ChangeError("unknown sample"))?;
                ensure(*slot == *before, "sample differs from before")?;
                *slot = after.clone();
            }
            Change::SampleOrderChanged { before, after } => {
                let current: Vec<SampleId> = self.samples.iter().map(|s| s.id).collect();
                ensure(current == *before, "sample order differs from before")?;
                ensure(is_permutation(before, after), "new order is no permutation")?;
                let mut old = std::mem::take(&mut self.samples);
                for id in after {
                    // Present: `after` is a permutation of the current IDs.
                    if let Some(i) = old.iter().position(|s| s.id == *id) {
                        self.samples.push(old.swap_remove(i));
                    }
                }
                self.sort_results();
            }
            Change::ResultChanged {
                cell,
                before,
                after,
            } => self.change_result(*cell, before.as_ref(), after.as_ref())?,
            Change::ReportInfoChanged { before, after } => {
                ensure(
                    self.report_info == *before,
                    "report info differs from before",
                )?;
                self.report_info = after.clone();
            }
            Change::RevisionLabelChanged {
                revision,
                before,
                after,
            } => {
                let slot = self
                    .revisions
                    .iter_mut()
                    .find(|r| r.id == *revision)
                    .ok_or(ChangeError("unknown revision"))?;
                ensure(slot.label == *before, "revision label differs from before")?;
                slot.label.clone_from(after);
            }
            Change::IssuedReportInserted { index, report } => {
                let index = position(*index);
                ensure(
                    index <= self.issued_reports.len(),
                    "insert index out of range",
                )?;
                self.issued_reports.insert(index, report.clone());
            }
            Change::IssuedReportRemoved { index, report } => {
                let index = position(*index);
                ensure(
                    self.issued_reports.get(index) == Some(report),
                    "removed issued report differs",
                )?;
                self.issued_reports.remove(index);
            }
            _ => return Err(ChangeError("not a sample, result or report change")),
        }
        Ok(())
    }

    fn reorder(&mut self, before: &[CharId], after: &[CharId]) -> Result<(), ChangeError> {
        let current: Vec<CharId> = self.characteristics.iter().map(|c| c.id).collect();
        ensure(current == before, "order differs from before")?;
        ensure(is_permutation(before, after), "new order is no permutation")?;
        let mut old = std::mem::take(&mut self.characteristics);
        for id in after {
            // Present: `after` is a permutation of the current IDs.
            if let Some(i) = old.iter().position(|c| c.id == *id) {
                self.characteristics.push(old.swap_remove(i));
            }
        }
        self.sort_results();
        Ok(())
    }

    /// Serials are trimmed, not empty and unique (M3 decision 6).
    fn check_serial(&self, sample: &Sample) -> Result<(), ChangeError> {
        ensure(
            !sample.serial.is_empty() && sample.serial.trim() == sample.serial,
            "serial is empty or not trimmed",
        )?;
        ensure(
            !self
                .samples
                .iter()
                .any(|s| s.id != sample.id && s.serial == sample.serial),
            "serial exists",
        )
    }

    /// Position of every sample and characteristic, the sort key of results.
    fn result_order(&self) -> (HashMap<SampleId, usize>, HashMap<CharId, usize>) {
        let samples = self.samples.iter().enumerate().map(|(i, s)| (s.id, i));
        let chars = self
            .characteristics
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i));
        (samples.collect(), chars.collect())
    }

    /// Restores the order of results after the sample or placement order changed.
    fn sort_results(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let (samples, chars) = self.result_order();
        self.results.sort_by_key(|r| {
            (
                samples.get(&r.sample).copied().unwrap_or(usize::MAX),
                chars.get(&r.characteristic).copied().unwrap_or(usize::MAX),
            )
        });
    }

    fn change_result(
        &mut self,
        cell: ResultCell,
        before: Option<&MeasurementResult>,
        after: Option<&MeasurementResult>,
    ) -> Result<(), ChangeError> {
        ensure(
            before.is_some() || after.is_some(),
            "result change without state",
        )?;
        ensure(
            before.into_iter().chain(after).all(|r| r.cell() == cell),
            "result belongs to another cell",
        )?;
        let (samples, chars) = self.result_order();
        let key = |c: ResultCell| {
            (
                samples.get(&c.sample).copied().unwrap_or(usize::MAX),
                chars.get(&c.characteristic).copied().unwrap_or(usize::MAX),
            )
        };
        let target = key(cell);
        ensure(
            target.0 != usize::MAX && target.1 != usize::MAX,
            "result of an unknown sample or characteristic",
        )?;
        let at = self.results.partition_point(|r| key(r.cell()) < target);
        let current = self.results.get(at).filter(|r| r.cell() == cell);
        ensure(current == before, "result differs from before")?;
        match (current.is_some(), after) {
            (true, Some(after)) => self.results[at] = after.clone(),
            (true, None) => {
                self.results.remove(at);
            }
            (false, Some(after)) => self.results.insert(at, after.clone()),
            (false, None) => {}
        }
        Ok(())
    }

    fn check_balloon_refs(&self, balloon: &Balloon) -> Result<(), ChangeError> {
        ensure(
            self.characteristic(balloon.characteristic).is_some(),
            "balloon of an unknown characteristic",
        )?;
        ensure(
            self.sheet(balloon.sheet).is_some(),
            "balloon on an unknown sheet",
        )
    }
}
