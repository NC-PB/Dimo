//! Creating a project from a drawing file through a raster opener (FR-DOC-07, MF F1).
//!
//! Most cases use a fake opener with a synthetic source, so they run without PDFium. The
//! corpus drawing test opens the real PDF through the PDF engine; it skips without PDFium and
//! fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

mod common;

use std::cell::RefCell;
use std::sync::Arc;

use dimo_core::{
    DrawingSource, ImageFormat, ProjectInfo, Rect, ReportInfo, Sha256Hex, SheetKind, Size,
    Timestamp,
};
use dimo_io::lock::LockOwner;
use dimo_io::project::{
    AutosaveTarget, CreateError, Layout, NewProject, ProjectError, ProjectFile, ProjectSession,
    create_from_drawing, create_from_drawing_file, open_drawing,
};
use dimo_pdf::{Opened, PdfEngine, PdfError, RasterOpener, RasterSource, RgbaImage, SourceKind};

/// A source with fixed sheets, as a reader would report them.
#[derive(Debug)]
struct Fake {
    hash: Sha256Hex,
    sheets: Vec<(Size, SheetKind)>,
}

impl RasterSource for Fake {
    fn kind(&self) -> SourceKind {
        SourceKind::Pdf
    }

    fn content_hash(&self) -> &Sha256Hex {
        &self.hash
    }

    fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    fn sheet_size(&self, sheet: usize) -> Result<Size, PdfError> {
        self.sheets
            .get(sheet)
            .map(|s| s.0)
            .ok_or(PdfError::SheetOutOfRange {
                index: sheet,
                count: self.sheets.len(),
            })
    }

    fn render_region(&self, _: usize, _: Rect, _: f64) -> Result<RgbaImage, PdfError> {
        Err(PdfError::Render("not drawn in these tests".into()))
    }

    fn sheet_kind(&self, sheet: usize) -> Result<SheetKind, PdfError> {
        self.sheets
            .get(sheet)
            .map(|s| s.1)
            .ok_or(PdfError::SheetOutOfRange {
                index: sheet,
                count: self.sheets.len(),
            })
    }
}

/// Opens every PDF as a [`Fake`] with the given sheets and records what happens.
#[derive(Default)]
struct FakeOpener {
    sheets: Vec<(Size, SheetKind)>,
    already_open: bool,
    opened: RefCell<usize>,
    released: RefCell<Vec<Sha256Hex>>,
}

impl RasterOpener for FakeOpener {
    fn open_pdf(&self, bytes: Vec<u8>) -> Result<Opened, PdfError> {
        *self.opened.borrow_mut() += 1;
        Ok(Opened {
            source: Arc::new(Fake {
                hash: Sha256Hex::of(&bytes),
                sheets: self.sheets.clone(),
            }),
            newly_opened: !self.already_open,
        })
    }

    fn release(&self, hash: &Sha256Hex) {
        self.released.borrow_mut().push(*hash);
    }
}

const A3: Size = Size {
    width: 1190.55,
    height: 841.89,
};

fn opener() -> FakeOpener {
    FakeOpener {
        sheets: vec![(A3, SheetKind::VectorText), (A3, SheetKind::Raster)],
        ..FakeOpener::default()
    }
}

fn new_project() -> NewProject {
    NewProject {
        info: ProjectInfo {
            part_number: "P-100".into(),
            ..ProjectInfo::default()
        },
        report_info: ReportInfo {
            company_name: "Shop".into(),
            ..ReportInfo::default()
        },
        autosave: None,
    }
}

fn owner() -> LockOwner {
    LockOwner {
        user: "tester".into(),
        pid: std::process::id(),
        since: Timestamp::parse("2026-03-01T08:00:00Z").unwrap(),
    }
}

#[test]
fn a_pdf_becomes_a_project_with_the_sheets_the_reader_found() {
    let opener = opener();
    let mut env = common::env();
    let created = create_from_drawing(
        common::DRAWING.to_vec(),
        "/drawings/part-100.pdf",
        &opener,
        new_project(),
        &mut env,
    )
    .unwrap();
    let project = created.session.project();
    let revision = project.current_revision().unwrap();
    assert_eq!(revision.source, DrawingSource::Pdf);
    assert_eq!(revision.file_name, "part-100.pdf");
    assert_eq!(revision.sha256, Sha256Hex::of(common::DRAWING));
    assert_eq!(
        revision.sheets.iter().map(|s| s.kind).collect::<Vec<_>>(),
        [SheetKind::VectorText, SheetKind::Raster]
    );
    assert_eq!(project.info.part_number, "P-100");
    assert_eq!(project.report_info.company_name, "Shop");
    assert!(
        !created.session.is_modified(),
        "initial state, no undo step"
    );
    assert_eq!(created.source.content_hash(), &revision.sha256);
    assert!(created.lock.is_none() && created.autosave_error.is_none());
    assert_eq!(
        created.session.drawing(&revision.sha256),
        Some(common::DRAWING)
    );
    assert!(opener.released.borrow().is_empty());
}

#[test]
fn images_and_unknown_files_are_refused_before_opening() {
    let opener = opener();
    let mut env = common::env();
    for (bytes, format) in [
        (&b"\xFF\xD8\xFF\xE0 jpeg"[..], ImageFormat::Jpeg),
        (&b"\x89PNG\r\n\x1a\n png"[..], ImageFormat::Png),
        (&b"II*\0 tiff"[..], ImageFormat::Tiff),
        (&b"MM\0* tiff"[..], ImageFormat::Tiff),
    ] {
        let error =
            create_from_drawing(bytes.to_vec(), "x", &opener, new_project(), &mut env).unwrap_err();
        assert!(
            matches!(error, CreateError::ImageNotSupported(f) if f == format),
            "{error}"
        );
    }
    let error = create_from_drawing(
        b"GIF89a".to_vec(),
        "x.gif",
        &opener,
        new_project(),
        &mut env,
    )
    .unwrap_err();
    assert!(matches!(error, CreateError::UnknownType), "{error}");
    assert_eq!(*opener.opened.borrow(), 0, "nothing was opened");
}

