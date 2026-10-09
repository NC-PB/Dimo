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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::Balloon;
use crate::characteristic::Characteristic;
use crate::id::CharId;
use crate::project::{Numbering, Project, ProjectInfo, ProjectSettings};
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

fn position(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

impl Project {
    /// Applies one change after checking that it fits: the `before` state must match, IDs must
    /// be unique, and balloons must refer to existing characteristics and sheets.
    /// On error the project is unchanged.
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
        }
        Ok(())
    }

    fn reorder(&mut self, before: &[CharId], after: &[CharId]) -> Result<(), ChangeError> {
        let current: Vec<CharId> = self.characteristics.iter().map(|c| c.id).collect();
        ensure(current == before, "order differs from before")?;
        let mut sorted_before = before.to_vec();
        let mut sorted_after = after.to_vec();
        sorted_before.sort_unstable();
        sorted_after.sort_unstable();
        ensure(sorted_before == sorted_after, "new order is no permutation")?;
        let mut old = std::mem::take(&mut self.characteristics);
        for id in after {
            // Present: `after` is a permutation of the current IDs.
            if let Some(i) = old.iter().position(|c| c.id == *id) {
                self.characteristics.push(old.swap_remove(i));
            }
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
