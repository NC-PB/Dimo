//! Photos and scans become projects (R.19, FR-DOC-02): JPEG, PNG and multi-page TIFF are opened
//! through the image source, stored unchanged, saved and reopened byte identical (rule 11).
//! Runs without PDFium: the default `RasterOpener::open_image` decodes the images.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use std::io::Cursor;
use std::sync::Arc;

use dimo_core::{DrawingSource, ImageEdit, ImageFormat, SheetKind};
use dimo_io::project::{
    CreatedProject, Layout, NewProject, ProjectSession, create_from_drawing_file, open_drawing,
    plan_image_edit,
};
use dimo_pdf::{Opened, PdfError, RasterOpener, RasterSource};
use image::{ImageEncoder, RgbImage};
use tiff::encoder::{Rational, TiffEncoder, colortype};
use tiff::tags::ResolutionUnit;

/// Opens images with the default trait method; PDFs are not used here.
struct ImagesOnly;

impl RasterOpener for ImagesOnly {
    fn open_pdf(&self, _bytes: Vec<u8>) -> Result<Opened, PdfError> {
        Err(PdfError::NoPdfium)
    }

    fn release(&self, _key: &dimo_core::Sha256Hex) {}
}

fn page(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([x.to_le_bytes()[0], y.to_le_bytes()[0], 128])
    })
}

