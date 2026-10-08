//! Tile service against the real PDFium library (T0.7).
//!
//! Like `pdfium.rs`, tests skip with a message when PDFium is missing, unless `CI=true` or
//! `DIMO_REQUIRE_PDFIUM=1` is set.

// Test code (rust.md): unwrap is fine in helpers too, and the skip message must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;
use std::sync::mpsc;

use dimo_pdf::tiles::{
    TILE_SIZE, Tile, TileConfig, TileError, TileGrid, TileKey, TileRange, TileService, zoom_scale,
};
use dimo_pdf::{ContentHash, PdfEngine, PdfError, SheetRect};

const TEST_DRAWING_1: &str = "test_drawing_1.pdf";

fn corpus_drawing(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/drawings")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

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

fn service(config: TileConfig) -> Option<(TileService, ContentHash)> {
    let service = TileService::new(engine()?, config);
    let doc = service
        .open_document(corpus_drawing(TEST_DRAWING_1))
        .unwrap();
    let hash = *doc.content_hash();
    Some((service, hash))
}

fn key(doc: ContentHash, zoom: i32, x: u32, y: u32) -> TileKey {
    TileKey {
        doc,
        sheet: 0,
        zoom,
        x,
        y,
    }
}

/// Every tile of a sheet at the zoom levels `zooms`.
fn all_tiles(service: &TileService, doc: ContentHash, zooms: &[i32]) -> Vec<TileKey> {
    let size = service.document(&doc).unwrap().sheet_size(0).unwrap();
    let mut keys = Vec::new();
    for &zoom in zooms {
        let grid = TileGrid::new(size, zoom).unwrap();
        for y in 0..grid.rows() {
            for x in 0..grid.columns() {
                keys.push(key(doc, zoom, x, y));
            }
        }
    }
    keys
}

/// Decodes a tile to RGB.
fn decode(tile: &Tile) -> (u32, u32, Vec<[u8; 3]>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(tile.as_bytes()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Grayscale => buf.iter().map(|&v| [v, v, v]).collect(),
        png::ColorType::Rgb => buf.as_chunks::<3>().0.to_vec(),
        other => panic!("unexpected colour type {other:?}"),
    };
    (info.width, info.height, pixels)
}

/// Tiles put together equal one render of the whole sheet (rule 4, no seams). Text crosses
/// tile edges at both levels, which catches the PDFium glyph edge quirk (`render_margin`).
#[test]
fn tiles_join_into_the_full_sheet() {
    let Some((service, doc)) = service(TileConfig::default()) else {
        return;
    };
    let document = service.document(&doc).unwrap();
    let size = document.sheet_size(0).unwrap();
    for (zoom, columns, rows) in [(0, 4, 3), (1, 7, 5)] {
        let grid = TileGrid::new(size, zoom).unwrap();
        assert_eq!((grid.columns(), grid.rows()), (columns, rows));
        let full = document
            .render_region(0, SheetRect::full(size), zoom_scale(zoom))
            .unwrap();
        let mut max_diff = 0u8;
        for k in all_tiles(&service, doc, &[zoom]) {
            let (width, height, pixels) = decode(&service.get(k).unwrap());
            assert_eq!(Some((width, height)), grid.tile_pixels(k.x, k.y));
            for (index, tile_px) in pixels.iter().enumerate() {
                let index = u32::try_from(index).unwrap();
                let sheet_x = k.x * TILE_SIZE + index % width;
                let sheet_y = k.y * TILE_SIZE + index / width;
                let full_px = full.pixel(sheet_x, sheet_y).unwrap();
                for c in 0..3 {
                    max_diff = max_diff.max(tile_px[c].abs_diff(full_px[c]));
                }
            }
        }
        assert!(
            max_diff <= 2,
            "zoom {zoom}: tiles differ from full render by {max_diff}"
        );
    }
}

/// T0.7 acceptance: the memory cache never exceeds its byte budget under concurrent load.
#[test]
fn memory_budget_holds_under_stress() {
    let budget = 256 * 1024;
    let Some((service, doc)) = service(TileConfig {
        memory_budget: budget,
        workers: 4,
        ..TileConfig::default()
    }) else {
        return;
    };
    let keys = all_tiles(&service, doc, &[-2, -1, 0, 1]);
    assert_eq!(keys.len(), 1 + 4 + 12 + 35);
    std::thread::scope(|s| {
        for t in 0..8 {
            let (service, keys) = (&service, &keys);
            s.spawn(move || {
                for round in 0..3 {
                    // Each thread walks the tiles in its own order.
                    let shift = (t * 7 + round * 13) % keys.len();
                    for k in keys.iter().cycle().skip(shift).take(keys.len()) {
                        let tile = service.get(*k).unwrap();
                        assert!(!tile.is_empty());
                        let stats = service.stats();
                        assert!(stats.memory_bytes <= budget, "{stats:?}");
                    }
                }
            });
        }
    });
    let stats = service.stats();
    eprintln!("{stats:?}");
    assert!(stats.memory_peak_bytes <= budget, "{stats:?}");
    // The budget is too small for all tiles, so tiles were evicted and rendered again.
    assert!(stats.memory_tiles < keys.len(), "{stats:?}");
    assert!(stats.renders > keys.len() as u64, "{stats:?}");
    assert!(stats.memory_hits > 0, "{stats:?}");
    assert_eq!(stats.queued, 0);
}

#[test]
fn disk_cache_serves_a_new_service() {
    let dir = tempfile::tempdir().unwrap();
    let config = TileConfig {
        disk_dir: Some(dir.path().to_path_buf()),
        ..TileConfig::default()
    };
    let Some((first, doc)) = service(config.clone()) else {
        return;
    };
    let k = key(doc, 1, 2, 1);
    let rendered = first.get(k).unwrap();
    assert_eq!(first.stats().renders, 1);
    drop(first);

    let (second, _) = service(config).unwrap();
    assert_eq!(second.get(k).unwrap(), rendered);
    let stats = second.stats();
    assert_eq!((stats.renders, stats.disk_hits), (0, 1), "{stats:?}");
    // Now it is in memory as well.
    assert_eq!(second.get(k).unwrap(), rendered);
    assert_eq!(second.stats().memory_hits, 1);
}

/// Queued requests outside the declared interest are cancelled, the rest stay queued.
#[test]
fn interest_cancels_queued_requests() {
    // No workers: requests stay queued until cancelled or the service stops.
    let Some((service, doc)) = service(TileConfig {
        workers: 0,
        ..TileConfig::default()
    }) else {
        return;
    };
    let (tx, rx) = mpsc::channel();
    for x in 0..7 {
        let tx = tx.clone();
        service.request(key(doc, 1, x, 0), move |result| {
            tx.send((x, result)).unwrap();
        });
    }
    assert_eq!(service.stats().queued, 7);
    service.set_interest(
        doc,
        Some(vec![TileRange {
            sheet: 0,
            zoom: 1,
            columns: 2..4,
            rows: 0..1,
        }]),
    );
    let mut cancelled: Vec<u32> = rx
        .try_iter()
        .map(|(x, result)| {
            assert_eq!(result, Err(TileError::Cancelled));
            x
        })
        .collect();
    cancelled.sort_unstable();
    assert_eq!(cancelled, vec![0, 1, 4, 5, 6]);
    assert_eq!(service.stats().queued, 2);
    assert_eq!(service.stats().cancelled, 5);

    drop(service);
    let mut stopped: Vec<u32> = rx
        .try_iter()
        .map(|(x, result)| {
            assert_eq!(result, Err(TileError::Stopped));
            x
        })
        .collect();
    stopped.sort_unstable();
    assert_eq!(stopped, vec![2, 3]);
}

/// A worker skips requests that left the interest before it took them.
#[test]
fn interest_applies_to_later_requests() {
    let Some((service, doc)) = service(TileConfig::default()) else {
        return;
    };
    service.set_interest(
        doc,
        Some(vec![TileRange {
            sheet: 0,
            zoom: 0,
            columns: 0..1,
            rows: 0..1,
        }]),
    );
    assert!(service.get(key(doc, 0, 0, 0)).is_ok());
    assert_eq!(service.get(key(doc, 0, 1, 0)), Err(TileError::Cancelled));
    service.set_interest(doc, None);
    assert!(service.get(key(doc, 0, 1, 0)).is_ok());
}

#[test]
fn invalid_requests_fail_fast() {
    let Some((service, doc)) = service(TileConfig {
        workers: 0,
        ..TileConfig::default()
    }) else {
        return;
    };
    let unknown = ContentHash::of(b"not open");
    assert!(matches!(
        service.get(key(unknown, 0, 0, 0)),
        Err(TileError::UnknownDocument(_))
    ));
    for k in [
        key(doc, 0, 4, 0),
        key(doc, 0, 0, 3),
        key(doc, 99, 0, 0),
        TileKey {
            sheet: 1,
            ..key(doc, 0, 0, 0)
        },
    ] {
        assert!(
            matches!(service.get(k), Err(TileError::OutOfRange(_))),
            "{k:?}"
        );
    }
    assert!(service.close_document(&doc));
    assert!(!service.close_document(&doc));
    assert!(matches!(
        service.get(key(doc, 0, 0, 0)),
        Err(TileError::UnknownDocument(_))
    ));
}

#[test]
fn opening_the_same_file_twice_shares_the_document() {
    let Some((service, doc)) = service(TileConfig::default()) else {
        return;
    };
    let again = service
        .open_document(corpus_drawing(TEST_DRAWING_1))
        .unwrap();
    assert_eq!(*again.content_hash(), doc);
    assert!(std::sync::Arc::ptr_eq(
        &again,
        &service.document(&doc).unwrap()
    ));
}
