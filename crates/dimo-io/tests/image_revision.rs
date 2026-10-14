//! An image drawing revision in the project file (MF F1, ADR 0010, schema 4): stored as
//! `drawings/<sha256>.<ext>`, its edit settings changed by a command, journaled in the audit
//! log, saved, loaded and checked against the published JSON Schema. Image revisions cannot be
//! created yet (R.19), so the test adds one to the sample project by hand. The bytes are a
//! stand-in: `dimo-io` stores and hashes drawings, it never decodes them.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use dimo_core::{
    Change, Command, DrawingSource, ImageEdit, ImageFormat, Orientation, Paper, PaperSize,
    RevisionId,
};
use dimo_io::project::{
    Layout, PROJECT, ProjectError, ProjectFile, ProjectSession, schema_json, sha256,
};

/// An ID that no sample project uses.
fn id<T: serde::de::DeserializeOwned>(text: &str) -> T {
    serde_json::from_value(serde_json::json!(text)).unwrap()
}

const SCAN: &[u8] = b"\x89PNG\r\n\x1a\n stand-in scan, never decoded";

/// The sample project with a second revision: a one page PNG scan.
fn with_scan() -> (ProjectFile, RevisionId) {
    let mut file = common::sample();
    let mut scan = file.project.revisions[0].clone();
    scan.id = id("00000000-0000-0000-0000-0000000000a1");
    scan.file_name = "scan.png".into();
    scan.sha256 = sha256(SCAN);
    scan.sheets.truncate(1);
    scan.sheets[0].id = id("00000000-0000-0000-0000-0000000000a2");
    scan.source = DrawingSource::Image {
        format: ImageFormat::Png,
        pages: vec![ImageEdit::default()],
    };
    let id = scan.id;
    file.project.revisions.push(scan);
    file.insert_drawing(SCAN.to_vec());
    (file, id)
}

#[test]
fn an_image_revision_saves_loads_and_keeps_its_edit() {
    let (file, scan) = with_scan();
    let mut session = ProjectSession::from_file(file, None);
    let edit = ImageEdit {
        clean_up: true,
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: Orientation::Landscape,
        }),
        ..ImageEdit::default()
    };
    let mut env = common::env();
    common::at(&mut env, "2026-03-02T09:00:00Z");
    let patch = session
        .document_mut()
        .execute(
            Command::SetImageEdit {
                revision: scan,
                page: 0,
                edit: Box::new(edit.clone()),
            },
            &mut env,
        )
        .unwrap();
    assert!(matches!(
        patch.changes[..],
        [Change::RevisionChanged { .. }]
    ));

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("scan.dimo");
    session.save_as(&file_path, Layout::Zip).unwrap();
    let loaded = ProjectFile::load(&file_path).unwrap().file;
    assert_eq!(loaded.project, *session.project());
    assert_eq!(loaded.drawing(&sha256(SCAN)), Some(SCAN));
    let source = &loaded.project.revisions[1].source;
    assert_eq!(source.image_pages().unwrap(), [edit]);
    let last = loaded.audit.last().unwrap();
    assert!(matches!(last.changes[..], [Change::RevisionChanged { .. }]));

    // The file holds the scan under its image extension, next to the PDF.
    let zip = loaded.to_zip().unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip)).unwrap();
    let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    assert!(
        names.contains(&format!("drawings/{}.png", sha256(SCAN))),
        "{names:?}"
    );
    assert!(names.contains(&format!("drawings/{}.pdf", sha256(common::DRAWING))));

    // project.json matches the published schema, image source included.
    let mut json = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(PROJECT).unwrap(), &mut json).unwrap();
    let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("urn:dimo:project", schema).unwrap();
    let index = compiler.compile("urn:dimo:project", &mut schemas).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    schemas.validate(&value, index).unwrap();
}

/// Revisions that share a file must agree on its source, on save and on load (one stored file
/// per hash).
#[test]
fn one_file_cannot_be_two_kinds_of_source() {
    let (mut file, _) = with_scan();
    let mut twin = file.project.revisions[1].clone();
    twin.id = id("00000000-0000-0000-0000-0000000000a3");
    twin.source = DrawingSource::Pdf;
    file.project.revisions.push(twin);
    assert!(matches!(
        file.to_zip(),
        Err(ProjectError::InvalidDrawing(_))
    ));

    // A hand edited file with the same conflict does not open either.
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("p");
    let (good, _) = with_scan();
    good.save(&folder, Layout::Folder).unwrap();
    let project_json = folder.join(PROJECT);
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&project_json).unwrap()).unwrap();
    let mut twin = value["revisions"][1].clone();
    twin["id"] = serde_json::json!("00000000-0000-0000-0000-0000000000ff");
    twin["source"] = serde_json::json!({ "type": "pdf" });
    value["revisions"].as_array_mut().unwrap().push(twin);
    std::fs::write(&project_json, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(matches!(
        ProjectFile::from_folder(&folder),
        Err(ProjectError::InvalidDrawing(_))
    ));
}