fn jpeg() -> Vec<u8> {
    let image = page(640, 480);
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
        .write_image(image.as_raw(), 640, 480, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn png() -> Vec<u8> {
    let image = page(300, 420);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(image.as_raw(), 300, 420, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn tiff_3_pages() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut out).unwrap();
    for (width, height) in [(420u32, 297u32), (297, 420), (210, 297)] {
        let grey = vec![200u8; (width * height) as usize];
        let mut page = encoder
            .new_image::<colortype::Gray8>(width, height)
            .unwrap();
        page.resolution(ResolutionUnit::Centimeter, Rational { n: 100, d: 1 });
        page.write_data(&grey).unwrap();
    }
    out.into_inner()
}

fn create(name: &str, bytes: &[u8]) -> (CreatedProject, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    let mut env = common::env();
    let created =
        create_from_drawing_file(&path, &ImagesOnly, NewProject::default(), &mut env).unwrap();
    (created, dir)
}

/// Saves, reopens and saves again; both files must be identical, and the reopened drawing
/// must open with the same sheets.
fn saves_byte_identical(created: CreatedProject, dir: &std::path::Path) -> ProjectSession {
    let mut session = created.session;
    let first = dir.join("first.dimo");
    session.save_as(&first, Layout::Zip).unwrap();
    let (mut reopened, _) = ProjectSession::open(&first).unwrap();
    let second = dir.join("second.dimo");
    reopened.save_as(&second, Layout::Zip).unwrap();
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap(),
        "save, reopen, save is byte identical"
    );
    let revision = reopened.project().current_revision().unwrap().clone();
    let source = open_drawing(&reopened, &ImagesOnly).unwrap().source;
    let sizes: Vec<_> = revision.sheets.iter().map(|s| s.size).collect();
    assert_eq!(source.sheet_sizes().unwrap(), sizes);
    assert_eq!(source.render_key(), created.source.render_key());
    reopened
}

#[test]
fn a_jpeg_photo_becomes_a_project() {
    let bytes = jpeg();
    let (created, dir) = create("photo.jpg", &bytes);
    let revision = created
        .session
        .project()
        .current_revision()
        .unwrap()
        .clone();
    assert_eq!(revision.file_name, "photo.jpg");
    assert_eq!(revision.sha256, dimo_core::Sha256Hex::of(&bytes));
    assert_eq!(revision.stored_name(), format!("{}.jpg", revision.sha256));
    assert_eq!(
        revision.source,
        DrawingSource::Image {
            format: ImageFormat::Jpeg,
            pages: vec![ImageEdit::default()],
        }
    );
    assert_eq!(revision.sheets.len(), 1);
    assert_eq!(revision.sheets[0].kind, SheetKind::Raster);
    assert!(revision.sheets[0].raster_dpi.is_some());
    assert_eq!(
        created.session.drawing(&revision.sha256),
        Some(bytes.as_slice()),
        "stored unchanged"
    );
    let reopened = saves_byte_identical(created, dir.path());
    assert_eq!(reopened.drawing(&revision.sha256), Some(bytes.as_slice()));
}

#[test]
fn a_png_becomes_a_project() {
    let (created, dir) = create("scan.png", &png());
    let revision = created
        .session
        .project()
        .current_revision()
        .unwrap()
        .clone();
    assert!(matches!(
        revision.source,
        DrawingSource::Image {
            format: ImageFormat::Png,
            ..
        }
    ));
    assert!(revision.sheets[0].size.height > revision.sheets[0].size.width);
    saves_byte_identical(created, dir.path());
}

#[test]
fn a_three_page_tiff_becomes_a_project_with_three_sheets() {
    let (created, dir) = create("scan.tif", &tiff_3_pages());
    let revision = created
        .session
        .project()
        .current_revision()
        .unwrap()
        .clone();
    assert_eq!(revision.sheets.len(), 3);
    assert_eq!(revision.source.image_pages().unwrap().len(), 3);
    assert_eq!(revision.stored_name(), format!("{}.tiff", revision.sha256));
    // 100 pixels per cm: 42 x 29.7 mm and so on.
    let mm = |units: f64| units / dimo_core::UNITS_PER_MM;
    let first = revision.sheets[0].size;
    assert!((mm(first.width) - 42.0).abs() < 1e-6, "{first:?}");
    assert!((mm(first.height) - 29.7).abs() < 1e-6);
    let indices: Vec<u32> = revision.sheets.iter().map(|s| s.index).collect();
    assert_eq!(indices, [0, 1, 2]);
    assert_eq!(revision.sheets[0].raster_dpi, Some(254));
    saves_byte_identical(created, dir.path());
}

#[test]
fn reopening_uses_the_stored_edit_settings() {
    let (created, dir) = create("photo.png", &png());
    let mut session = created.session;
    let turned = ImageEdit {
        rotation: dimo_core::Rotation::Deg90,
        ..ImageEdit::default()
    };
    let mut env = common::env();
    let plan = plan_image_edit(session.project(), created.source.as_ref(), 0, &turned).unwrap();
    session.execute(plan.command, &mut env).unwrap();
    let path = dir.path().join("turned.dimo");
    session.save_as(&path, Layout::Zip).unwrap();
    let (reopened, _) = ProjectSession::open(&path).unwrap();
    let source: Arc<dyn RasterSource> = open_drawing(&reopened, &ImagesOnly).unwrap().source;
    assert_ne!(source.render_key(), created.source.render_key());
    let size = source.sheet_size(0).unwrap();
    let before = created.source.sheet_size(0).unwrap();
    assert!(
        (size.width - before.height).abs() < 1e-9,
        "the turn swaps the sheet"
    );
    // R.20: the project sheet follows the edit, nothing drifts.
    let sheet = &reopened.project().current_revision().unwrap().sheets[0];
    assert_eq!(sheet.size, size);
}

#[test]
fn an_edit_keeps_balloons_on_their_features_and_undo_restores_it() {
    // R.20 acceptance: turn, straighten, trim and put on A4; the balloon still points at the
    // same pixel of the photo, and undo gives back edit, sheet and balloon.
    let (created, _dir) = create("photo.png", &png());
    let mut session = created.session;
    let source = created.source;
    let image = source.as_image().unwrap();
    let mut env = common::env();
    let sheet_id = session.project().current_revision().unwrap().sheets[0].id;
    // The pixel (100, 200) of the photo has the colour (100, 200, 128).
    let feature = dimo_core::Point { x: 100.5, y: 200.5 };
    let on_sheet = image
        .page_geometry(0)
        .unwrap()
        .to_sheet
        .apply(feature)
        .unwrap();
    session
        .execute(
            dimo_core::Command::AddCharacteristic {
                sheet: sheet_id,
                position: dimo_core::Point {
                    x: on_sheet.x + 20.0,
                    y: on_sheet.y - 20.0,
                },
                anchor: on_sheet,
                region: None,
                values: vec![],
                insert_after: None,
            },
            &mut env,
        )
        .unwrap();
    let before = session.project().clone();
    let edit = ImageEdit {
        rotation: dimo_core::Rotation::Deg90,
        fine_angle: "3.5".parse().unwrap(),
        trim: Some(dimo_core::Trim {
            left: "0.05".parse().unwrap(),
            top: "0.1".parse().unwrap(),
            right: "0.9".parse().unwrap(),
            bottom: "0.95".parse().unwrap(),
        }),
        paper: Some(dimo_core::Paper::Standard {
            size: dimo_core::PaperSize::A4,
            orientation: dimo_core::Orientation::Portrait,
        }),
        ..ImageEdit::default()
    };
    let plan = plan_image_edit(session.project(), source.as_ref(), 0, &edit).unwrap();
    assert_eq!(plan.off_sheet, 0);
    session.execute(plan.command, &mut env).unwrap();

    let edited = image.with_edits(std::slice::from_ref(&edit)).unwrap();
    let project = session.project();
    let sheet = &project.current_revision().unwrap().sheets[0];
    assert_eq!(sheet.size, edited.sheet_size(0).unwrap());
    assert_eq!(sheet.raster_dpi, edited.raster_dpi(0));
    let anchor = project.balloons[0].anchor;
    let back = edited.page_point(0, anchor).unwrap().unwrap();
    assert!(
        (back.x - feature.x).abs() < 1e-6 && (back.y - feature.y).abs() < 1e-6,
        "{back:?}"
    );
    let tile = edited
        .render_region(
            0,
            dimo_core::Rect::new(anchor.x - 0.05, anchor.y - 0.05, 0.1, 0.1),
            10.0,
        )
        .unwrap();
    let [r, g, b, _] = tile.pixel(0, 0).unwrap();
    assert!(
        r.abs_diff(100) <= 3 && g.abs_diff(200) <= 3 && b == 128,
        "{r} {g} {b}"
    );

    session.document_mut().undo(&mut env).unwrap();
    assert_eq!(
        *session.project(),
        before,
        "undo restores edit, sheet and balloon"
    );
    session.document_mut().redo(&mut env).unwrap();
    assert_eq!(session.project().balloons[0].anchor, anchor);
}

#[test]
fn a_clean_up_only_edit_leaves_the_sheet_and_balloons_alone() {
    // R.20 review: a balloon on the sheet edge is not counted as leaving the sheet, and no
    // balloon changes, when only the clean up is switched.
    let (created, _dir) = create("photo.png", &png());
    let mut session = created.session;
    let source = created.source;
    let mut env = common::env();
    let sheet = session.project().current_revision().unwrap().sheets[0].clone();
    session
        .execute(
            dimo_core::Command::AddCharacteristic {
                sheet: sheet.id,
                position: dimo_core::Point {
                    x: sheet.size.width,
                    y: 10.0,
                },
                anchor: dimo_core::Point {
                    x: sheet.size.width,
                    y: sheet.size.height,
                },
                region: None,
                values: vec![],
                insert_after: None,
            },
            &mut env,
        )
        .unwrap();
    let cleaned = ImageEdit {
        clean_up: true,
        ..ImageEdit::default()
    };
    let plan = plan_image_edit(session.project(), source.as_ref(), 0, &cleaned).unwrap();
    assert_eq!(plan.off_sheet, 0);
    assert!(matches!(
        plan.command,
        dimo_core::Command::SetImageEdit { geometry: None, .. }
    ));
    let patch = session.execute(plan.command, &mut env).unwrap();
    assert!(matches!(
        patch.changes[..],
        [dimo_core::Change::RevisionChanged { .. }]
    ));
}
