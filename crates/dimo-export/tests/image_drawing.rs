//! An image drawing exports the same bytes before saving and after reopening (R.19, rule 11):
//! the project is created from a generated PNG, every list and report format is rendered,
//! the project is saved, reopened and rendered again.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

use dimo_core::{FixedEnvironment, Language, Sha256Hex, Timestamp};
use dimo_export::{BalloonMode, ExportError, ExportFormat, RenderOptions, Source, render};
use dimo_io::project::{Layout, NewProject, ProjectSession, create_from_drawing};
use dimo_pdf::{Opened, PdfError, RasterOpener};
use image::ImageEncoder;

/// Opens images through the default trait method; no PDFs here.
struct ImagesOnly;

impl RasterOpener for ImagesOnly {
    fn open_pdf(&self, _bytes: Vec<u8>) -> Result<Opened, PdfError> {
        Err(PdfError::NoPdfium)
    }

    fn release(&self, _key: &Sha256Hex) {}
}

fn photo() -> Vec<u8> {
    let pixels: Vec<u8> = (0..200u32 * 140)
        .flat_map(|i| [i.to_le_bytes()[0], 90, 200])
        .collect();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&pixels, 200, 140, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn exports(session: &ProjectSession) -> Vec<(ExportFormat, Vec<u8>)> {
    let modified = Timestamp::parse("2026-10-18T08:00:00Z").unwrap();
    let project = session.project();
    let revision = project.current_revision().unwrap();
    let source = Source {
        project,
        modified: &modified,
        drawing: session.drawing(&revision.sha256),
    };
    ExportFormat::ALL
        .into_iter()
        .filter_map(|format| {
            let options = RenderOptions {
                format,
                language: Language::De,
                balloons: BalloonMode::PageContent,
                blank_columns: 3,
            };
            match render(&source, options, None) {
                Err(ExportError::ImageDrawing) => {
                    assert!(format.needs_drawing(), "{format:?}");
                    None
                }
                other => Some((format, other.unwrap())),
            }
        })
        .collect()
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
    let before = exports(&session);
    assert_eq!(
        before.len(),
        ExportFormat::ALL.len() - 1,
        "all but the ballooned PDF"
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.dimo");
    session.save_as(&path, Layout::Zip).unwrap();
    let (reopened, _) = ProjectSession::open(&path).unwrap();
    assert_eq!(exports(&reopened), before);
}