#[test]
fn a_failed_import_releases_only_a_newly_opened_source() {
    let mut env = common::env();
    for already_open in [false, true] {
        let opener = FakeOpener {
            already_open,
            ..FakeOpener::default()
        };
        let error = create_from_drawing(
            common::DRAWING.to_vec(),
            "empty.pdf",
            &opener,
            new_project(),
            &mut env,
        )
        .unwrap_err();
        assert!(
            matches!(error, CreateError::Project(ProjectError::InvalidDrawing(_))),
            "no sheets: {error}"
        );
        let released = opener.released.borrow().clone();
        if already_open {
            assert!(released.is_empty(), "the open project still uses it");
        } else {
            assert_eq!(released, [Sha256Hex::of(common::DRAWING)]);
        }
    }
}

#[test]
fn a_new_project_is_autosaved_or_created_without_autosave() {
    let dir = tempfile::tempdir().unwrap();
    let opener = opener();
    let mut env = common::env();
    let autosave_dir = dir.path().join("autosave");
    let options = NewProject {
        autosave: Some(AutosaveTarget {
            dir: autosave_dir.clone(),
            owner: owner(),
        }),
        ..new_project()
    };
    let created = create_from_drawing(
        common::DRAWING.to_vec(),
        "a.pdf",
        &opener,
        options,
        &mut env,
    )
    .unwrap();
    let base = created.session.autosave_path().unwrap().to_owned();
    assert!(base.starts_with(&autosave_dir) && base.is_file());
    assert!(created.lock.is_some() && created.autosave_error.is_none());
    let saved = ProjectFile::load(&base).unwrap().file;
    assert_eq!(saved.project, *created.session.project());

    // A file where the folder should be: the project starts anyway, without autosave.
    let blocked = dir.path().join("blocked");
    std::fs::write(&blocked, b"").unwrap();
    let options = NewProject {
        autosave: Some(AutosaveTarget {
            dir: blocked,
            owner: owner(),
        }),
        ..new_project()
    };
    let created = create_from_drawing(
        common::DRAWING.to_vec(),
        "a.pdf",
        &opener,
        options,
        &mut env,
    )
    .unwrap();
    assert!(created.session.autosave_path().is_none());
    assert!(created.lock.is_none());
    assert!(matches!(
        created.autosave_error,
        Some(ProjectError::Io { .. })
    ));
}

#[test]
fn a_missing_file_is_a_read_error() {
    let mut env = common::env();
    let error = create_from_drawing_file(
        std::path::Path::new("/nonexistent/drawing.pdf"),
        &opener(),
        new_project(),
        &mut env,
    )
    .unwrap_err();
    assert!(matches!(error, CreateError::Read { .. }), "{error}");
}

#[test]
fn the_drawing_of_a_project_opens_through_its_source() {
    let opener = opener();
    let mut env = common::env();
    let created = create_from_drawing(
        common::DRAWING.to_vec(),
        "a.pdf",
        &opener,
        new_project(),
        &mut env,
    )
    .unwrap();
    let drawing = open_drawing(&created.session, &opener).unwrap();
    assert_eq!(
        drawing.source.content_hash(),
        &Sha256Hex::of(common::DRAWING)
    );

    // An image revision (release 0.1) cannot be opened yet.
    let mut file = ProjectFile::new(
        created.session.project().clone(),
        Timestamp::parse("2026-03-01T08:00:00Z").unwrap(),
    );
    file.project.revisions[0].source = DrawingSource::Image {
        format: ImageFormat::Png,
        edit: Box::default(),
    };
    file.insert_drawing(common::DRAWING.to_vec());
    let session = ProjectSession::from_file(file, None);
    assert!(matches!(
        open_drawing(&session, &opener),
        Err(CreateError::ImageNotSupported(ImageFormat::Png))
    ));
}

/// SHA-256 of `corpus/drawings/test_drawing_1.pdf` from `corpus/PROVENANCE.md`.
const TEST_DRAWING_1_SHA256: &str =
    "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

/// T1.3 end to end, now through `dimo-io` itself: the corpus PDF read by PDFium, stored byte
/// identical, saved and reopened with the same sheets.
#[test]
fn test_drawing_1_creates_saves_and_reopens() {
    let Some(engine) = engine() else { return };
    let path = common::repo_root().join("corpus/drawings/test_drawing_1.pdf");
    let bytes = std::fs::read(&path).unwrap();
    let mut env = common::env();
    let created =
        create_from_drawing_file(&path, &engine, NewProject::default(), &mut env).unwrap();
    let mut session = created.session;
    let revision = session.project().current_revision().unwrap().clone();
    assert_eq!(revision.sha256.to_hex(), TEST_DRAWING_1_SHA256);
    assert_eq!(revision.file_name, "test_drawing_1.pdf");
    assert_eq!(revision.sheets.len(), 1);
    assert_eq!(revision.sheets[0].kind, SheetKind::VectorText);

    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("test_drawing_1.dimo");
    session.save_as(&saved, Layout::Zip).unwrap();
    let (reopened, _) = ProjectSession::open(&saved).unwrap();
    assert_eq!(
        reopened.drawing(&revision.sha256).unwrap(),
        bytes.as_slice(),
        "byte identical to the import"
    );
    let source = open_drawing(&reopened, &engine).unwrap().source;
    assert_eq!(source.sheet_sizes().unwrap(), [revision.sheets[0].size]);
}
