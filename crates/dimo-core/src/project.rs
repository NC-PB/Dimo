//! The project: the root of everything stored in `project.json` (data model 07).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::{Balloon, BalloonStyle};
use crate::characteristic::Characteristic;
use crate::decimal::decimal_schema;
use crate::derivation::TableRef;
use crate::env::Timestamp;
use crate::id::{BalloonId, CharId, RevisionId, SheetId};
use crate::number::DisplayNumber;
use crate::sheet::{DrawingRevision, Sha256Hex, Sheet};

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
    /// How characteristics are numbered (D-21, D-22, D-23).
    pub numbering: NumberingSettings,
    /// Tolerance rules of the project (M2 decision 2, FR-TOL-06, FR-TOL-09).
    pub tolerance: ToleranceSettings,
}

/// Order in which characteristics are numbered (FR-BAL-04, D-21). The last strategy applied with
/// [`Command::ApplyNumbering`](crate::command::Command::ApplyNumbering); the rules are in
/// [`numbering`](crate::numbering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum NumberingStrategy {
    /// Sheet, then zone, then reading order inside the zone (top to bottom, left to right).
    /// Without a zone grid, reading order on the sheet. Default for new projects (D-21).
    #[default]
    SheetZone,
    /// View by view, reading order inside each view.
    View,
    /// View by view, clockwise around the view center starting at 12 o'clock.
    ViewClockwise,
    /// Grouped by characteristic kind.
    Kind,
    /// The current placement order; the user reorders by hand.
    Manual,
}

/// How a callout for several features (`4X`, `2 PL`) is numbered (FR-BAL-07, D-22).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MultiInstance {
    /// One balloon with the quantity (default, D-22).
    #[default]
    Quantity,
    /// One number per feature as sub-numbers, `5.1`, `5.2`.
    SubNumber,
}

/// Number given to a characteristic added while numbering is locked (FR-BAL-11, D-23).
///
/// Sub-numbers and letters follow the characteristic the new one is inserted after (the
/// anchor). Numbers given while locked are never given again, even after a delete, so an issued
/// number never changes meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum InsertPolicy {
    /// One more than the highest base number ever given: after `12` comes `13` (default).
    #[default]
    NextFree,
    /// Next sub-number of the anchor's base number: after `12` comes `12.1`, then `12.2`.
    SubNumber,
    /// Next letter of the anchor's number: after `12` comes `12A`, then `12B`.
    LetterSuffix,
}

/// Numbering settings of a project (FR-BAL-04, FR-BAL-07, FR-BAL-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct NumberingSettings {
    /// Numbering strategy (D-21).
    pub strategy: NumberingStrategy,
    /// Numbering of multi-instance callouts (D-22).
    pub multi_instance: MultiInstance,
    /// How characteristics added while numbering is locked are numbered (D-23).
    pub insert_when_locked: InsertPolicy,
}

/// A range table and its class column, e.g. ISO 2768-1 class `m` (FR-TOL-02, M2 decision 2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct TableClass {
    /// The table, shipped or custom.
    pub table: TableRef,
    /// The class column, e.g. `m`.
    pub class: String,
}

/// Decimal place rule: a nominal written with `places` decimals gets `±tolerance` (FR-TOL-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct DecimalPlaceRule {
    /// Decimal places of the nominal, 0 to 10.
    pub places: u32,
    /// Symmetric tolerance, positive.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub tolerance: Decimal,
}

/// Rounding of values converted between mm and inch (FR-TOL-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct UnitRounding {
    /// Decimal places of values converted to mm, 0 to 10.
    pub mm_places: u32,
    /// Decimal places of values converted to inch, 0 to 10.
    pub inch_places: u32,
}

impl UnitRounding {
    /// Most decimal places allowed.
    pub const MAX_PLACES: u32 = 10;
}

impl Default for UnitRounding {
    /// 0.001 mm and 0.0001 in.
    fn default() -> Self {
        Self {
            mm_places: 3,
            inch_places: 4,
        }
    }
}

/// A custom tolerance table stored in the project container as `tolerances/<id>.toml`
/// (FR-TOL-07, M2 decision 4).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct CustomTable {
    /// Table id and version as in the file.
    pub table: TableRef,
    /// SHA-256 of the stored file, checked when the project is opened.
    pub sha256: Sha256Hex,
}

/// True if `id` is a tolerance table id: lowercase letters and digits in groups joined by `-`,
/// as `TABLE_ID_PATTERN` of `dimo-tolerance`. Such an id is also a safe file name.
pub fn is_table_id(id: &str) -> bool {
    id.len() <= 128
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Tolerance settings of a project (M2 decision 2, FR-TOL-01, FR-TOL-06, FR-TOL-09).
///
/// New projects have no general tolerance, so untoleranced dimensions get "no tolerance
/// defined" until the user sets one.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ToleranceSettings {
    /// General tolerance standard and class (FR-TOL-02).
    pub general: Option<TableClass>,
    /// Custom table of the project that applies as drawing specific rule, above the general
    /// tolerance (FR-TOL-01 level 3).
    pub drawing_rule: Option<TableClass>,
    /// Decimal place rules, sorted by places, at most one per number of places (FR-TOL-06).
    pub decimal_rules: Vec<DecimalPlaceRule>,
    /// Rounding of unit conversions (FR-TOL-09).
    pub unit_rounding: UnitRounding,
    /// Custom tables stored in the project, sorted by id, ids unique (M2 decision 4).
    pub custom_tables: Vec<CustomTable>,
}

