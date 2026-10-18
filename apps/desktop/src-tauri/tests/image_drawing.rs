//! A photo as drawing in the app session (R.19): it opens without PDFium, its tiles come from
//! the image, and a box drawn on it finds no text, so the box falls back to manual placement
//! like a PDF without text instead of failing.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

use dimo_core::Language;
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::proposal::BalloonPlacement;
use dimo_desktop::recognition::propose;
use dimo_desktop::session::AppSession;
use dimo_pdf::SourceKind;
use dimo_pdf::tiles::{TileConfig, TileService};
use image::ImageEncoder;

fn photo() -> Vec<u8> {
    let pixels: Vec<u8> = (0..320u32 * 240)
        .map(|i| if i % 320 < 160 { 0 } else { 255 })
        .collect();
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&pixels, 320, 240, image::ExtendedColorType::L8)
        .unwrap();
    out
}

#[test]
fn a_box_on_a_photo_finds_no_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.png");
    std::fs::write(&path, photo()).unwrap();
    let tiles = TileService::without_pdf(TileConfig::default());
    let mut session = AppSession::new(None);
    session.create_from_drawing(&tiles, &path, false).unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let target = session.recognition_target(sheet).unwrap();
    let source = tiles.source(&target.drawing).unwrap();
    assert_eq!(source.kind(), SourceKind::Image);

    let region = OrientedBox {
        center: Point { x: 200.0, y: 150.0 },
        size: Size {
            width: 80.0,
            height: 30.0,
        },
        angle: 0.0,
    };
    let placement = BalloonPlacement {
        position: Point { x: 260.0, y: 110.0 },
        anchor: Point { x: 240.0, y: 135.0 },
    };
    let views = propose(&tiles, &target, sheet, region, placement, Language::En).unwrap();
    assert!(
        views.is_empty(),
        "no text in an image: {} proposals",
        views.len()
    );
}

#[test]
fn the_image_editor_applies_asks_and_undoes_through_the_session() {
    // R.20: apply derives the sheet in Rust, asks before balloons leave the sheet, and the
    // drawing follows the settings through undo and redo.
    use dimo_core::{Command, ImageEdit, Paper, PaperSize, Rotation, Trim};
    use dimo_desktop::image_edit::{ImagePreviewState, PreviewStage, preview};
    use dimo_desktop::session::ImageEditOutcome;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.png");
    std::fs::write(&path, photo()).unwrap();
    let tiles = TileService::without_pdf(TileConfig::default());
    let mut session = AppSession::new(None);
    session.create_from_drawing(&tiles, &path, false).unwrap();
    let first_key = session.drawing_key().unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    session
        .execute(Command::AddCharacteristic {
            sheet,
            position: Point { x: 700.0, y: 500.0 },
            anchor: Point { x: 690.0, y: 520.0 },
            region: None,
            values: vec![],
            insert_after: None,
        })
        .unwrap();

    // The frontend cannot send an edit with its own geometry.
    let revision = session.project().unwrap().current_revision;
    let refused = session.execute(Command::SetImageEdit {
        revision,
        page: 0,
        edit: Box::new(ImageEdit::default()),
        geometry: None,
    });
    assert!(refused.is_err());

    // Trimming to the left half moves the balloon at x = 700 off the sheet: asked first.
    let left_half = ImageEdit {
        trim: Some(Trim {
            left: "0".parse().unwrap(),
            top: "0".parse().unwrap(),
            right: "0.5".parse().unwrap(),
            bottom: "1".parse().unwrap(),
        }),
        ..ImageEdit::default()
    };
    let asked = session
        .apply_image_edit(&tiles, 0, &left_half, false)
        .unwrap();
    assert_eq!(asked, ImageEditOutcome::OffSheet { balloons: 1 });
    assert_eq!(session.drawing_key(), Some(first_key), "nothing changed");

    let turned = ImageEdit {
        rotation: Rotation::Deg90,
        paper: Some(Paper::Standard {
            size: PaperSize::A4,
            orientation: dimo_core::Orientation::Portrait,
        }),
        ..ImageEdit::default()
    };
    let ImageEditOutcome::Applied { patched } =
        session.apply_image_edit(&tiles, 0, &turned, false).unwrap()
    else {
        panic!("applied");
    };
    let drawing = patched.drawing.unwrap();
    let key = session.drawing_key().unwrap();
    assert_ne!(key, first_key);
    assert_eq!(drawing.doc, key.to_hex());
    assert!(tiles.source(&key).is_some(), "the new drawing serves tiles");
    assert!(
        tiles.source(&first_key).is_none(),
        "the old one is released"
    );
    let size = &session.project().unwrap().revisions[0].sheets[0].size;
    assert!((size.width - drawing.sheets[0].width).abs() < 1e-9);

    session.undo().unwrap();
    assert_eq!(
        session.sync_drawing(&tiles).unwrap().unwrap().doc,
        first_key.to_hex()
    );
    assert!(tiles.source(&first_key).is_some());
    session.redo().unwrap();
    assert_eq!(
        session.sync_drawing(&tiles).unwrap().unwrap().doc,
        key.to_hex()
    );
    assert_eq!(session.sync_drawing(&tiles).unwrap(), None, "in step");

    // Previews come and go without touching the drawing.
    let source = session.image_source().unwrap();
    let state = ImagePreviewState::default();
    let corners = preview(&tiles, source.as_ref(), 0, &turned, PreviewStage::Corners).unwrap();
    let corners_key = dimo_core::Sha256Hex::from_hex(&corners.doc.doc).unwrap();
    state.replace(&tiles, Some(corners_key), Some(key));
    let result = preview(&tiles, source.as_ref(), 0, &turned, PreviewStage::Result).unwrap();
    assert_eq!(
        result.doc.doc,
        key.to_hex(),
        "unchanged settings show the drawing"
    );
    state.replace(&tiles, Some(key), Some(key));
    assert!(
        tiles.source(&corners_key).is_none(),
        "the old preview is closed"
    );
    state.replace(&tiles, None, Some(key));
    assert!(tiles.source(&key).is_some(), "the drawing stays open");
    // The corner view is the turned page: the page's top left is the view's top right.
    let p = corners.to_view.apply(Point { x: 0.0, y: 0.0 }).unwrap();
    assert!((p.x - corners.doc.sheets[0].width).abs() < 1e-6 && p.y.abs() < 1e-6);
}
