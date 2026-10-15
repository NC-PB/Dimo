//! The project: the root of everything stored in `project.json` (data model 07).

use std::collections::{BTreeSet, HashMap};

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::{Balloon, BalloonStyle};
use crate::characteristic::Characteristic;
use crate::decimal::decimal_schema;
use crate::derivation::TableRef;
use crate::env::Timestamp;
use crate::id::{BalloonId, CharId, RevisionId, SampleId, SheetId};
use crate::measurement::{MAX_SAMPLES, MeasurementResult, ResultCell, Sample};
use crate::number::DisplayNumber;
use crate::report::{IssuedReport, ReportInfo};
use crate::sheet::{DrawingRevision, Sha256Hex, Sheet};

/// The answer of [`Project::delete_check`]: the results that go with deleted characteristics.
/// Deleting characteristics with results asks the user first (M3 decision 16, FR-MEA-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DeleteCheck {
    /// Number of results of the characteristics.
    pub results: u32,
    /// Whether the deletion needs the user's confirmation.
    pub needs_confirmation: bool,
}

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

/// One field of the part data, used by
/// [`Command::UpdateProjectInfo`](crate::command::Command::UpdateProjectInfo) (MF F4).
///
/// Serialized as `{ "field": "part_number", "value": "4711" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum ProjectInfoField {
    /// Part number.
    PartNumber(String),
    /// Part name.
    PartName(String),
    /// Customer.
    Customer(String),
    /// Order or purchase reference.
    OrderReference(String),
}

impl ProjectInfo {
    /// Sets one field.
    pub fn set(&mut self, value: &ProjectInfoField) {
        let (slot, text) = match value {
            ProjectInfoField::PartNumber(t) => (&mut self.part_number, t),
            ProjectInfoField::PartName(t) => (&mut self.part_name, t),
            ProjectInfoField::Customer(t) => (&mut self.customer, t),
            ProjectInfoField::OrderReference(t) => (&mut self.order_reference, t),
        };
        slot.clone_from(text);
    }
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

/// One tolerance setting other than decimal place rules and custom tables, used by
/// [`Command::UpdateToleranceSettings`](crate::command::Command::UpdateToleranceSettings)
/// (MF F4). Decimal place rules change one at a time with
/// [`Command::AddDecimalRule`](crate::command::Command::AddDecimalRule) and
/// [`Command::RemoveDecimalRule`](crate::command::Command::RemoveDecimalRule).
///
/// Serialized as `{ "field": "mm_places", "value": 3 }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "field", content = "value", rename_all = "snake_case")]
pub enum ToleranceField {
    /// General tolerance standard and class; `null` removes it.
    General(Option<TableClass>),
    /// Drawing specific rule, a custom table of the project; `null` removes it.
    DrawingRule(Option<TableClass>),
    /// Decimal places of values converted to mm.
    MmPlaces(u32),
    /// Decimal places of values converted to inch.
    InchPlaces(u32),
}

impl ToleranceSettings {
    /// Sets one field. Call [`Self::validate`] afterwards.
    pub fn set(&mut self, value: &ToleranceField) {
        match value {
            ToleranceField::General(v) => self.general.clone_from(v),
            ToleranceField::DrawingRule(v) => self.drawing_rule.clone_from(v),
            ToleranceField::MmPlaces(v) => self.unit_rounding.mm_places = *v,
            ToleranceField::InchPlaces(v) => self.unit_rounding.inch_places = *v,
        }
    }

    /// Inserts `rule` in the order of places. A rule for places that already have one stays
    /// next to it, so [`Self::validate`] refuses the result.
    pub fn add_decimal_rule(&mut self, rule: DecimalPlaceRule) {
        let index = self
            .decimal_rules
            .partition_point(|r| r.places <= rule.places);
        self.decimal_rules.insert(index, rule);
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
    /// Measured parts; the order is the column order of the grid and the reports (FR-MEA-01).
    pub samples: Vec<Sample>,
    /// Measurement results, at most one per sample and characteristic, sorted by sample order
    /// and then by characteristic placement order (M3 decision 1). Commands keep the order.
    pub results: Vec<MeasurementResult>,
    /// Report header and approval data (M3 decision 1).
    pub report_info: ReportInfo,
    /// Reports exported as issued, oldest first (D-23, M3 decision 17).
    pub issued_reports: Vec<IssuedReport>,
}

impl Project {
    /// A new project for one imported drawing, without characteristics and samples, numbering
    /// unlocked, default settings.
    pub fn new(info: ProjectInfo, revision: DrawingRevision) -> Self {
        Self {
            info,
            current_revision: revision.id,
            revisions: vec![revision],
            characteristics: Vec::new(),
            balloons: Vec::new(),
            numbering: Numbering::default(),
            settings: ProjectSettings::default(),
            samples: Vec::new(),
            results: Vec::new(),
            report_info: ReportInfo::default(),
            issued_reports: Vec::new(),
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

    /// The sample with this ID.
    pub fn sample(&self, id: SampleId) -> Option<&Sample> {
        self.samples.iter().find(|s| s.id == id)
    }

    /// The result of one sample for one characteristic.
    pub fn result(&self, cell: ResultCell) -> Option<&MeasurementResult> {
        self.results.iter().find(|r| r.cell() == cell)
    }

    /// Results of one characteristic, in sample order.
    pub fn results_of(&self, id: CharId) -> impl Iterator<Item = &MeasurementResult> {
        self.results.iter().filter(move |r| r.characteristic == id)
    }

    /// What deleting the characteristics `ids` would remove besides them (M3 decision 16):
    /// their results. Unknown IDs count nothing.
    pub fn delete_check(&self, ids: &[CharId]) -> DeleteCheck {
        let ids: BTreeSet<CharId> = ids.iter().copied().collect();
        let results = self
            .results
            .iter()
            .filter(|r| ids.contains(&r.characteristic))
            .count();
        let results = u32::try_from(results).unwrap_or(u32::MAX);
        DeleteCheck {
            results,
            needs_confirmation: results > 0,
        }
    }

    /// Checks the rules commands keep for samples and results, for a project read from a file
    /// (M3 decisions 1 and 6): at most [`MAX_SAMPLES`] samples with unique IDs and unique,
    /// trimmed, non empty serials; results of existing samples and characteristics, at most
    /// one per cell, sorted by sample order and then placement order. Returns what is wrong.
    pub fn check_measurements(&self) -> Result<(), &'static str> {
        if self.samples.len() > MAX_SAMPLES {
            return Err("more than 500 samples");
        }
        let mut ids = BTreeSet::new();
        let mut serials = BTreeSet::new();
        for s in &self.samples {
            if s.serial.is_empty() || s.serial.trim() != s.serial {
                return Err("a sample serial is empty or not trimmed");
            }
            if !ids.insert(s.id) || !serials.insert(s.serial.as_str()) {
                return Err("a sample ID or serial is used twice");
            }
        }
        let samples: HashMap<SampleId, usize> = self
            .samples
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        let chars: HashMap<CharId, usize> = self
            .characteristics
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();
        let mut previous = None;
        for r in &self.results {
            let (Some(&s), Some(&c)) = (samples.get(&r.sample), chars.get(&r.characteristic))
            else {
                return Err("a result refers to an unknown sample or characteristic");
            };
            if previous.is_some_and(|p| p >= (s, c)) {
                return Err(
                    "results are not sorted by sample and characteristic, or a cell has two",
                );
            }
            previous = Some((s, c));
        }
        Ok(())
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
            |s| s.custom_tables[1].sha256 = s.custom_tables[0].sha256,
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
