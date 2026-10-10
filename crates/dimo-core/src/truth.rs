//! Corpus ground truth format: `corpus/truth/<name>.truth.json` (D-42, T0.10).
//!
//! A truth file states what a correct recognition of one corpus drawing returns: the sheets with
//! their kind, and every characteristic with kind, requirement text, source region, expected
//! nominal and limits, tolerance rule and inspect flag. Names and shapes follow the data model
//! (07) so that evaluation can compare proposals field by field (08, Evaluation).
//!
//! Expected limits come from what the drawing states plus the tolerance rules, never from
//! recognizer output. The JSON schema in `docs/schema/truth.schema.json` is generated from these
//! types (see [`truth_schema_json`]).
//!
//! This module is pure: it parses and checks text. Reading files and comparing the drawing hash
//! with `corpus/PROVENANCE.md` is done by the corpus tests.

use std::collections::BTreeSet;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use crate::decimal::decimal_schema;
use crate::geometry::{OrientedBox, Size};
use crate::project::ToleranceSettings;
use crate::sheet::SheetKind;

/// Version of the truth format written by this code. Increase it on any incompatible change.
pub const TRUTH_FORMAT_VERSION: u32 = 1;

const SHA256_PATTERN: &str = "^[0-9a-f]{64}$";
const FIT_PATTERN: &str = "^[A-Za-z]{1,2}[0-9]{1,2}$";
const ID_PATTERN: &str = "^[a-z0-9][a-z0-9_-]*$";

/// Ground truth for one corpus drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[schemars(title = "Dimo corpus truth file")]
pub struct TruthFile {
    /// Format version, currently `1`.
    pub format_version: u32,
    /// The drawing this truth describes.
    pub drawing: TruthDrawing,
    /// Every sheet of the drawing, in document order.
    pub sheets: Vec<TruthSheet>,
    /// Every characteristic a correct recognition proposes, in a stable order.
    pub characteristics: Vec<TruthCharacteristic>,
    /// Tolerance settings of the project that the expected limits assume (FR-TOL-01, M2
    /// decision 2). Absent means the settings of a new project: no general tolerance and no
    /// rules, so untoleranced dimensions have no limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_settings: Option<ToleranceSettings>,
    /// Free text: how the truth was made (for example how regions were measured), conventions,
    /// open questions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Identifies the drawing file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TruthDrawing {
    /// Path relative to the `corpus/` directory with `/` separators,
    /// e.g. `drawings/test_drawing_1.pdf`.
    pub file: String,
    /// SHA-256 of the file, 64 lowercase hex digits. Must match `corpus/PROVENANCE.md`.
    #[schemars(pattern(SHA256_PATTERN))]
    pub sha256: String,
}

/// One sheet of the drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TruthSheet {
    /// Zero based sheet index (page index in the PDF).
    pub index: u32,
    /// Expected sheet classification (FR-DOC-03).
    pub kind: SheetKind,
    /// Sheet size in PDF user units.
    pub size: Size,
}

/// One expected characteristic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TruthCharacteristic {
    /// Key that is unique within the truth file and stable across edits, e.g. `c07`.
    /// Not the balloon number.
    #[schemars(pattern(ID_PATTERN))]
    pub id: String,
    /// Zero based index of the sheet that holds the callout.
    pub sheet: u32,
    /// What the characteristic describes.
    pub kind: CharacteristicKind,
    /// Callout text as printed, normalized: the text objects of the callout joined with one
    /// space in reading order (main text, then upper deviation, then lower deviation), without
    /// leading, trailing or repeated spaces. Characters are kept as printed, including `Ø` and
    /// the Unicode minus U+2212.
    pub requirement_text: String,
    /// Oriented box around all text of the callout, in sheet space.
    pub region: OrientedBox,
    /// Nominal value. Absent for characteristics without a value (notes).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::decimal::serde_str_option"
    )]
    #[schemars(schema_with = "decimal_schema")]
    pub nominal: Option<Decimal>,
    /// Unit of the nominal and the limits. Present exactly when a nominal is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<Unit>,
    /// Fit designation such as `H7`, if the callout has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(pattern(FIT_PATTERN))]
    pub fit: Option<String>,
    /// Rule that produces the limits (FR-TOL-01). Present exactly when a nominal is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_rule: Option<ToleranceRule>,
    /// Expected upper limit. Present exactly when the rule yields limits.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::decimal::serde_str_option"
    )]
    #[schemars(schema_with = "decimal_schema")]
    pub upper_limit: Option<Decimal>,
    /// Expected lower limit. Present exactly when the rule yields limits.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::decimal::serde_str_option"
    )]
    #[schemars(schema_with = "decimal_schema")]
    pub lower_limit: Option<Decimal>,
    /// Whether the characteristic is inspected (false for reference and basic dimensions).
    pub inspect: bool,
    /// Why this entry is uncertain and what the owner should check. Absent when certain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_note: Option<String>,
}

