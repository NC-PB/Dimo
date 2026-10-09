//! The project: the root of everything stored in `project.json` (data model 07).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::{Balloon, BalloonStyle};
use crate::characteristic::Characteristic;
use crate::env::Timestamp;
use crate::id::{BalloonId, CharId, RevisionId, SheetId};
use crate::sheet::{DrawingRevision, Sheet};

/// Descriptive data of the inspected part. Empty text means not set.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ProjectInfo {
    /// Part number.
    pub part_number: String,
    /// Part name.
    pub part_name: String,
    /// Customer.
    pub customer: String,
    /// Order or purchase reference.
    pub order_reference: String,
}

/// Project wide settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ProjectSettings {
    /// Default balloon style (D-24).
    pub balloon_style: BalloonStyle,
}

/// Why numbering was locked (FR-BAL-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum LockReason {
    /// The user locked it.
    Manual,
    /// A report was exported as issued (D-23).
    IssuedReport,
}

/// Number given to a characteristic added while numbering is locked (D-23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum InsertPolicy {
    /// One more than the highest number ever given since the lock. Deleted numbers are not
    /// reused, so an issued number never changes meaning.
    #[default]
    NextFree,
}

/// State of a numbering lock (FR-BAL-10, D-23).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct NumberingLock {
    /// Why the numbering was locked.
    pub reason: LockReason,
    /// When it was locked.
    pub locked_at: Timestamp,
    /// Who locked it (D-27).
    pub locked_by: String,
    /// How added characteristics are numbered while locked.
    pub insert_policy: InsertPolicy,
    /// Highest number given so far, including numbers of deleted characteristics.
    pub highest_number: u32,
}

/// Numbering state of the project (D-21, D-23).
///
/// Unlocked: numbers are always `1..=n` in placement order, and every add, delete or move
/// renumbers. Locked: numbers never change, deletions leave gaps, moves are refused and
/// added characteristics get the next free number.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Numbering {
    /// The lock, if numbering is locked.
    pub lock: Option<NumberingLock>,
}

/// A Dimo project (data model `Project`).
///
/// Change it only through [`Document::execute`](crate::document::Document::execute), so every
/// change is undoable and audited (ADR 0001, NFR-REL-02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct Project {
    /// Part and order data.
    pub info: ProjectInfo,
    /// Imported drawing files, oldest first. Old revisions stay for traceability.
    pub revisions: Vec<DrawingRevision>,
    /// The revision that is ballooned.
    pub current_revision: RevisionId,
    /// Characteristics in placement order (D-21).
    pub characteristics: Vec<Characteristic>,
    /// Balloons in placement order.
    pub balloons: Vec<Balloon>,
    /// Numbering state.
    pub numbering: Numbering,
    /// Project settings.
    pub settings: ProjectSettings,
}

impl Project {
    /// A new project for one imported drawing, without characteristics, numbering unlocked,
    /// default settings.
    pub fn new(info: ProjectInfo, revision: DrawingRevision) -> Self {
        Self {
            info,
            current_revision: revision.id,
            revisions: vec![revision],
            characteristics: Vec::new(),
            balloons: Vec::new(),
            numbering: Numbering::default(),
            settings: ProjectSettings::default(),
        }
    }

    /// The characteristic with this ID.
    pub fn characteristic(&self, id: CharId) -> Option<&Characteristic> {
        self.characteristics.iter().find(|c| c.id == id)
    }

    /// The balloon with this ID.
    pub fn balloon(&self, id: BalloonId) -> Option<&Balloon> {
        self.balloons.iter().find(|b| b.id == id)
    }

    /// Balloons of one characteristic, in placement order.
    pub fn balloons_of(&self, id: CharId) -> impl Iterator<Item = &Balloon> {
        self.balloons.iter().filter(move |b| b.characteristic == id)
    }

    /// The sheet with this ID, in any revision.
    pub fn sheet(&self, id: SheetId) -> Option<&Sheet> {
        self.revisions
            .iter()
            .flat_map(|r| &r.sheets)
            .find(|s| s.id == id)
    }

    /// The revision that is ballooned.
    pub fn current_revision(&self) -> Option<&DrawingRevision> {
        self.revisions
            .iter()
            .find(|r| r.id == self.current_revision)
    }

    /// True if numbering is locked.
    pub fn is_numbering_locked(&self) -> bool {
        self.numbering.lock.is_some()
    }
}