impl ToleranceSettings {
    /// Checks the rules listed on the fields. Returns what is wrong.
    pub fn validate(&self) -> Result<(), &'static str> {
        let places_ok = |p: u32| p <= UnitRounding::MAX_PLACES;
        if !self
            .decimal_rules
            .iter()
            .all(|r| places_ok(r.places) && r.tolerance > Decimal::ZERO)
        {
            return Err("decimal rules need 0 to 10 places and a positive tolerance");
        }
        if !self
            .decimal_rules
            .windows(2)
            .all(|w| w[0].places < w[1].places)
        {
            return Err("decimal rules must be sorted by places, one per number of places");
        }
        if !places_ok(self.unit_rounding.mm_places) || !places_ok(self.unit_rounding.inch_places) {
            return Err("unit rounding allows 0 to 10 places");
        }
        if !self.custom_tables.iter().all(|t| is_table_id(&t.table.id)) {
            return Err("custom table ids use lowercase letters, digits and '-'");
        }
        if !self
            .custom_tables
            .windows(2)
            .all(|w| w[0].table.id < w[1].table.id)
        {
            return Err("custom tables must be sorted by id, ids unique");
        }
        let mut hashes: Vec<&Sha256Hex> = self.custom_tables.iter().map(|t| &t.sha256).collect();
        hashes.sort();
        hashes.dedup();
        if hashes.len() != self.custom_tables.len() {
            return Err("two custom tables cannot have the same file");
        }
        for class in [&self.general, &self.drawing_rule].into_iter().flatten() {
            if !is_table_id(&class.table.id) || class.class.trim().is_empty() {
                return Err("a table setting needs a table id and a class");
            }
        }
        if let Some(rule) = &self.drawing_rule
            && !self.custom_tables.iter().any(|t| t.table == rule.table)
        {
            return Err("the drawing rule must be a custom table of the project");
        }
        Ok(())
    }
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
    /// Highest base number given so far, including numbers of deleted characteristics.
    pub highest_number: u32,
    /// Sub-numbered and lettered numbers present when locking (D-22) or given while locked,
    /// including those of deleted characteristics, sorted (FR-BAL-11).
    #[cfg_attr(feature = "specta", specta(type = Vec<String>))]
    pub given: Vec<DisplayNumber>,
}

/// Numbering state of the project (D-21, D-23).
///
/// Unlocked: numbers always follow the placement order, `1..=n` with sub-numbers for
/// multi-instance groups when the project uses them (D-22), and every add, delete or move
/// renumbers. Locked: numbers never change, deletions leave gaps, moves are refused and added
/// characteristics are numbered by [`NumberingSettings::insert_when_locked`].
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

#[cfg(test)]
mod tests {
    use super::*;

    fn table(id: &str) -> CustomTable {
        CustomTable {
            table: TableRef {
                id: id.into(),
                version: 1,
            },
            sha256: Sha256Hex::from_bytes(&[u8::try_from(id.len()).unwrap(); 32]),
        }
    }

    #[test]
    fn table_ids_are_lowercase_groups() {
        for id in ["iso-2768-1", "shop", "a1-b2"] {
            assert!(is_table_id(id), "{id}");
        }
        for id in ["", "-a", "a-", "a--b", "A", "a_b", "a/b", "../x", "a.toml"] {
            assert!(!is_table_id(id), "{id}");
        }
    }

    #[test]
    fn tolerance_settings_validation() {
        let ok = ToleranceSettings {
            general: Some(TableClass {
                table: TableRef {
                    id: "iso-2768-1".into(),
                    version: 1,
                },
                class: "m".into(),
            }),
            drawing_rule: Some(TableClass {
                table: table("shop").table,
                class: "a".into(),
            }),
            decimal_rules: vec![
                DecimalPlaceRule {
                    places: 1,
                    tolerance: Decimal::new(1, 1),
                },
                DecimalPlaceRule {
                    places: 2,
                    tolerance: Decimal::new(5, 2),
                },
            ],
            unit_rounding: UnitRounding::default(),
            custom_tables: vec![table("a"), table("shop")],
        };
        assert_eq!(ok.validate(), Ok(()));
        let broken: [fn(&mut ToleranceSettings); 8] = [
            |s| s.custom_tables[1].sha256 = s.custom_tables[0].sha256.clone(),
            |s| s.decimal_rules.reverse(),
            |s| s.decimal_rules[0].tolerance = Decimal::ZERO,
            |s| s.decimal_rules[1].places = 11,
            |s| s.unit_rounding.mm_places = 11,
            |s| s.custom_tables.reverse(),
            |s| s.custom_tables.truncate(1),
            |s| s.general.as_mut().unwrap().class = " ".into(),
        ];
        for (i, edit) in broken.iter().enumerate() {
            let mut settings = ok.clone();
            edit(&mut settings);
            assert!(settings.validate().is_err(), "case {i}");
        }
    }
}