/// Errors from loading a truth file.
#[derive(Debug, thiserror::Error)]
pub enum TruthError {
    /// The text is not JSON of the truth format.
    #[error("truth file does not match the truth format: {0}")]
    Parse(#[from] serde_json::Error),
    /// The file parsed but breaks consistency rules.
    #[error("truth file is inconsistent:\n  {}", .0.join("\n  "))]
    Invalid(Vec<String>),
}

impl TruthFile {
    /// Parse a truth file and check its consistency (see [`TruthFile::issues`]).
    pub fn from_json_str(text: &str) -> Result<Self, TruthError> {
        let truth: Self = serde_json::from_str(text)?;
        let issues = truth.issues();
        if issues.is_empty() {
            Ok(truth)
        } else {
            Err(TruthError::Invalid(issues))
        }
    }

    /// Consistency problems the JSON schema cannot express. Empty if the file is consistent.
    pub fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.format_version != TRUTH_FORMAT_VERSION {
            issues.push(format!(
                "format_version {} is not supported, expected {TRUTH_FORMAT_VERSION}",
                self.format_version
            ));
        }
        check_drawing(&self.drawing, &mut issues);
        check_sheets(&self.sheets, &mut issues);
        if let Some(Err(problem)) = self
            .tolerance_settings
            .as_ref()
            .map(ToleranceSettings::validate)
        {
            issues.push(format!("tolerance_settings: {problem}"));
        }
        let mut ids = BTreeSet::new();
        for c in &self.characteristics {
            if !ids.insert(c.id.as_str()) {
                issues.push(format!("{}: duplicate id", c.id));
            }
            let mut push = |message: String| issues.push(format!("{}: {message}", c.id));
            check_characteristic(c, &self.sheets, &mut push);
        }
        issues
    }
}

fn check_drawing(drawing: &TruthDrawing, issues: &mut Vec<String>) {
    let file = &drawing.file;
    if file.is_empty()
        || file.starts_with('/')
        || file.contains('\\')
        || file.split('/').any(|part| part.is_empty() || part == "..")
    {
        issues.push(format!(
            "drawing.file {file:?} must be a relative path below corpus/"
        ));
    }
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    if drawing.sha256.len() != 64 || !drawing.sha256.bytes().all(hex) {
        issues.push("drawing.sha256 must be 64 lowercase hex digits".to_string());
    }
}

fn check_sheets(sheets: &[TruthSheet], issues: &mut Vec<String>) {
    if sheets.is_empty() {
        issues.push("sheets must not be empty".to_string());
    }
    for (position, sheet) in sheets.iter().enumerate() {
        if usize::try_from(sheet.index).ok() != Some(position) {
            issues.push(format!(
                "sheets[{position}].index is {}, expected {position}",
                sheet.index
            ));
        }
        if !(positive(sheet.size.width) && positive(sheet.size.height)) {
            issues.push(format!("sheets[{position}].size must be positive"));
        }
    }
}

fn check_characteristic(
    c: &TruthCharacteristic,
    sheets: &[TruthSheet],
    push: &mut impl FnMut(String),
) {
    if c.id.is_empty() {
        push("id must not be empty".to_string());
    }
    let text = &c.requirement_text;
    if text.is_empty() || text.trim() != text || text.contains("  ") {
        push(format!("requirement_text {text:?} is not normalized"));
    }

    // Region: finite, positive size, angle in (-180, 180], inside its sheet.
    let r = &c.region;
    let finite = [r.center.x, r.center.y, r.angle]
        .iter()
        .all(|v| v.is_finite());
    if !finite || !positive(r.size.width) || !positive(r.size.height) {
        push("region must be finite with a positive size".to_string());
    } else if r.angle <= -180.0 || r.angle > 180.0 {
        push(format!("region.angle {} is outside (-180, 180]", r.angle));
    }
    match sheets.iter().find(|s| s.index == c.sheet) {
        None => push(format!("sheet {} does not exist", c.sheet)),
        Some(sheet) => {
            let inside = r.corners().iter().all(|p| {
                (0.0..=sheet.size.width).contains(&p.x) && (0.0..=sheet.size.height).contains(&p.y)
            });
            if finite && !inside {
                push("region is not inside its sheet".to_string());
            }
        }
    }

    if c.fit.as_deref().is_some_and(|fit| !is_fit(fit)) {
        push("fit must look like H7 or js6".to_string());
    }
    check_values(c, push);
}

