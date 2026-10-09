//! Loading and validation of tolerance tables (T2.2, D-43, FR-TOL-07).

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;

use common::d;
use dimo_tolerance::{AppliesTo, LookupError, Table, TableError, TableSet, load_dir};

/// A minimal valid custom table (FR-TOL-07) with the given id.
fn custom(id: &str) -> String {
    format!(
        r#"
[table]
id = "{id}"
title = "Company general tolerances"
kind = "custom"
unit = "mm"
version = 2
status = "draft"
source = "Company standard 12, page 3"

[[part]]
id = "linear"
source = "page 3"
kind = "symmetric"
applies_to = "linear"
value_unit = "mm"
columns = ["fine", "coarse"]
rows = [
  {{ max_exclusive = "10", values = ["0.05", "0.2"] }},
  {{ min_inclusive = "10", max_inclusive = "100", values = ["0.1", "-"] }},
  {{ min_exclusive = "100", values = ["0.2", "0.5"] }},
]
"#
    )
}

fn invalid_message(text: &str) -> String {
    match Table::parse("test.toml", text) {
        Err(TableError::Invalid {
            location, message, ..
        }) => format!("{location}: {message}"),
        Err(TableError::Parse { message, .. }) => format!("parse: {message}"),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn custom_table_lookup_respects_bounds() {
    // FR-TOL-07: user tables use the same format and lookup.
    let table = Table::parse("test.toml", &custom("company-a")).unwrap();
    assert!(table.is_draft());
    let at = |class: &str, size: &str| table.general(AppliesTo::Linear, class, d(size));
    assert_eq!(at("fine", "9.999").unwrap().value, d("0.05"));
    assert_eq!(at("fine", "10").unwrap().value, d("0.1"));
    assert_eq!(at("fine", "100").unwrap().value, d("0.1"));
    assert_eq!(at("fine", "100.001").unwrap().value, d("0.2"));
    assert_eq!(at("fine", "100000").unwrap().value, d("0.2"));
    let found = at("fine", "50").unwrap();
    assert_eq!(found.range.to_string(), "from 10 up to and including 100");
    assert_eq!(found.version, 2);
    assert!(found.draft);
    assert!(matches!(
        at("coarse", "50"),
        Err(LookupError::NotDefined { .. })
    ));
    assert!(matches!(
        at("medium", "50"),
        Err(LookupError::UnknownColumn { .. })
    ));
    assert!(matches!(
        at("fine", "0"),
        Err(LookupError::SizeNotPositive { .. })
    ));
    assert!(matches!(
        table.general(AppliesTo::Angular, "fine", d("5")),
        Err(LookupError::NoPart { .. })
    ));
}

#[test]
fn iso_2768_1_ranges_and_units() {
    // FR-TOL-02: linear, broken edges and angular parts.
    let set = TableSet::shipped().unwrap();
    let table = set.get("iso-2768-1").unwrap();
    let linear = table.general(AppliesTo::Linear, "m", d("45")).unwrap();
    assert_eq!(linear.value, d("0.3"));
    assert_eq!(linear.range.to_string(), "over 30 up to and including 120");
    let angular = table.general(AppliesTo::Angular, "m", d("45")).unwrap();
    assert_eq!(angular.value, d("30"));
    assert_eq!(angular.unit, dimo_tolerance::ValueUnit::Arcmin);
    assert_eq!(angular.range_of, dimo_tolerance::RangeOf::ShorterLeg);
    assert!(matches!(
        table.general(AppliesTo::Linear, "m", d("4001")),
        Err(LookupError::OutOfRange { .. })
    ));
}

#[test]
fn duplicate_ids_are_refused() {
    let a = Table::parse("a.toml", &custom("company-a")).unwrap();
    let b = Table::parse("b.toml", &custom("company-a")).unwrap();
    let err = TableSet::new(vec![a.clone(), b.clone()]).unwrap_err();
    assert!(matches!(err, TableError::Duplicate { ref id, .. } if id == "company-a"));
    assert!(err.to_string().contains("a.toml") && err.to_string().contains("b.toml"));

    let mut set = TableSet::new(vec![a]).unwrap();
    assert!(set.add(b).is_err());

    // A user table may not take the id of a shipped table.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("mine.toml"), custom("iso-2768-1")).unwrap();
    assert!(matches!(
        TableSet::with_user_dir(dir.path()),
        Err(TableError::Duplicate { .. })
    ));
}

#[test]
fn user_folder_is_loaded_sorted_and_validated() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("b.toml"), custom("company-b")).unwrap();
    fs::write(dir.path().join("a.toml"), custom("company-a")).unwrap();
    // Test vector files and other files are not tables.
    fs::write(dir.path().join("a.test.toml"), "not a table").unwrap();
    fs::write(dir.path().join("notes.txt"), "not a table").unwrap();
    let tables = load_dir(dir.path()).unwrap();
    let ids: Vec<&str> = tables.iter().map(Table::id).collect();
    assert_eq!(ids, ["company-a", "company-b"]);

    let set = TableSet::with_user_dir(dir.path()).unwrap();
    assert!(set.get("iso-2768-1").is_some());
    assert!(set.get("company-b").is_some());

    // Every load is validated: one broken file fails the folder with its name.
    fs::write(dir.path().join("c.toml"), custom("Company C")).unwrap();
    let err = load_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("c.toml"), "{err}");

    // A missing folder holds no tables.
    assert_eq!(load_dir(&dir.path().join("missing")).unwrap().len(), 0);
}

