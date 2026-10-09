//! Table driven tests from `data/tolerances/*.test.toml` (D-43, FR-TOL-02, FR-TOL-04).
//!
//! Every shipped table has a test vector file and every vector must pass. The vectors were
//! drafted by the agent (`derived_by = "agent"`); the owner flips `checked_by_owner` after
//! comparing them with his Tabellenbuch.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;

use dimo_core::decimal::parse_decimal;
use dimo_tolerance::load::{SHIPPED, is_table_file_name};
use dimo_tolerance::{AppliesTo, TableSet};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorFile {
    table: String,
    #[serde(default)]
    general: Vec<GeneralVector>,
    #[serde(default)]
    fit: Vec<FitVector>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneralVector {
    applies_to: AppliesTo,
    class: String,
    size: String,
    /// Expected tolerance, or `-` for no value.
    value: String,
    derived_by: String,
    checked_by_owner: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FitVector {
    fit: String,
    nominal: String,
    /// Expected upper deviation in mm, or `-` for no value.
    upper: String,
    /// Expected lower deviation in mm, or `-` for no value.
    lower: String,
    derived_by: String,
    checked_by_owner: bool,
    #[serde(default)]
    note: Option<String>,
}

fn normalized(text: &str) -> String {
    if text == "-" {
        return text.to_owned();
    }
    parse_decimal(text).unwrap().normalize().to_string()
}

fn vector_files() -> Vec<(String, VectorFile)> {
    common::files_with_suffix(".test.toml")
        .into_iter()
        .map(|name| {
            let text = fs::read_to_string(common::tables_dir().join(&name)).unwrap();
            let file: VectorFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, file)
        })
        .collect()
}

#[test]
fn shipped_list_matches_the_folder() {
    let on_disk: Vec<String> = common::files_with_suffix(".toml")
        .into_iter()
        .filter(|n| is_table_file_name(n))
        .collect();
    let mut shipped: Vec<String> = SHIPPED.iter().map(|s| s.file_name.to_owned()).collect();
    shipped.sort();
    assert_eq!(shipped, on_disk, "load::SHIPPED must list every table file");
    for s in SHIPPED {
        let disk = fs::read_to_string(common::tables_dir().join(s.file_name)).unwrap();
        assert_eq!(disk, s.text, "{} is stale in the build", s.file_name);
    }
}

#[test]
fn every_shipped_table_has_a_vector_file_named_after_it() {
    let set = TableSet::shipped().unwrap();
    let files = vector_files();
    for table in set.tables() {
        let expected = format!("{}.test.toml", table.id());
        let file = files.iter().find(|(name, _)| *name == expected);
        let (_, file) = file.unwrap_or_else(|| panic!("missing {expected}"));
        assert_eq!(file.table, table.id());
    }
    assert_eq!(
        files.len(),
        set.tables().len(),
        "vector file without a table"
    );
}

#[test]
fn general_vectors_pass() {
    // FR-TOL-02: general tolerances by size range and class.
    let set = TableSet::shipped().unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for (name, file) in vector_files() {
        let table = set.get(&file.table).unwrap();
        for v in &file.general {
            count += 1;
            assert!(
                matches!(v.derived_by.as_str(), "agent" | "owner"),
                "{name}: derived_by must be agent or owner"
            );
            // The owner flips this flag; the test only reads it.
            let _ = v.checked_by_owner;
            let size = parse_decimal(&v.size).unwrap();
            let got = table
                .general(v.applies_to, &v.class, size)
                .map_or_else(|_| "-".to_owned(), |g| g.value.normalize().to_string());
            let want = normalized(&v.value);
            if got != want {
                failures.push(format!(
                    "{name}: {:?} {} at {}: expected {want}, got {got}",
                    v.applies_to, v.class, v.size
                ));
            }
        }
    }
    assert!(count > 0);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn fit_vectors_pass() {
    // FR-TOL-04: ISO fits expanded to limits, including range bounds and special cases.
    let set = TableSet::shipped().unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for (name, file) in vector_files() {
        let table = set.get(&file.table).unwrap();
        for v in &file.fit {
            count += 1;
            assert!(
                matches!(v.derived_by.as_str(), "agent" | "owner"),
                "{name}: derived_by must be agent or owner"
            );
            let _ = (v.checked_by_owner, &v.note);
            let nominal = parse_decimal(&v.nominal).unwrap();
            let got = table.fit_limits(&v.fit, nominal).map_or_else(
                |_| ("-".to_owned(), "-".to_owned()),
                |l| {
                    (
                        l.upper.normalize().to_string(),
                        l.lower.normalize().to_string(),
                    )
                },
            );
            let want = (normalized(&v.upper), normalized(&v.lower));
            if got != want {
                failures.push(format!(
                    "{name}: {} at {}: expected {want:?}, got {got:?}",
                    v.fit, v.nominal
                ));
            }
        }
    }
    assert!(count > 0);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
