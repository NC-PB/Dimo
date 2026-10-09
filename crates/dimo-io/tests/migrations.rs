//! One fixture per schema version, loaded and migrated to the current version (NFR-REL-03,
//! testing.md).
//!
//! `tests/fixtures/v<n>/` is a project in folder mode as schema version `n` wrote it. Old
//! fixtures never change. When the schema version increases:
//!
//! 1. add the migration to `dimo_io::project::migrate::MIGRATIONS`,
//! 2. create the fixture of the new version with
//!    `DIMO_WRITE_FIXTURE=1 cargo test -p dimo-io --test migrations` (writes only a missing one),
//! 3. review the snapshots of every older fixture after migration.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;
use std::path::PathBuf;

use dimo_io::project::{APP_VERSION, Layout, MANIFEST, ProjectFile, SCHEMA_VERSION};

fn fixtures() -> PathBuf {
    common::crate_dir().join("tests/fixtures")
}

fn fixture(version: u32) -> PathBuf {
    fixtures().join(format!("v{version}"))
}

#[test]
fn current_version_has_a_fixture() {
    let dir = fixture(SCHEMA_VERSION);
    if !dir.exists() && std::env::var_os("DIMO_WRITE_FIXTURE").is_some() {
        common::sample().save(&dir, Layout::Folder).unwrap();
    }
    assert!(
        dir.join(MANIFEST).is_file(),
        "{} is missing. Create it with DIMO_WRITE_FIXTURE=1 cargo test -p dimo-io --test migrations",
        dir.display()
    );
}

#[test]
fn every_fixture_belongs_to_a_known_version() {
    let mut versions: Vec<u32> = fs::read_dir(fixtures())
        .unwrap()
        .map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            name.strip_prefix('v').unwrap().parse().unwrap()
        })
        .collect();
    versions.sort_unstable();
    assert_eq!(versions, (1..=SCHEMA_VERSION).collect::<Vec<_>>());
}

#[test]
fn every_fixture_loads_and_migrates() {
    for version in 1..=SCHEMA_VERSION {
        let loaded = ProjectFile::from_folder(&fixture(version)).unwrap();
        assert_eq!(loaded.manifest.schema_version, version);
        assert_eq!(
            loaded.migrated_from(),
            (version != SCHEMA_VERSION).then_some(version)
        );
        assert!(!loaded.file.audit.is_empty(), "fixtures carry an audit log");
        let migrated = serde_json::to_string_pretty(&loaded.file.project).unwrap();
        insta::assert_snapshot!(format!("v{version}_migrated_project"), migrated);
    }
}

/// The fixture of the current version is exactly what this build writes, so it also guards
/// the byte layout (FR-EXP-11).
#[test]
fn current_fixture_is_written_byte_identical() {
    let dir = fixture(SCHEMA_VERSION);
    let loaded = ProjectFile::from_folder(&dir).unwrap();
    let out = tempfile::tempdir().unwrap();
    let written = out.path().join("p");
    loaded.file.save(&written, Layout::Folder).unwrap();

    let mut names = vec![
        MANIFEST.to_owned(),
        "project.json".into(),
        "audit.jsonl".into(),
    ];
    for entry in fs::read_dir(dir.join("drawings")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        names.push(format!("drawings/{name}"));
    }
    for name in names {
        let expected = fs::read(dir.join(&name)).unwrap();
        let mut actual = fs::read(written.join(&name)).unwrap();
        if name == MANIFEST {
            // The fixture keeps the app version that wrote it.
            let text = String::from_utf8(actual).unwrap().replace(
                &format!("\"app_version\": \"{APP_VERSION}\""),
                &format!("\"app_version\": \"{}\"", loaded.manifest.app_version),
            );
            actual = text.into_bytes();
        }
        assert!(actual == expected, "{name} differs from the fixture");
    }
}