#[test]
fn parse_errors_name_the_line() {
    let text = custom("company-a").replace(r#"values = ["0.05", "0.2"]"#, "values = [0.05, 0.2]");
    let message = invalid_message(&text);
    assert!(message.contains("line"), "{message}");
}

#[test]
fn validation_rules() {
    let base = custom("company-a");
    let cases: Vec<(String, &str)> = vec![
        // Floats are never accepted (rule 5).
        (
            base.replace(r#""0.05", "0.2""#, r#""0.05", "2e-1""#),
            "invalid cell",
        ),
        (base.replace("version = 2", "version = 0"), "version"),
        (
            base.replace(r#"id = "company-a""#, r#"id = "Company""#),
            "id",
        ),
        (
            base.replace(
                r#"source = "Company standard 12, page 3""#,
                r#"source = " ""#,
            ),
            "source",
        ),
        // D-43: verification fields only with the verified status.
        (
            base.replace(
                r#"status = "draft""#,
                "status = \"draft\"\nverified_by = \"x\"",
            ),
            "draft",
        ),
        (
            base.replace(r#"status = "draft""#, r#"status = "verified""#),
            "verified",
        ),
        (
            base.replace(r#""0.05", "0.2""#, r#""0.05""#),
            "1 values for 2 columns",
        ),
        (
            base.replace(r#"min_inclusive = "10""#, r#"min_exclusive = "10""#),
            "start where",
        ),
        (
            base.replace(r#"min_inclusive = "10""#, r#"min_inclusive = "11""#),
            "start where",
        ),
        (base.replace(r#"max_exclusive = "10", "#, ""), "last row"),
        (
            base.replace(
                r#"min_exclusive = "100", "#,
                r#"min_exclusive = "100", max_inclusive = "90", "#,
            ),
            "below the upper",
        ),
        (
            base.replace(r#""0.1", "-""#, r#""-0.1", "-""#),
            "negative value",
        ),
        (
            base.replace(
                r#"columns = ["fine", "coarse"]"#,
                r#"columns = ["fine", "fine"]"#,
            ),
            "used twice",
        ),
        (
            base.replace(r#"applies_to = "linear""#, r#"applies_to = "angular""#),
            "arcmin",
        ),
        (
            base.replace(r#"kind = "symmetric""#, r#"kind = "delta""#),
            "symmetric",
        ),
        (
            base.replace(r#"kind = "custom""#, r#"kind = "fit""#),
            "no symmetric",
        ),
        (
            base.replace("[[part]]", "[[part]]\nunknown = 1"),
            "unknown field",
        ),
        (
            format!("{base}\n{}", &base[base.find("[[part]]").unwrap()..]),
            "used twice",
        ),
    ];
    for (text, expected) in cases {
        let message = invalid_message(&text);
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
    }
}
