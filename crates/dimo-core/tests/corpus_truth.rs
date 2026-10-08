//! Corpus truth files and their JSON schema (T0.10, D-42).
//!
//! - `docs/schema/truth.schema.json` must equal the schema generated from the Rust types.
//!   Regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-core --test corpus_truth`.
//! - Every `corpus/truth/*.truth.json` must validate against that schema, load without
//!   consistency issues, and reference a drawing whose SHA-256 matches the file on disk and
//!   `corpus/PROVENANCE.md` (testing.md: corpus tests verify the hash).

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::fs;
use std::path::{Path, PathBuf};

use dimo_core::characteristic::{CharacteristicKind, ToleranceRule};
use dimo_core::sheet::SheetKind;
use dimo_core::truth::{TruthFile, truth_schema_json};
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema_path() -> PathBuf {
    repo_root().join("docs/schema/truth.schema.json")
}

fn truth_files() -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(repo_root().join("corpus/truth"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".truth.json"))
        .collect();
    files.sort();
    files
}

fn compiled_schema() -> (boon::Schemas, boon::SchemaIndex) {
    let schema: serde_json::Value = serde_json::from_str(&truth_schema_json()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:truth.schema.json", schema)
        .unwrap();
    let index = compiler
        .compile("urn:dimo:truth.schema.json", &mut schemas)
        .unwrap();
    (schemas, index)
}

#[test]
fn committed_schema_is_current() {
    let generated = truth_schema_json();
    if std::env::var_os("DIMO_UPDATE_SCHEMA").is_some() {
        fs::create_dir_all(schema_path().parent().unwrap()).unwrap();
        fs::write(schema_path(), &generated).unwrap();
    }
    let committed = fs::read_to_string(schema_path()).unwrap_or_default();
    assert!(
        committed == generated,
        "docs/schema/truth.schema.json is stale. Regenerate with \
         DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-core --test corpus_truth"
    );
}

#[test]
fn there_is_at_least_one_truth_file() {
    assert_ne!(truth_files(), [] as [PathBuf; 0]);
}

#[test]
fn truth_files_validate_against_the_schema() {
    let (schemas, index) = compiled_schema();
    for path in truth_files() {
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        if let Err(error) = schemas.validate(&value, index) {
            panic!("{} does not match the schema: {error:#}", path.display());
        }
    }
}

#[test]
fn schema_rejects_numbers_and_loose_decimal_text() {
    let (schemas, index) = compiled_schema();
    let path = repo_root().join("corpus/truth/test_drawing_1.truth.json");
    let valid: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for bad in [
        serde_json::json!(30.0203),
        serde_json::json!("1e3"),
        serde_json::json!("−0.6"),
        serde_json::json!(".5"),
    ] {
        let mut doc = valid.clone();
        doc["characteristics"][0]["upper_limit"] = bad.clone();
        assert!(
            schemas.validate(&doc, index).is_err(),
            "{bad} must be rejected"
        );
    }
}

#[test]
fn truth_files_load_and_match_their_drawing() {
    let provenance = fs::read_to_string(repo_root().join("corpus/PROVENANCE.md")).unwrap();
    for path in truth_files() {
        let truth = TruthFile::from_json_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.display();

        let drawing = fs::read(repo_root().join("corpus").join(&truth.drawing.file))
            .unwrap_or_else(|e| panic!("{name}: cannot read {}: {e}", truth.drawing.file));
        let actual = format!("{:x}", Sha256::digest(&drawing));
        assert_eq!(truth.drawing.sha256, actual, "{name}: drawing hash");

        // PROVENANCE.md lists `| <file> | <first 8>...<last 7> | ...`.
        let row = provenance
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", truth.drawing.file)))
            .unwrap_or_else(|| panic!("{name}: no PROVENANCE.md row for the drawing"));
        let short = row.split('|').nth(2).unwrap().trim();
        let (head, tail) = short.split_once("...").unwrap();
        assert!(
            head.len() >= 8 && tail.len() >= 7,
            "{name}: short hash {short:?}"
        );
        assert!(
            actual.starts_with(head) && actual.ends_with(tail),
            "{name}: PROVENANCE.md hash {short} does not match {actual}"
        );
    }
}

/// Regression facts from `corpus/notes/test_drawing_1.md`.
#[test]
fn test_drawing_1_truth_matches_the_notes() {
    let path = repo_root().join("corpus/truth/test_drawing_1.truth.json");
    let truth = TruthFile::from_json_str(&fs::read_to_string(path).unwrap()).unwrap();

    assert_eq!(truth.sheets.len(), 1);
    assert_eq!(truth.sheets[0].kind, SheetKind::VectorText);
    assert_eq!(truth.characteristics.len(), 24);

    let texts: Vec<_> = truth
        .characteristics
        .iter()
        .map(|c| c.requirement_text.as_str())
        .collect();
    // Finding 3: hyphen and Unicode minus on the same sheet, kept as printed.
    assert!(texts.contains(&"100 +0 \u{2212}0.6"));
    assert!(texts.contains(&"90.0° +0.0° \u{2212}0.1°"));
    assert!(texts.contains(&"Ø30 H7 +0.0203 -0"));
    // Finding 5: three separate Ø8 f7 callouts.
    assert_eq!(texts.iter().filter(|t| t.starts_with("Ø8 f7")).count(), 3);

    // Finding 2: untoleranced dimensions have no limits.
    let untoleranced: Vec<_> = truth
        .characteristics
        .iter()
        .filter(|c| c.tolerance_rule == Some(ToleranceRule::NoToleranceDefined))
        .map(|c| c.requirement_text.as_str())
        .collect();
    assert_eq!(
        untoleranced,
        [
            "50", "90.0°", "31.05", "52.61", "71.04", "86.16", "30", "20", "200", "40", "100"
        ]
    );

    // Finding 1: explicit deviations win over ISO 286.
    let c03 = truth
        .characteristics
        .iter()
        .find(|c| c.id == "c03")
        .unwrap();
    assert_eq!(c03.kind, CharacteristicKind::Diameter);
    assert_eq!(c03.tolerance_rule, Some(ToleranceRule::Explicit));
    assert_eq!(c03.upper_limit.unwrap().to_string(), "30.0203");
}
