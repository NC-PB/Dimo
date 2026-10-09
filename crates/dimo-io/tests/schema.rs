//! The published JSON Schema of `project.json` (T1.3, ADR 0004, data model 07).
//!
//! `docs/schema/project.schema.json` must equal the schema generated from the Rust types.
//! Regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-io --test schema`. A changed
//! schema usually needs a schema version increase with a migration (NFR-REL-03).

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;
use std::path::PathBuf;

use dimo_io::project::{PROJECT, ProjectFile, schema_json};

fn schema_path() -> PathBuf {
    common::repo_root().join("docs/schema/project.schema.json")
}

#[test]
fn committed_schema_is_current() {
    let generated = schema_json();
    if std::env::var_os("DIMO_UPDATE_SCHEMA").is_some() {
        fs::write(schema_path(), &generated).unwrap();
    }
    let committed = fs::read_to_string(schema_path()).unwrap_or_default();
    assert!(
        committed == generated,
        "docs/schema/project.schema.json is stale. Regenerate with \
         DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-io --test schema, and check whether the \
         change needs a schema version increase with a migration"
    );
}

#[test]
fn written_projects_match_the_published_schema() {
    let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("urn:dimo:project.schema.json", schema)
        .unwrap();
    let index = compiler
        .compile("urn:dimo:project.schema.json", &mut schemas)
        .unwrap();

    let file = common::sample();
    let bytes = file.to_zip().unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_slice())).unwrap();
    let project: serde_json::Value =
        serde_json::from_reader(archive.by_name(PROJECT).unwrap()).unwrap();
    if let Err(error) = schemas.validate(&project, index) {
        panic!("project.json does not match the schema: {error:#}");
    }
    assert!(ProjectFile::from_zip(&bytes).is_ok());
}
