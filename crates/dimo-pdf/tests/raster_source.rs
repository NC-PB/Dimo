//! The tile service renders any [`RasterSource`], not only PDFs (MF F1). Runs without PDFium:
//! a synthetic in-memory source stands in for the image drawings of release 0.1.

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

use std::sync::Arc;

use dimo_core::{Rect, Sha256Hex, SheetKind, Size};
use dimo_pdf::tiles::{Tile, TileConfig, TileError, TileKey, TileService};
use dimo_pdf::{PdfError, RasterSource, RgbaImage, SourceKind};

const RED: [u8; 3] = [200, 0, 0];
const BLUE: [u8; 3] = [0, 0, 200];
const GREEN: [u8; 3] = [0, 160, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// Two sheets in sheet space: 800 x 600 units, red left of x = 400 and blue right of it, and
/// 300 x 300 units, all green.
#[derive(Debug)]
struct Synthetic {
    hash: Sha256Hex,
    sheets: Vec<Size>,
}

impl Synthetic {
    fn new() -> Self {
        Self {
            hash: Sha256Hex::of(b"synthetic raster source"),
            sheets: vec![
                Size {
                    width: 800.0,
                    height: 600.0,
                },
                Size {
                    width: 300.0,
                    height: 300.0,
                },
            ],
        }
    }

    fn color(sheet: usize, size: Size, x: f64, y: f64) -> [u8; 3] {
        if x < 0.0 || y < 0.0 || x >= size.width || y >= size.height {
            WHITE
        } else if sheet == 1 {
            GREEN
        } else if x < 400.0 {
            RED
        } else {
            BLUE
        }
    }
}

impl RasterSource for Synthetic {
    fn kind(&self) -> SourceKind {
        SourceKind::Image
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
            .copied()
            .ok_or(PdfError::SheetOutOfRange {
                index: sheet,
                count: self.sheets.len(),
            })
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "test pixel sizes are small and positive"
    )]
    fn render_region(&self, sheet: usize, region: Rect, zoom: f64) -> Result<RgbaImage, PdfError> {
        let size = self.sheet_size(sheet)?;
        let width = (region.size.width * zoom).round() as u32;
        let height = (region.size.height * zoom).round() as u32;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for j in 0..height {
            for i in 0..width {
                let x = region.origin.x + (f64::from(i) + 0.5) / zoom;
                let y = region.origin.y + (f64::from(j) + 0.5) / zoom;
                pixels.extend_from_slice(&Self::color(sheet, size, x, y));
                pixels.push(255);
            }
        }
        Ok(RgbaImage::from_raw(width, height, pixels).unwrap())
    }

    fn sheet_kind(&self, _sheet: usize) -> Result<SheetKind, PdfError> {
        Ok(SheetKind::Raster)
    }
}

fn key(doc: Sha256Hex, sheet: u32, zoom: i32, x: u32, y: u32) -> TileKey {
    TileKey {
        doc,
        sheet,
        zoom,
        x,
        y,
    }
}

/// Width, height and RGB pixels of a tile.
fn decode(tile: &Tile) -> (u32, u32, Vec<[u8; 3]>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(tile.as_bytes()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Grayscale => buf.iter().map(|&v| [v, v, v]).collect(),
        png::ColorType::Rgb => buf.as_chunks::<3>().0.to_vec(),
        png::ColorType::Rgba => buf
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect(),
        other => panic!("unexpected colour type {other:?}"),
    };
    (info.width, info.height, pixels)
}

fn pixel(decoded: &(u32, u32, Vec<[u8; 3]>), x: u32, y: u32) -> [u8; 3] {
    decoded.2[(y * decoded.0 + x) as usize]
}

#[test]
fn a_synthetic_source_renders_tiles_through_the_service() {
    let service = TileService::without_pdf(TileConfig::default());
    let source = Synthetic::new();
    let doc = source.hash;
    let registered = service.insert_source(Arc::new(source));
    assert_eq!(registered.kind(), SourceKind::Image);
    assert!(service.document(&doc).is_none(), "not a PDF document");
    assert_eq!(service.source(&doc).unwrap().sheet_count(), 2);

    // Zoom 0 is one pixel per sheet unit: sheet 0 is 2 x 2 tiles of at most 512 pixels.
    let first = decode(&service.get(key(doc, 0, 0, 0, 0)).unwrap());
    assert_eq!((first.0, first.1), (512, 512));
    assert_eq!(pixel(&first, 10, 10), RED);
    assert_eq!(pixel(&first, 399, 300), RED);
    assert_eq!(pixel(&first, 400, 300), BLUE);
    let right = decode(&service.get(key(doc, 0, 0, 1, 1)).unwrap());
    assert_eq!(
        (right.0, right.1),
        (288, 88),
        "edge tiles end at the sheet edge"
    );
    assert_eq!(pixel(&right, 0, 0), BLUE);

    // Zoom 1: two pixels per unit, the colour edge moves to pixel 800.
    let zoomed = decode(&service.get(key(doc, 0, 1, 1, 0)).unwrap());
    assert_eq!(pixel(&zoomed, 800 - 512 - 1, 0), RED);
    assert_eq!(pixel(&zoomed, 800 - 512, 0), BLUE);

    let second = decode(&service.get(key(doc, 1, 0, 0, 0)).unwrap());
    assert_eq!((second.0, second.1), (300, 300));
    assert_eq!(pixel(&second, 299, 299), GREEN);

    let renders = service.stats().renders;
    assert_eq!(renders, 4);
    service.get(key(doc, 0, 0, 0, 0)).unwrap();
    assert_eq!(
        service.stats().renders,
        renders,
        "served from the memory cache"
    );
    assert_eq!(service.stats().memory_hits, 1);

    assert!(matches!(
        service.get(key(doc, 2, 0, 0, 0)),
        Err(TileError::OutOfRange(_))
    ));
    assert!(matches!(
        service.get(key(Sha256Hex::of(b"other"), 0, 0, 0, 0)),
        Err(TileError::UnknownDocument(_))
    ));
    assert!(service.close_document(&doc));
    assert!(matches!(
        service.get(key(doc, 0, 0, 0, 0)),
        Err(TileError::UnknownDocument(_))
    ));
}

#[test]
fn the_disk_cache_keeps_tiles_of_other_sources() {
    let dir = tempfile::tempdir().unwrap();
    let config = || TileConfig {
        disk_dir: Some(dir.path().to_owned()),
        ..TileConfig::default()
    };
    let doc = Synthetic::new().hash;
    let tile = {
        let service = TileService::without_pdf(config());
        service.insert_source(Arc::new(Synthetic::new()));
        service.get(key(doc, 1, -1, 0, 0)).unwrap()
    };
    let service = TileService::without_pdf(config());
    service.insert_source(Arc::new(Synthetic::new()));
    assert_eq!(service.get(key(doc, 1, -1, 0, 0)).unwrap(), tile);
    let stats = service.stats();
    assert_eq!((stats.disk_hits, stats.renders), (1, 0));
}

#[test]
fn a_service_without_pdfium_refuses_pdfs() {
    let service = TileService::without_pdf(TileConfig::default());
    assert!(service.engine().is_none());
    assert!(matches!(
        service.open_document(b"%PDF-1.7".to_vec()),
        Err(PdfError::EngineStopped)
    ));
}
