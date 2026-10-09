//! The published JSON Schema of tolerance table files (T2.2, D-43, FR-TOL-07).
//!
//! `docs/schema/tolerance-table.schema.json` must equal the schema generated from the Rust
//! types. Regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-tolerance --test schema`.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;
use std::path::PathBuf;

use dimo_tolerance::format::table_schema_json;
use dimo_tolerance::load::is_table_file_name;

fn schema_path() -> PathBuf {
    common::repo_root().join("docs/schema/tolerance-table.schema.json")
}

#[test]
fn committed_schema_is_current() {
    let generated = table_schema_json();
    if std::env::var_os("DIMO_UPDATE_SCHEMA").is_some() {
        fs::write(schema_path(), &generated).unwrap();
    }
    let committed = fs::read_to_string(schema_path()).unwrap_or_default();
    assert!(
        committed == generated,
        "docs/schema/tolerance-table.schema.json is stale. Regenerate with \
         DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-tolerance --test schema"
    );
}

#[test]
fn shipped_table_files_match_the_published_schema() {
    let schema: serde_json::Value = serde_json::from_str(&table_schema_json()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:tolerance-table.schema.json", schema)
        .unwrap();
    let index = compiler
        .compile("urn:dimo:tolerance-table.schema.json", &mut schemas)
        .unwrap();

    let names: Vec<String> = common::files_with_suffix(".toml")
        .into_iter()
        .filter(|n| is_table_file_name(n))
        .collect();
    assert_ne!(names.len(), 0, "no table files found");
    for name in names {
        let text = fs::read_to_string(common::tables_dir().join(&name)).unwrap();
        let value: serde_json::Value = toml::from_str(&text).unwrap();
        if let Err(error) = schemas.validate(&value, index) {
            panic!("{name} does not match the schema: {error:#}");
        }
    }
}