/// Nominal, unit, rule and limits must agree with each other and with the kind.
fn check_values(c: &TruthCharacteristic, push: &mut impl FnMut(String)) {
    let Some(_nominal) = c.nominal else {
        if c.unit.is_some()
            || c.tolerance_rule.is_some()
            || c.upper_limit.is_some()
            || c.lower_limit.is_some()
            || c.fit.is_some()
        {
            push("without a nominal there must be no unit, fit, rule or limits".to_string());
        }
        return;
    };

    match c.unit {
        None => push("a nominal needs a unit".to_string()),
        Some(unit) => {
            let length_kind = matches!(
                c.kind,
                CharacteristicKind::Linear
                    | CharacteristicKind::Diameter
                    | CharacteristicKind::Radius
                    | CharacteristicKind::SphericalRadius
            );
            if c.kind == CharacteristicKind::Angle && unit != Unit::Deg {
                push("an angle must use unit deg".to_string());
            }
            if length_kind && !unit.is_length() {
                push(format!("kind {:?} needs a length unit", c.kind));
            }
        }
    }

    let Some(rule) = c.tolerance_rule else {
        push("a nominal needs a tolerance_rule".to_string());
        return;
    };
    if rule == ToleranceRule::Fit && c.fit.is_none() {
        push("rule fit needs a fit designation".to_string());
    }
    match (rule.yields_limits(), c.upper_limit, c.lower_limit) {
        (true, Some(upper), Some(lower)) => {
            if lower >= upper {
                push(format!(
                    "lower_limit {lower} must be below upper_limit {upper}"
                ));
            }
        }
        (true, _, _) => push(format!("rule {rule:?} needs upper_limit and lower_limit")),
        (false, None, None) => {}
        (false, _, _) => push(format!("rule {rule:?} must not have limits")),
    }
}

fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

fn is_fit(fit: &str) -> bool {
    let letters = fit.bytes().take_while(u8::is_ascii_alphabetic).count();
    let digits = &fit[letters..];
    (1..=2).contains(&letters)
        && (1..=2).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
}

/// The JSON schema of the truth format, with object keys sorted so the output does not depend
/// on `serde_json` features enabled elsewhere in the workspace.
pub fn truth_schema() -> serde_json::Value {
    sort_keys(schemars::schema_for!(TruthFile).to_value())
}

/// [`truth_schema`] as pretty printed JSON with a trailing newline, the exact content of
/// `docs/schema/truth.schema.json`.
pub fn truth_schema_json() -> String {
    // Serializing a `Value` cannot fail: all keys are strings.
    let mut text = serde_json::to_string_pretty(&truth_schema()).unwrap_or_default();
    text.push('\n');
    text
}

