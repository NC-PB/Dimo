//! The `.dimo` container: round trips, determinism and safe reading (T1.3, ADR 0004,
//! FR-EXP-11, NFR-SEC-05, NFR-REL-04).

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::fs;
use std::io::{Cursor, Read as _, Write as _};

use dimo_core::{Point, Sha256Hex};
use dimo_io::project::{
    APP_VERSION, Layout, Limits, ProjectError, ProjectFile, SCHEMA_VERSION, sha256,
};
use proptest::prelude::*;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Entry names and contents of a ZIP container, in stored order.
fn unzip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            (file.name().to_owned(), data)
        })
        .collect()
}

/// A ZIP with the given entries, deflated, as other tools would write it.
fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The sample's entries with `name` replaced or added.
fn sample_entries_with(name: &str, data: &[u8]) -> Vec<u8> {
    let mut entries = unzip(&common::sample().to_zip().unwrap());
    match entries.iter_mut().find(|(n, _)| n == name) {
        Some(entry) => entry.1 = data.to_vec(),
        None => entries.push((name.to_owned(), data.to_vec())),
    }
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    zip_of(&borrowed)
}

// ---------------------------------------------------------------------------------------------
// Acceptance: save, load, save again gives identical bytes

#[test]
fn save_load_save_gives_identical_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("part.dimo");
    let file = common::sample();
    file.save(&path, Layout::Zip).unwrap();
    let first = fs::read(&path).unwrap();

    let loaded = ProjectFile::load(&path).unwrap();
    assert_eq!(loaded.file, file, "load gives back what was saved");
    assert_eq!(loaded.migrated_from(), None);
    loaded.file.save(&path, Layout::Zip).unwrap();
    assert_eq!(fs::read(&path).unwrap(), first);
}

#[test]
fn folder_mode_holds_the_same_entries() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("part.dimo");
    let file = common::sample();
    file.save(&folder, Layout::Folder).unwrap();
    assert_eq!(Layout::of(&folder), Layout::Folder);

    for (name, data) in unzip(&file.to_zip().unwrap()) {
        assert_eq!(fs::read(folder.join(&name)).unwrap(), data, "{name}");
    }
    let loaded = ProjectFile::load(&folder).unwrap();
    assert_eq!(loaded.file, file);
    loaded.file.save(&folder, Layout::Folder).unwrap();
    assert_eq!(ProjectFile::load(&folder).unwrap().file, file);
}

#[test]
fn folder_mode_removes_drawings_that_left_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("p");
    let file = common::sample();
    file.save(&folder, Layout::Folder).unwrap();
    let old = folder.join(format!("drawings/{}.pdf", sha256(b"old")));
    fs::write(&old, b"old").unwrap();
    let notes = folder.join("drawings/notes.txt");
    fs::write(&notes, b"mine").unwrap();
    file.save(&folder, Layout::Folder).unwrap();
    assert!(!old.exists());
    assert!(notes.exists(), "files that are no drawings are left alone");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Floats in sheet geometry load back exactly, so the bytes stay identical.
    #[test]
    fn any_balloon_position_round_trips(x in -1e6f64..1e6, y in any::<f64>().prop_filter("finite", |v| v.is_finite())) {
        let mut file = common::sample();
        file.project.balloons[0].position = Point { x, y };
        let bytes = file.to_zip().unwrap();
        let loaded = ProjectFile::from_zip(&bytes).unwrap();
        prop_assert_eq!(&loaded.file, &file);
        prop_assert_eq!(loaded.file.to_zip().unwrap(), bytes);
    }
}

// ---------------------------------------------------------------------------------------------
// Deterministic layout

#[test]
fn container_layout_is_fixed() {
    let file = common::sample();
    let bytes = file.to_zip().unwrap();
    assert_eq!(bytes, common::sample().to_zip().unwrap());

    let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
    let hash = sha256(common::DRAWING);
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "manifest.json".to_owned(),
            "project.json".to_owned(),
            "audit.jsonl".to_owned(),
            format!("drawings/{hash}.pdf"),
        ]
    );
    for i in 0..archive.len() {
        let entry = archive.by_index(i).unwrap();
        assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
        assert_eq!(entry.unix_mode(), Some(0o100_644));
        // The time of the last audit entry, 2026-03-01T08:09:59.999Z, at 2 s resolution.
        let time = entry.last_modified().unwrap();
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (2026, 3, 1),
            "{}",
            entry.name()
        );
        assert_eq!((time.hour(), time.minute(), time.second()), (8, 9, 58));
    }
}

#[test]
fn drawings_are_stored_byte_identical() {
    let file = common::sample();
    let hash = sha256(common::DRAWING);
    let entries = unzip(&file.to_zip().unwrap());
    let stored = &entries
        .iter()
        .find(|(n, _)| *n == format!("drawings/{hash}.pdf"))
        .unwrap()
        .1;
    assert_eq!(stored.as_slice(), common::DRAWING);
    let loaded = ProjectFile::from_zip(&file.to_zip().unwrap()).unwrap();
    assert_eq!(loaded.file.drawing(&hash), Some(common::DRAWING));
}

#[test]
fn manifest_and_audit_content() {
    let file = common::sample();
    let entries = unzip(&file.to_zip().unwrap());
    let manifest = String::from_utf8(entries[0].1.clone()).unwrap();
    let manifest = manifest.replace(APP_VERSION, "<app version>");
    insta::assert_snapshot!("manifest", manifest);

    let audit = String::from_utf8(entries[2].1.clone()).unwrap();
    assert_eq!(audit.lines().count(), file.audit.len());
    assert!(audit.ends_with('\n'));
    assert_eq!(SCHEMA_VERSION, 1);
}

