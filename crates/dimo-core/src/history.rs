//! Change history of one characteristic, read from the audit log (FR-CHR-10, NFR-REL-02).
//!
//! Every audit entry holds the changes of one command, undo or redo with the state before and
//! after. [`characteristic_history`] picks the entries that touched one characteristic and says
//! who, when, what and by which source: by hand, by a rule (renumbering, re-interpretation), or
//! by recognition (accepted proposals).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::characteristic::{Characteristic, FieldValue};
use crate::command::Command;
use crate::derivation::DerivationRule;
use crate::document::{AuditAction, AuditEntry};
use crate::env::Timestamp;
use crate::id::CharId;
use crate::patch::Change;

/// What made a change (FR-CHR-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ChangeSource {
    /// The user changed this characteristic.
    Manual,
    /// A rule changed it as a consequence, e.g. renumbering after a delete, or limits from a
    /// tolerance rule.
    Rule,
    /// It came from recognition: an accepted proposal (ADR 0006).
    Recognition,
}

/// Whether the entry is a command, an undo or a redo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum HistoryAction {
    /// A command was executed.
    Command,
    /// A command was undone.
    Undo,
    /// An undone command was applied again.
    Redo,
}

/// A field of a characteristic, to list what changed.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CharacteristicField {
    /// Display number.
    Number,
    /// Kind.
    Kind,
    /// Requirement text.
    RequirementText,
    /// Nominal.
    Nominal,
    /// Unit.
    Unit,
    /// Upper deviation.
    UpperDev,
    /// Lower deviation.
    LowerDev,
    /// Upper limit.
    UpperLimit,
    /// Lower limit.
    LowerLimit,
    /// Fit.
    Fit,
    /// Tolerance derivation.
    Derivation,
    /// Quantity.
    Quantity,
    /// Classification.
    Classification,
    /// Inspection method, gauge, sampling or frequency.
    Inspection,
    /// Inspect flag.
    Inspect,
    /// Review state.
    Status,
    /// Origin.
    Origin,
    /// Source regions.
    Sources,
    /// Comment.
    Comment,
}

/// The fields that differ between two states of a characteristic.
pub fn changed_fields(before: &Characteristic, after: &Characteristic) -> Vec<CharacteristicField> {
    use CharacteristicField as F;
    let checks = [
        (F::Number, before.number != after.number),
        (F::Kind, before.kind != after.kind),
        (
            F::RequirementText,
            before.requirement_text != after.requirement_text,
        ),
        (F::Nominal, !same_decimal(before.nominal, after.nominal)),
        (F::Unit, before.unit != after.unit),
        (
            F::UpperDev,
            !same_decimal(before.upper_dev, after.upper_dev),
        ),
        (
            F::LowerDev,
            !same_decimal(before.lower_dev, after.lower_dev),
        ),
        (
            F::UpperLimit,
            !same_decimal(before.upper_limit, after.upper_limit),
        ),
        (
            F::LowerLimit,
            !same_decimal(before.lower_limit, after.lower_limit),
        ),
        (F::Fit, before.fit != after.fit),
        (F::Derivation, before.derivation != after.derivation),
        (F::Quantity, before.quantity != after.quantity),
        (
            F::Classification,
            before.classification != after.classification,
        ),
        (F::Inspection, before.inspection != after.inspection),
        (F::Inspect, before.inspect != after.inspect),
        (F::Status, before.status != after.status),
        (F::Origin, before.origin != after.origin),
        (F::Sources, before.sources != after.sources),
        (F::Comment, before.comment != after.comment),
    ];
    checks
        .into_iter()
        .filter_map(|(field, changed)| changed.then_some(field))
        .collect()
}

/// Equal value and written form (`8.0` and `8.00` differ for the user).
fn same_decimal(a: Option<rust_decimal::Decimal>, b: Option<rust_decimal::Decimal>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b && a.scale() == b.scale(),
        (a, b) => a.is_none() && b.is_none(),
    }
}