pub(crate) fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, sort_keys(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_keys).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> serde_json::Value {
        json!({
            "format_version": 1,
            "drawing": { "file": "drawings/x.pdf", "sha256": "0".repeat(64) },
            "sheets": [ { "index": 0, "kind": "vector_text",
                          "size": { "width": 100.0, "height": 50.0 } } ],
            "characteristics": [ {
                "id": "c01", "sheet": 0, "kind": "diameter", "requirement_text": "Ø8 f7",
                "region": { "center": { "x": 10.0, "y": 10.0 },
                            "size": { "width": 8.0, "height": 4.0 }, "angle": 0.0 },
                "nominal": "8", "unit": "mm", "fit": "f7", "tolerance_rule": "fit",
                "upper_limit": "7.987", "lower_limit": "7.972", "inspect": true
            } ]
        })
    }

    fn issues_of(value: &serde_json::Value) -> Vec<String> {
        match TruthFile::from_json_str(&value.to_string()) {
            Ok(_) => Vec::new(),
            Err(TruthError::Invalid(issues)) => issues,
            Err(e @ TruthError::Parse(_)) => vec![e.to_string()],
        }
    }

    #[test]
    fn valid_file_round_trips_with_exact_decimals() {
        let truth = TruthFile::from_json_str(&valid().to_string()).unwrap();
        let c = &truth.characteristics[0];
        assert_eq!(c.upper_limit, Some(Decimal::new(7987, 3)));
        assert_eq!(serde_json::to_value(&truth).unwrap(), valid());
    }

    // T2.9: the settings the expected limits assume round trip and are validated.
    #[test]
    fn tolerance_settings_round_trip_and_are_checked() {
        let mut value = valid();
        value["tolerance_settings"] = json!({
            "general": { "table": { "id": "iso-2768-1", "version": 1 }, "class": "m" },
            "drawing_rule": null, "decimal_rules": [],
            "unit_rounding": { "mm_places": 3, "inch_places": 4 }, "custom_tables": []
        });
        let truth = TruthFile::from_json_str(&value.to_string()).unwrap();
        let settings = truth.tolerance_settings.as_ref().unwrap();
        assert_eq!(settings.general.as_ref().unwrap().class, "m");
        assert_eq!(serde_json::to_value(&truth).unwrap(), value);
        value["tolerance_settings"]["unit_rounding"]["mm_places"] = json!(11);
        let issues = issues_of(&value);
        assert!(
            issues.iter().any(|i| i.starts_with("tolerance_settings:")),
            "{issues:?}"
        );
    }

    /// Table driven: each edit must produce an issue containing the expected text.
    #[test]
    fn inconsistencies_are_reported() {
        let cases: &[(&str, serde_json::Value, &str)] = &[
            ("/format_version", json!(2), "format_version"),
            ("/drawing/file", json!("../x.pdf"), "relative path"),
            ("/drawing/sha256", json!("ABC"), "sha256"),
            ("/sheets/0/index", json!(1), "expected 0"),
            ("/characteristics/0/sheet", json!(3), "does not exist"),
            (
                "/characteristics/0/requirement_text",
                json!(" Ø8"),
                "normalized",
            ),
            (
                "/characteristics/0/region/center/x",
                json!(99.0),
                "inside its sheet",
            ),
            ("/characteristics/0/region/angle", json!(-180.0), "outside"),
            (
                "/characteristics/0/region/size/width",
                json!(0.0),
                "positive",
            ),
            ("/characteristics/0/fit", json!("f"), "fit must"),
            ("/characteristics/0/unit", json!("deg"), "length unit"),
            ("/characteristics/0/lower_limit", json!("7.987"), "below"),
            (
                "/characteristics/0/tolerance_rule",
                json!("no_tolerance_defined"),
                "must not have limits",
            ),
            (
                "/characteristics/0/upper_limit",
                json!("1e3"),
                "invalid decimal",
            ),
            (
                "/characteristics/0/upper_limit",
                json!(7.987),
                "invalid type",
            ),
            ("/characteristics/0/extra", json!(1), "unknown field"),
        ];
        for (pointer, value, expected) in cases {
            let mut doc = valid();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            doc.pointer_mut(parent).unwrap()[key] = value.clone();
            let issues = issues_of(&doc);
            assert!(
                issues.iter().any(|i| i.contains(expected)),
                "{pointer} = {value}: expected an issue with {expected:?}, got {issues:?}"
            );
        }
    }

    #[test]
    fn removals_are_reported() {
        let cases: &[(&str, &str)] = &[
            ("unit", "needs a unit"),
            ("tolerance_rule", "needs a tolerance_rule"),
            ("fit", "needs a fit"),
            ("lower_limit", "needs upper_limit and lower_limit"),
        ];
        for (key, expected) in cases {
            let mut doc = valid();
            doc["characteristics"][0]
                .as_object_mut()
                .unwrap()
                .remove(*key);
            let issues = issues_of(&doc);
            assert!(
                issues.iter().any(|i| i.contains(expected)),
                "without {key}: expected {expected:?}, got {issues:?}"
            );
        }
    }

    #[test]
    fn untoleranced_and_note_entries_are_valid() {
        let mut doc = valid();
        let c = doc["characteristics"][0].as_object_mut().unwrap();
        c.remove("fit");
        c.remove("upper_limit");
        c.remove("lower_limit");
        c.insert("tolerance_rule".into(), json!("no_tolerance_defined"));
        assert_eq!(issues_of(&doc), Vec::<String>::new());

        let c = doc["characteristics"][0].as_object_mut().unwrap();
        for key in ["nominal", "unit", "tolerance_rule"] {
            c.remove(key);
        }
        c.insert("kind".into(), json!("note"));
        assert_eq!(issues_of(&doc), Vec::<String>::new());
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let mut doc = valid();
        let copy = doc["characteristics"][0].clone();
        doc["characteristics"].as_array_mut().unwrap().push(copy);
        assert!(issues_of(&doc).iter().any(|i| i.contains("duplicate id")));
    }
}