#[test]
fn deflated_containers_from_other_tools_load() {
    let file = common::sample();
    let entries = unzip(&file.to_zip().unwrap());
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let deflated = zip_of(&borrowed);
    let loaded = ProjectFile::from_zip(&deflated).unwrap();
    assert_eq!(loaded.file, file);
    assert_eq!(loaded.file.to_zip().unwrap(), file.to_zip().unwrap());
}

#[test]
fn unknown_entries_are_ignored() {
    let bytes = sample_entries_with("extra/readme.txt", b"hello");
    assert_eq!(
        ProjectFile::from_zip(&bytes).unwrap().file,
        common::sample()
    );
}

// ---------------------------------------------------------------------------------------------
// Refusals (NFR-SEC-05, NFR-REL-04, FR-DOC-07)

#[test]
fn zip_slip_names_are_refused() {
    for name in [
        "../evil.pdf",
        "/abs/project.json",
        "drawings/../../x",
        "a\\b",
    ] {
        let bytes = sample_entries_with(name, b"x");
        let error = ProjectFile::from_zip(&bytes).unwrap_err();
        assert!(
            matches!(&error, ProjectError::UnsafeEntry(n) if n == name),
            "{name}: {error}"
        );
    }
}

#[test]
fn symlink_entries_are_refused() {
    let mut entries = unzip(&common::sample().to_zip().unwrap());
    entries.truncate(3);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in &entries {
        writer
            .start_file(name.as_str(), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer
        .add_symlink(
            "drawings/link.pdf",
            "/etc/passwd",
            SimpleFileOptions::default(),
        )
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let error = ProjectFile::from_zip(&bytes).unwrap_err();
    assert!(matches!(error, ProjectError::Symlink(_)), "{error}");
}

#[test]
fn oversized_entries_are_refused() {
    let bytes = common::sample().to_zip().unwrap();
    let small = Limits {
        max_drawing: 16,
        ..Limits::DEFAULT
    };
    let error = ProjectFile::from_zip_with_limits(&bytes, &small).unwrap_err();
    assert!(
        matches!(&error, ProjectError::TooLarge { name, limit: 16 } if name.starts_with("drawings/")),
        "{error}"
    );
    let small = Limits {
        max_total: 1000,
        ..Limits::DEFAULT
    };
    let error = ProjectFile::from_zip_with_limits(&bytes, &small).unwrap_err();
    assert!(matches!(error, ProjectError::TooLarge { .. }), "{error}");
    let few = Limits {
        max_entries: 3,
        ..Limits::DEFAULT
    };
    let error = ProjectFile::from_zip_with_limits(&bytes, &few).unwrap_err();
    assert!(matches!(error, ProjectError::TooManyEntries(3)), "{error}");
}

#[test]
fn newer_schema_versions_are_refused_and_left_untouched() {
    let manifest = br#"{"format":"dimo-project","schema_version":99,"future":"field"}"#;
    let bytes = sample_entries_with("manifest.json", manifest);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.dimo");
    fs::write(&path, &bytes).unwrap();
    let error = ProjectFile::load(&path).unwrap_err();
    assert!(
        matches!(
            error,
            ProjectError::NewerVersion {
                found: 99,
                supported: 1
            }
        ),
        "{error}"
    );
    assert!(error.to_string().contains("newer version of Dimo"));
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn foreign_files_are_no_projects() {
    let bytes = sample_entries_with("manifest.json", br#"{"format":"other","schema_version":1}"#);
    assert!(matches!(
        ProjectFile::from_zip(&bytes),
        Err(ProjectError::NotAProject(_))
    ));
    assert!(matches!(
        ProjectFile::from_zip(b"%PDF-1.7 not a zip"),
        Err(ProjectError::Container(_))
    ));
    let no_manifest = zip_of(&[("project.json", b"{}")]);
    assert!(matches!(
        ProjectFile::from_zip(&no_manifest),
        Err(ProjectError::MissingEntry("manifest.json"))
    ));
}

#[test]
fn drawings_must_match_their_hash() {
    let hash = sha256(common::DRAWING);
    let bytes = sample_entries_with(&format!("drawings/{hash}.pdf"), b"%PDF changed");
    let error = ProjectFile::from_zip(&bytes).unwrap_err();
    assert!(
        matches!(&error, ProjectError::HashMismatch { expected, .. } if *expected == hash),
        "{error}"
    );

    let mut file = common::sample();
    file.drawings.clear();
    assert!(matches!(
        file.to_zip(),
        Err(ProjectError::MissingDrawing(h)) if h == hash
    ));
}

#[test]
fn invalid_json_names_the_entry() {
    let bytes = sample_entries_with("audit.jsonl", b"{\"broken\": \n");
    let error = ProjectFile::from_zip(&bytes).unwrap_err();
    assert!(
        error.to_string().starts_with("audit.jsonl line 1"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn folder_mode_refuses_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("p");
    common::sample().save(&folder, Layout::Folder).unwrap();
    let hash: Sha256Hex = sha256(common::DRAWING);
    let drawing = folder.join(format!("drawings/{hash}.pdf"));
    let outside = dir.path().join("outside.pdf");
    fs::rename(&drawing, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &drawing).unwrap();
    let error = ProjectFile::load(&folder).unwrap_err();
    assert!(matches!(error, ProjectError::Symlink(_)), "{error}");
}
