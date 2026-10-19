//! An image drawing exports like a PDF drawing (R.19, R.21, rule 11): the project is created
//! from a generated photo, set to A4 landscape and given two balloons; every format, the
//! ballooned PDF included, is rendered, the project is saved, reopened and rendered again.
//!
//! The ballooned PDF needs PDFium. Without it locally the test checks the other formats only
//! and prints SKIPPED; with `CI=true` or `DIMO_REQUIRE_PDFIUM=1` it fails instead.

#![allow(clippy::unwrap_used, clippy::print_stderr)] // Test crate (rust.md).

use dimo_core::geometry::{Point, Size};
use dimo_core::{
    Command, FixedEnvironment, ImageEdit, Language, Orientation, Paper, PaperSize, Sha256Hex,
    Timestamp, UNITS_PER_MM,
};
use dimo_export::{BalloonMode, ExportError, ExportFormat, RenderOptions, Source, render};
use dimo_io::project::{
    Layout, NewProject, ProjectSession, create_from_drawing, open_drawing, plan_image_edit,
};
use dimo_pdf::{Opened, PdfEngine, PdfError, RasterOpener};
use image::ImageEncoder;

/// Opens images through the default trait method; no PDFs here.
struct ImagesOnly;

impl RasterOpener for ImagesOnly {
    fn open_pdf(&self, _bytes: Vec<u8>) -> Result<Opened, PdfError> {
        Err(PdfError::NoPdfium)
    }

    fn release(&self, _key: &Sha256Hex) {}
}

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn engine() -> Option<PdfEngine> {
    match PdfEngine::start() {
        Ok(engine) => Some(engine),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED ballooned PDF (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn photo() -> Vec<u8> {
    let pixels: Vec<u8> = (0..300u32 * 210)
        .flat_map(|i| [i.to_le_bytes()[0], 90, 200])
        .collect();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&pixels, 300, 210, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn exports(session: &ProjectSession, engine: Option<&PdfEngine>) -> Vec<(ExportFormat, Vec<u8>)> {
    let modified = Timestamp::parse("2026-10-19T08:00:00Z").unwrap();
    let project = session.project();
    let revision = project.current_revision().unwrap();
    let source = Source {
        project,
        modified: &modified,
        drawing: session.drawing(&revision.sha256),
    };
    let mut out = Vec::new();
    for format in ExportFormat::ALL {
        for balloons in [BalloonMode::PageContent, BalloonMode::Annotations] {
            if balloons == BalloonMode::Annotations && !format.needs_drawing() {
                continue;
            }
            let options = RenderOptions {
                format,
                language: Language::De,
                balloons,
                blank_columns: 3,
            };
            match render(&source, options, engine) {
                Err(ExportError::NoPdfEngine) => {
                    assert!(engine.is_none() && format.needs_drawing());
                }
                other => out.push((format, other.unwrap())),
            }
        }
    }
    out
}

fn add_balloon(session: &mut ProjectSession, env: &mut FixedEnvironment, x: f64, y: f64) {
    let sheet = session.project().current_revision().unwrap().sheets[0].id;
    session
        .execute(
            Command::AddCharacteristic {
                sheet,
                position: Point { x, y },
                anchor: Point {
                    x: x - 20.0,
                    y: y + 15.0,
                },
                region: None,
                values: Vec::new(),
                insert_after: None,
            },
            env,
        )
        .unwrap();
}

#[test]
fn an_image_project_exports_the_same_bytes_after_save_and_reopen() {
    let mut env = FixedEnvironment::new();
    env.set_user("tester");
    let created = create_from_drawing(
        photo(),
        "photo.png",
        &ImagesOnly,
        NewProject::default(),
        &mut env,
    )
    .unwrap();
    let mut session = created.session;
    let a4 = ImageEdit {
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: Orientation::Landscape,
        }),
        ..ImageEdit::default()
    };
    let plan = plan_image_edit(session.project(), created.source.as_ref(), 0, &a4).unwrap();
    session.execute(plan.command, &mut env).unwrap();
    add_balloon(&mut session, &mut env, 300.0, 200.0);
    add_balloon(&mut session, &mut env, 650.0, 450.0);

    let engine = engine();
    let before = exports(&session, engine.as_ref());
    let expected = if engine.is_some() { 8 } else { 6 };
    assert_eq!(before.len(), expected, "every format, both balloon modes");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.dimo");
    session.save_as(&path, Layout::Zip).unwrap();
    let (reopened, _) = ProjectSession::open(&path).unwrap();
    // The reopened drawing has the stored edit (A4) and the sheets the project records.
    let source = open_drawing(&reopened, &ImagesOnly).unwrap().source;
    let sheet = reopened.project().current_revision().unwrap().sheets[0].size;
    assert_eq!(source.sheet_size(0).unwrap(), sheet);
    assert_eq!(
        exports(&reopened, engine.as_ref()),
        before,
        "same bytes after reopen"
    );

    // R.21: the ballooned PDF prints at the chosen paper size with the balloons in place.
    let Some(engine) = engine else { return };
    let (_, ballooned) = before
        .iter()
        .find(|(format, _)| *format == ExportFormat::BalloonedPdf)
        .unwrap();
    let doc = engine.open(ballooned.clone()).unwrap();
    assert_eq!(doc.sheet_count(), 1);
    let size: Size = doc.sheet_size(0).unwrap();
    let near = |a: f64, b: f64, tolerance: f64| (a - b).abs() < tolerance;
    assert!(
        near(size.width, 297.0 * UNITS_PER_MM, 1e-3)
            && near(size.height, 210.0 * UNITS_PER_MM, 1e-3),
        "A4 landscape: {size:?}"
    );
    let runs = doc.text_runs(0).unwrap();
    for (number, x, y) in [("1", 300.0, 200.0), ("2", 650.0, 450.0)] {
        let run = runs
            .iter()
            .find(|r| r.text.trim() == number)
            .unwrap_or_else(|| panic!("balloon {number} missing"));
        let b = run.bbox;
        let (cx, cy) = (
            b.origin.x + b.size.width / 2.0,
            b.origin.y + b.size.height / 2.0,
        );
        assert!(
            near(cx, x, 6.0) && near(cy, y, 6.0),
            "balloon {number} at {cx}, {cy}"
        );
    }
}
