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
    for folder in ["drawings", "tolerances"] {
        let Ok(listing) = fs::read_dir(dir.join(folder)) else {
            continue;
        };
        for entry in listing {
            let name = entry.unwrap().file_name().into_string().unwrap();
            names.push(format!("{folder}/{name}"));
        }
    }
    let mut written_names = Vec::new();
    for folder in ["drawings", "tolerances"] {
        if let Ok(listing) = fs::read_dir(written.join(folder)) {
            for entry in listing {
                let name = entry.unwrap().file_name().into_string().unwrap();
                written_names.push(format!("{folder}/{name}"));
            }
        }
    }
    written_names.sort();
    let mut expected_names: Vec<&String> = names.iter().skip(3).collect();
    expected_names.sort();
    assert_eq!(
        written_names.iter().collect::<Vec<_>>(),
        expected_names,
        "same drawings and tables"
    );
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

/// T2.4 acceptance: the version 1 fixture opens, saves as version 2, and saving the reopened
/// file again gives identical bytes (NFR-REL-03, FR-EXP-11).
#[test]
fn version_1_saves_as_version_2_and_then_byte_identical() {
    let loaded = ProjectFile::from_folder(&fixture(1)).unwrap();
    assert_eq!(loaded.migrated_from(), Some(1));
    let first = loaded.file.to_zip().unwrap();
    let reopened = ProjectFile::from_zip(&first).unwrap();
    assert_eq!(reopened.manifest.schema_version, 2);
    assert_eq!(reopened.migrated_from(), None);
    assert_eq!(reopened.file, loaded.file);
    let second = reopened.file.to_zip().unwrap();
    assert!(first == second, "second save differs from the first");

    // Folder mode as well.
    let out = tempfile::tempdir().unwrap();
    let (a, b) = (out.path().join("a"), out.path().join("b"));
    loaded.file.save(&a, Layout::Folder).unwrap();
    ProjectFile::from_folder(&a)
        .unwrap()
        .file
        .save(&b, Layout::Folder)
        .unwrap();
    for name in [MANIFEST, "project.json", "audit.jsonl"] {
        assert!(
            fs::read(a.join(name)).unwrap() == fs::read(b.join(name)).unwrap(),
            "{name} differs"
        );
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(a.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 2);
}

/// What the version 1 to 2 migration makes of the version 1 data (T2.4).
#[test]
fn version_1_migration_fills_the_new_fields() {
    let project = ProjectFile::from_folder(&fixture(1)).unwrap().file.project;
    let numbers: Vec<String> = project
        .characteristics
        .iter()
        .map(|c| c.number.to_string())
        .collect();
    assert_eq!(numbers, ["1", "2"]);
    // Limits typed by hand in version 1 are manual (FR-TOL-08); no limits, no derivation.
    assert_eq!(
        project.characteristics[0].derivation,
        Some(dimo_core::ToleranceDerivation::manual())
    );
    assert_eq!(project.characteristics[1].derivation, None);
    let settings = &project.settings;
    assert_eq!(
        settings.numbering.strategy,
        dimo_core::NumberingStrategy::Manual
    );
    assert_eq!(
        settings.numbering.insert_when_locked,
        dimo_core::InsertPolicy::NextFree
    );
    assert_eq!(settings.tolerance, dimo_core::ToleranceSettings::default());
    let lock = project.numbering.lock.as_ref().unwrap();
    assert_eq!((lock.highest_number, lock.given.len()), (2, 0));
    let sheet = &project.revisions[0].sheets[0];
    assert!(sheet.zone_grid.is_none() && sheet.views.is_empty());
}
