//! The open document: a project with unlimited undo and redo and an audit log (NFR-REL-02).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::command::{self, Command, CommandError};
use crate::env::{Environment, Timestamp};
use crate::patch::{Change, Patch, invert};
use crate::project::Project;

/// What an audit entry records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditAction {
    /// A command was executed.
    Command {
        /// The command as requested.
        command: Command,
    },
    /// The last executed command was undone.
    Undo {
        /// The command that was undone.
        command: Command,
    },
    /// An undone command was applied again.
    Redo {
        /// The command that was applied again.
        command: Command,
    },
}

/// One line of the audit log (data model `AuditEntry`, NFR-REL-02).
///
/// `changes` hold the state before and after each change, so the log alone explains every
/// value in the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct AuditEntry {
    /// When it happened.
    pub timestamp: Timestamp,
    /// Who did it (D-27).
    pub user: String,
    /// What was done.
    pub action: AuditAction,
    /// What changed, in application order.
    pub changes: Vec<Change>,
}

/// One undo step: a command and the changes it made.
#[derive(Debug, Clone)]
struct Step {
    id: u64,
    command: Command,
    changes: Vec<Change>,
}

/// A project being edited: applies commands, keeps unlimited undo and redo, records audit
/// entries (05 Architecture, principle 1).
#[derive(Debug, Clone)]
pub struct Document {
    project: Project,
    undo: Vec<Step>,
    redo: Vec<Step>,
    audit: Vec<AuditEntry>,
    next_step: u64,
    /// Undo step on top of the history when the project was last saved or loaded.
    saved_step: Option<u64>,
}

impl Document {
    /// Opens a project with empty history. The project counts as saved.
    pub fn new(project: Project) -> Self {
        Self {
            project,
            undo: Vec::new(),
            redo: Vec::new(),
            audit: Vec::new(),
            next_step: 0,
            saved_step: None,
        }
    }

    /// The current project state.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The project, dropping the history.
    pub fn into_project(self) -> Project {
        self.project
    }

    /// Executes a command as one undo step and records an audit entry.
    ///
    /// A command that changes nothing returns an empty patch and adds neither an undo step nor
    /// an audit entry. On error nothing changes.
    pub fn execute(
        &mut self,
        command: Command,
        env: &mut dyn Environment,
    ) -> Result<Patch, CommandError> {
        let changes = command::execute(&mut self.project, &command, env)?;
        if changes.is_empty() {
            return Ok(Patch::default());
        }
        self.record(
            env,
            AuditAction::Command {
                command: command.clone(),
            },
            &changes,
        );
        self.undo.push(Step {
            id: self.next_step,
            command,
            changes: changes.clone(),
        });
        self.next_step += 1;
        self.redo.clear();
        Ok(Patch { changes })
    }

    /// Undoes the last executed or redone command.
    pub fn undo(&mut self, env: &mut dyn Environment) -> Result<Patch, CommandError> {
        let step = self.undo.pop().ok_or(CommandError::NothingToUndo)?;
        let changes = invert(&step.changes);
        if let Err(err) = self.apply_all(&changes) {
            self.undo.push(step);
            return Err(err);
        }
        self.record(
            env,
            AuditAction::Undo {
                command: step.command.clone(),
            },
            &changes,
        );
        self.redo.push(step);
        Ok(Patch { changes })
    }

    /// Applies the last undone command again, with the same IDs and values.
    pub fn redo(&mut self, env: &mut dyn Environment) -> Result<Patch, CommandError> {
        let step = self.redo.pop().ok_or(CommandError::NothingToRedo)?;
        if let Err(err) = self.apply_all(&step.changes) {
            self.redo.push(step);
            return Err(err);
        }
        self.record(
            env,
            AuditAction::Redo {
                command: step.command.clone(),
            },
            &step.changes,
        );
        let changes = step.changes.clone();
        self.undo.push(step);
        Ok(Patch { changes })
    }

    /// True if [`Document::undo`] has a step to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True if [`Document::redo`] has a step to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The command [`Document::undo`] would undo, for menu labels.
    pub fn undo_command(&self) -> Option<&Command> {
        self.undo.last().map(|s| &s.command)
    }

    /// The command [`Document::redo`] would apply, for menu labels.
    pub fn redo_command(&self) -> Option<&Command> {
        self.redo.last().map(|s| &s.command)
    }

    /// Audit entries recorded since the document was opened or since the last
    /// [`Document::take_audit`], oldest first.
    pub fn audit(&self) -> &[AuditEntry] {
        &self.audit
    }

    /// Removes and returns the recorded audit entries, for appending to `audit.jsonl`.
    pub fn take_audit(&mut self) -> Vec<AuditEntry> {
        std::mem::take(&mut self.audit)
    }

    /// Marks the current state as saved.
    pub fn mark_saved(&mut self) {
        self.saved_step = self.undo.last().map(|s| s.id);
    }

    /// True if the project differs from the last saved state (by undo position).
    pub fn is_modified(&self) -> bool {
        self.undo.last().map(|s| s.id) != self.saved_step
    }

    fn apply_all(&mut self, changes: &[Change]) -> Result<(), CommandError> {
        for (done, change) in changes.iter().enumerate() {
            if let Err(err) = self.project.apply_change(change) {
                for applied in invert(&changes[..done]) {
                    // Cannot fail: each inverse fits the state its forward change produced.
                    let _ = self.project.apply_change(&applied);
                }
                return Err(err.into());
            }
        }
        Ok(())
    }

    fn record(&mut self, env: &mut dyn Environment, action: AuditAction, changes: &[Change]) {
        self.audit.push(AuditEntry {
            timestamp: env.now(),
            user: env.user_name(),
            action,
            changes: changes.to_vec(),
        });
    }
}