/// One step in the history of a characteristic (FR-CHR-10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct HistoryEntry {
    /// When.
    pub timestamp: Timestamp,
    /// Who (D-27).
    pub user: String,
    /// Command, undo or redo.
    pub action: HistoryAction,
    /// The command that was executed, undone or redone.
    pub command: Command,
    /// What made the change.
    pub source: ChangeSource,
    /// State before; `null` if the entry created the characteristic.
    pub before: Option<Box<Characteristic>>,
    /// State after; `null` if the entry removed the characteristic.
    pub after: Option<Box<Characteristic>>,
    /// Fields that changed; empty when the characteristic was created or removed.
    pub fields: Vec<CharacteristicField>,
}

/// The entries of `audit` that changed characteristic `id`, oldest first (FR-CHR-10).
pub fn characteristic_history(audit: &[AuditEntry], id: CharId) -> Vec<HistoryEntry> {
    audit
        .iter()
        .filter_map(|entry| history_entry(entry, id))
        .collect()
}

fn history_entry(entry: &AuditEntry, id: CharId) -> Option<HistoryEntry> {
    let mut first: Option<Option<&Characteristic>> = None;
    let mut last: Option<&Characteristic> = None;
    let mut inserted = false;
    for change in &entry.changes {
        let (before, after) = match change {
            Change::CharacteristicInserted { characteristic, .. } if characteristic.id == id => {
                (None, Some(&**characteristic))
            }
            Change::CharacteristicRemoved { characteristic, .. } if characteristic.id == id => {
                (Some(&**characteristic), None)
            }
            Change::CharacteristicChanged { before, after } if before.id == id => {
                (Some(&**before), Some(&**after))
            }
            _ => continue,
        };
        if first.is_none() {
            first = Some(before);
            inserted = before.is_none();
        }
        last = after;
    }
    let before = first?;
    let (action, command) = match &entry.action {
        AuditAction::Command { command } => (HistoryAction::Command, command),
        AuditAction::Undo { command } => (HistoryAction::Undo, command),
        AuditAction::Redo { command } => (HistoryAction::Redo, command),
    };
    // An undo removes what the command inserted, so the command's view of "inserted" is the
    // opposite of what this entry shows.
    let created_by_command = if action == HistoryAction::Undo {
        last.is_none()
    } else {
        inserted
    };
    let fields = match (before, last) {
        (Some(before), Some(after)) => changed_fields(before, after),
        _ => Vec::new(),
    };
    Some(HistoryEntry {
        timestamp: entry.timestamp.clone(),
        user: entry.user.clone(),
        action,
        command: command.clone(),
        source: source(command, id, created_by_command).unwrap_or(ChangeSource::Rule),
        before: before.map(|c| Box::new(c.clone())),
        after: last.map(|c| Box::new(c.clone())),
        fields,
    })
}

/// The source if `command` names characteristic `id` (or created it); `None` if `id` only
/// changed as a consequence, which is a rule.
fn source(command: &Command, id: CharId, created: bool) -> Option<ChangeSource> {
    match command {
        Command::AddCharacteristic { .. } => created.then_some(ChangeSource::Manual),
        Command::AcceptProposals { .. } => created.then_some(ChangeSource::Recognition),
        Command::UpdateFields { ids, values } if ids.contains(&id) => {
            let by_rule = values.iter().any(|v| {
                matches!(v, FieldValue::Derivation(Some(d)) if d.rule != DerivationRule::Manual)
            });
            Some(if by_rule {
                ChangeSource::Rule
            } else {
                ChangeSource::Manual
            })
        }
        Command::DeleteCharacteristics { ids } | Command::MoveCharacteristics { ids, .. }
            if ids.contains(&id) =>
        {
            Some(ChangeSource::Manual)
        }
        Command::Batch { commands } => commands.iter().find_map(|c| source(c, id, created)),
        _ => None,
    }
}
