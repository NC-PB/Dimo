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
