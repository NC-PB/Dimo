//! Performance harness for M0 (T0.9): NFR-PERF-01 (first sheet), NFR-PERF-03 (memory) and the
//! tile side of NFR-PERF-02 (pan). Run through `scripts/perf.sh`, which also builds the 50 sheet
//! stress document and records the machine.
//!
//! ```text
//! cargo run --release -p dimo-pdf --example perf -- <file.pdf> [--json out.json]
//! ```
//!
//! What it models. The viewport of the desktop app is replaced by a fixed window of
//! [`VIEWPORT`] device pixels (about a 1280 x 800 point window at 2x with side panels). The tile
//! levels and the prefetch margin follow `Viewport.svelte` and `tiles.ts` (coarsest level with
//! at least one tile pixel per device pixel, one tile of margin). The disk cache is off, so every
//! number is a cold render. The webview side (PNG decode, compositing, SVG balloons) is **not**
//! measured here: the frame time of NFR-PERF-02 comes from the dev hooks, see `docs/perf/M0.md`.
//!
//! Phases, all on one process and one `TileService` with default settings:
//!
//! 1. `open`: read the file, hash it, parse it in PDFium (PDFium itself is started before, as at
//!    app start).
//! 2. `first_sheet`: sheet 0 at the fit level, all visible tiles plus margin requested at once.
//!    NFR-PERF-01 is read file + open + this phase.
//! 3. `serial`: single tile latency at the 100 percent level, one request at a time.
//! 4. `pan`: scripted pan at the 100 percent level in real time at 60 steps per second, new tiles
//!    requested as they enter the margin. A frame is *late* when a visible tile is not ready.
//! 5. `scroll`: every sheet in turn, fit view plus one detail view, as a user paging through the
//!    project. Resident set size is sampled during the whole run (NFR-PERF-03).

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::struct_field_names,
    reason = "command line tool: prints its report, small geometry values"
)]

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use dimo_pdf::PdfEngine;
use dimo_pdf::tiles::{TILE_SIZE, TileConfig, TileGrid, TileKey, TileRange, TileService};

type BoxError = Box<dyn std::error::Error>;

/// Window content size in device pixels.
const VIEWPORT: (f64, f64) = (1600.0, 1000.0);
/// Tiles requested around the visible ones, as `PREFETCH_TILES` in `Viewport.svelte`.
const PREFETCH: u32 = 1;
/// The 100 percent level of a 2x display: 2 pixels per sheet unit.
const DETAIL_ZOOM: i32 = 1;
/// Pan speed in device pixels per frame (about 1000 CSS pixels per second at 2x and 60 Hz).
const PAN_STEP_PX: f64 = 32.0;
const FRAME: Duration = Duration::from_micros(16_667);

fn main() -> Result<(), BoxError> {
    let mut args = std::env::args().skip(1);
    let mut pdf = None;
    let mut json = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json = Some(PathBuf::from(args.next().ok_or("--json needs a path")?)),
            _ => pdf = Some(PathBuf::from(arg)),
        }
    }
    let pdf = pdf.ok_or("usage: perf <file.pdf> [--json out.json]")?;

    let sampler = RssSampler::start();
    let rss_start = rss_kib();

    let t = Instant::now();
    let engine = PdfEngine::start()?;
    let engine_start_ms = ms(t.elapsed());
    let service = Arc::new(TileService::new(engine, TileConfig::default()));
    let rss_engine = rss_kib();

    // 1. open
    let t = Instant::now();
    let bytes = std::fs::read(&pdf)?;
    let read_ms = ms(t.elapsed());
    let file_mib = bytes.len() as f64 / (1024.0 * 1024.0);
    let t = Instant::now();
    let doc = service.open_document(bytes)?;
    let open_ms = ms(t.elapsed());
    let hash = *doc.content_hash();
    let sheets = doc.sheet_count();
    let rss_open = rss_kib();
    println!(
        "open: {sheets} sheets, {file_mib:.1} MiB, read {read_ms:.1} ms, open {open_ms:.1} ms"
    );

    // 2. first sheet
    let size0 = doc.sheet_size(0)?;
    let fit_zoom = fit_zoom(size0.width, size0.height);
    let grid = TileGrid::new(size0, fit_zoom)?;
    let center = (
        f64::from(grid.width_px()) / 2.0,
        f64::from(grid.height_px()) / 2.0,
    );
    let range = tile_range(&grid, 0, center, PREFETCH);
    let first = fetch(&service, hash, &range)?;
    let first_sheet_ms = read_ms + open_ms + first.all_ms;
    let rss_first = rss_kib();
    println!(
        "first sheet: level {fit_zoom}, {} tiles, first tile {:.1} ms, all {:.1} ms, \
         read+open+all {first_sheet_ms:.1} ms",
        first.latencies_ms.len(),
        first.first_ms,
        first.all_ms
    );

    // 3. serial tile latency at the detail level, sheet 1 (not touched so far)
    let serial_sheet = u32::from(sheets > 1);
    let serial = serial_phase(&service, hash, serial_sheet)?;
    println!(
        "serial: {} tiles at level {DETAIL_ZOOM}, mean {:.1} ms, p95 {:.1} ms, max {:.1} ms",
        serial.n, serial.mean, serial.p95, serial.max
    );

    // 4. pan
    let pan = pan_phase(&service, hash, 0)?;
    println!(
        "pan: {} frames, {} late ({:.1} percent), {} tiles, tile latency p50 {:.1} ms, \
         p95 {:.1} ms, max {:.1} ms, longest stall {} frames",
        pan.frames,
        pan.late_frames,
        100.0 * f64::from(pan.late_frames) / f64::from(pan.frames.max(1)),
        pan.tiles,
        pan.latency.p50,
        pan.latency.p95,
        pan.latency.max,
        pan.longest_stall
    );

    // 5. scroll through all sheets
    let mut per_sheet = Vec::new();
    let t_scroll = Instant::now();
    for sheet in 0..sheets {
        let t = Instant::now();
        let size = doc.sheet_size(sheet)?;
        let grid = TileGrid::new(size, fit_zoom)?;
        let fit = tile_range(
            &grid,
            u32::try_from(sheet)?,
            (
                f64::from(grid.width_px()) / 2.0,
                f64::from(grid.height_px()) / 2.0,
            ),
            PREFETCH,
        );
        let a = fetch(&service, hash, &fit)?;
        // Detail view somewhere else on every sheet, deterministic.
        let detail_grid = TileGrid::new(size, DETAIL_ZOOM)?;
        let fx = 0.2 + 0.6 * ((sheet * 37 % 100) as f64 / 100.0);
        let fy = 0.2 + 0.6 * ((sheet * 61 % 100) as f64 / 100.0);
        let detail = tile_range(
            &detail_grid,
            u32::try_from(sheet)?,
            (
                f64::from(detail_grid.width_px()) * fx,
                f64::from(detail_grid.height_px()) * fy,
            ),
            PREFETCH,
        );
        let b = fetch(&service, hash, &detail)?;
        per_sheet.push((ms(t.elapsed()), a.all_ms, b.all_ms, rss_kib()));
    }
    let scroll_total_ms = ms(t_scroll.elapsed());
    let stats = service.stats();
    let rss_end = rss_kib();
    let peak = sampler.stop();
    let fit_ms: Vec<f64> = per_sheet.iter().map(|s| s.1).collect();
    let detail_ms: Vec<f64> = per_sheet.iter().map(|s| s.2).collect();
    let fit_stats = Stats::of(&fit_ms);
    let detail_stats = Stats::of(&detail_ms);
    println!(
        "scroll: {sheets} sheets in {scroll_total_ms:.0} ms, fit view p50 {:.0} ms max {:.0} ms, \
         detail view p50 {:.0} ms max {:.0} ms",
        fit_stats.p50, fit_stats.max, detail_stats.p50, detail_stats.max
    );
    println!(
        "memory (RSS MiB): start {:.0}, engine {:.0}, open {:.0}, first sheet {:.0}, end {:.0}, \
         peak sampled {:.0}; tile cache {:.1} MiB in {} tiles (peak {:.1} MiB), renders {}",
        mib(rss_start),
        mib(rss_engine),
        mib(rss_open),
        mib(rss_first),
        mib(rss_end),
        mib(peak),
        stats.memory_bytes as f64 / 1_048_576.0,
        stats.memory_tiles,
        stats.memory_peak_bytes as f64 / 1_048_576.0,
        stats.renders,
    );

    if let Some(path) = json {
        let mut j = String::new();
        let _ = write!(
            j,
            "{{\n  \"file\": \"{}\",\n  \"file_mib\": {file_mib:.2},\n  \"sheets\": {sheets},\n  \
             \"viewport_px\": [{}, {}],\n  \"fit_level\": {fit_zoom},\n  \"detail_level\": {DETAIL_ZOOM},\n  \
             \"engine_start_ms\": {engine_start_ms:.1},\n  \"read_ms\": {read_ms:.1},\n  \
             \"open_ms\": {open_ms:.1},\n  \"first_sheet\": {{\"tiles\": {}, \"first_tile_ms\": {:.1}, \
             \"all_tiles_ms\": {:.1}, \"read_open_view_ms\": {first_sheet_ms:.1}}},\n  \
             \"serial\": {},\n  \"pan\": {{\"frames\": {}, \"late_frames\": {}, \"tiles\": {}, \
             \"longest_stall_frames\": {}, \"tile_latency_ms\": {}}},\n  \
             \"scroll\": {{\"total_ms\": {scroll_total_ms:.0}, \"fit_view_ms\": {}, \"detail_view_ms\": {}}},\n  \
             \"memory_mib\": {{\"start\": {:.0}, \"engine\": {:.0}, \"open\": {:.0}, \"first_sheet\": {:.0}, \
             \"end\": {:.0}, \"peak_sampled\": {:.0}, \"tile_cache\": {:.1}, \"tile_cache_peak\": {:.1}}},\n  \
             \"tile_renders\": {}\n}}\n",
            pdf.file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            VIEWPORT.0,
            VIEWPORT.1,
            first.latencies_ms.len(),
            first.first_ms,
            first.all_ms,
            serial.json(),
            pan.frames,
            pan.late_frames,
            pan.tiles,
            pan.longest_stall,
            pan.latency.json(),
            fit_stats.json(),
            detail_stats.json(),
            mib(rss_start),
            mib(rss_engine),
            mib(rss_open),
            mib(rss_first),
            mib(rss_end),
            mib(peak),
            stats.memory_bytes as f64 / 1_048_576.0,
            stats.memory_peak_bytes as f64 / 1_048_576.0,
            stats.renders,
        );
        std::fs::write(&path, j)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// View model

/// Tile level the viewport picks to fit a whole sheet into [`VIEWPORT`] (`tileZoomFor`).
fn fit_zoom(width: f64, height: f64) -> i32 {
    let scale = (VIEWPORT.0 / width).min(VIEWPORT.1 / height);
    // Same tolerance as `LEVEL_TOLERANCE` in tiles.ts.
    (scale.log2() - 0.03).ceil() as i32
}

/// Tiles of `grid` that a window centered on pixel `center` shows, plus `margin` tiles.
fn tile_range(grid: &TileGrid, sheet: u32, center: (f64, f64), margin: u32) -> TileRange {
    let tile = f64::from(TILE_SIZE);
    let span = |c: f64, view: f64, limit: u32, count: u32| {
        let lo = (c - view / 2.0).max(0.0);
        let hi = (c + view / 2.0).min(f64::from(limit));
        let first = (lo / tile).floor() as u32;
        let end = ((hi / tile).ceil() as u32).max(first + 1);
        first.saturating_sub(margin)..(end + margin).min(count)
    };
    TileRange {
        sheet,
        zoom: grid.zoom(),
        columns: span(center.0, VIEWPORT.0, grid.width_px(), grid.columns()),
        rows: span(center.1, VIEWPORT.1, grid.height_px(), grid.rows()),
    }
}

fn keys(hash: dimo_pdf::ContentHash, range: &TileRange) -> Vec<TileKey> {
    let mut out = Vec::new();
    for y in range.rows.clone() {
        for x in range.columns.clone() {
            out.push(TileKey {
                doc: hash,
                sheet: range.sheet,
                zoom: range.zoom,
                x,
                y,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Fetching

struct Fetched {
    first_ms: f64,
    all_ms: f64,
    latencies_ms: Vec<f64>,
}

/// Declares `range` as the interest and requests all its tiles at once, like the viewport.
fn fetch(
    service: &TileService,
    hash: dimo_pdf::ContentHash,
    range: &TileRange,
) -> Result<Fetched, BoxError> {
    service.set_interest(hash, Some(vec![range.clone()]));
    let want = keys(hash, range);
    let (tx, rx) = mpsc::channel();
    let t = Instant::now();
    for key in &want {
        let tx = tx.clone();
        service.request(*key, move |r| {
            let _ = tx.send((Instant::now(), r));
        });
    }
    drop(tx);
    let mut latencies_ms = Vec::new();
    for (at, result) in rx {
        result?;
        latencies_ms.push(ms(at.duration_since(t)));
    }
    latencies_ms.sort_by(f64::total_cmp);
    Ok(Fetched {
        first_ms: latencies_ms.first().copied().unwrap_or(0.0),
        all_ms: latencies_ms.last().copied().unwrap_or(0.0),
        latencies_ms,
    })
}

// ---------------------------------------------------------------------------------------------
// Serial latency

fn serial_phase(
    service: &TileService,
    hash: dimo_pdf::ContentHash,
    sheet: u32,
) -> Result<Stats, BoxError> {
    let doc = service.document(&hash).ok_or("document not open")?;
    let size = doc.sheet_size(sheet as usize)?;
    let grid = TileGrid::new(size, DETAIL_ZOOM)?;
    service.set_interest(hash, None);
    let mut times = Vec::new();
    for i in 0..24u32 {
        let key = TileKey {
            doc: hash,
            sheet,
            zoom: DETAIL_ZOOM,
            x: (i * 5 + 2) % grid.columns(),
            y: (i * 3 + 1) % grid.rows(),
        };
        let t = Instant::now();
        service.get(key)?;
        times.push(ms(t.elapsed()));
    }
    Ok(Stats::of(&times))
}

// ---------------------------------------------------------------------------------------------
// Pan

struct Pan {
    frames: u32,
    late_frames: u32,
    longest_stall: u32,
    tiles: usize,
    latency: Stats,
}

/// A pan along a path of waypoints (pixels at the detail level), one step per 60 Hz frame.
fn pan_phase(
    service: &Arc<TileService>,
    hash: dimo_pdf::ContentHash,
    sheet: u32,
) -> Result<Pan, BoxError> {
    let doc = service.document(&hash).ok_or("document not open")?;
    let grid = TileGrid::new(doc.sheet_size(sheet as usize)?, DETAIL_ZOOM)?;
    let (w, h) = (f64::from(grid.width_px()), f64::from(grid.height_px()));
    let (mx, my) = (VIEWPORT.0 / 2.0, VIEWPORT.1 / 2.0);
    let path = [(mx, my), (w - mx, my), (w - mx, h - my), (mx, h - my)];
    let ready: Arc<Mutex<HashSet<TileKey>>> = Arc::default();
    let latencies: Arc<Mutex<Vec<f64>>> = Arc::default();
    let mut requested: HashSet<TileKey> = HashSet::new();
    let (mut frames, mut late, mut stall, mut longest) = (0u32, 0u32, 0u32, 0u32);

    let mut pos = path[0];
    let mut next = Instant::now();
    for target in &path[1..] {
        loop {
            let (dx, dy) = (target.0 - pos.0, target.1 - pos.1);
            let dist = dx.hypot(dy);
            if dist < 0.5 {
                break;
            }
            let step = PAN_STEP_PX.min(dist);
            pos = (pos.0 + dx / dist * step, pos.1 + dy / dist * step);
            frames += 1;

            // The viewport: declare interest, request what is not requested yet.
            let wanted = tile_range(&grid, sheet, pos, PREFETCH);
            service.set_interest(hash, Some(vec![wanted.clone()]));
            for key in keys(hash, &wanted) {
                if requested.insert(key) {
                    let (ready, latencies) = (Arc::clone(&ready), Arc::clone(&latencies));
                    let t = Instant::now();
                    service.request(key, move |result| {
                        if result.is_ok() {
                            latencies
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push(ms(t.elapsed()));
                            ready
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .insert(key);
                        }
                    });
                }
            }
            // The frame is late when a visible tile (without margin) is missing.
            let visible = tile_range(&grid, sheet, pos, 0);
            let missing = {
                let ready = ready.lock().unwrap_or_else(PoisonError::into_inner);
                keys(hash, &visible).iter().any(|k| !ready.contains(k))
            };
            if missing {
                late += 1;
                stall += 1;
                longest = longest.max(stall);
            } else {
                stall = 0;
            }

            next += FRAME;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                next = now;
            }
        }
    }
    // Let cancelled or late tiles finish so the next phase starts quiet.
    service.set_interest(hash, None);
    let tiles = requested.len();
    let latency = Stats::of(&latencies.lock().unwrap_or_else(PoisonError::into_inner));
    Ok(Pan {
        frames,
        late_frames: late,
        longest_stall: longest,
        tiles,
        latency,
    })
}

// ---------------------------------------------------------------------------------------------
// Statistics and memory

struct Stats {
    n: usize,
    mean: f64,
    p50: f64,
    p95: f64,
    max: f64,
}

impl Stats {
    fn of(values: &[f64]) -> Self {
        let mut v = values.to_vec();
        v.sort_by(f64::total_cmp);
        let at = |q: f64| -> f64 {
            if v.is_empty() {
                0.0
            } else {
                v[(((v.len() - 1) as f64) * q).round() as usize]
            }
        };
        Self {
            n: v.len(),
            mean: if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            },
            p50: at(0.5),
            p95: at(0.95),
            max: v.last().copied().unwrap_or(0.0),
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"n\": {}, \"mean\": {:.1}, \"p50\": {:.1}, \"p95\": {:.1}, \"max\": {:.1}}}",
            self.n, self.mean, self.p50, self.p95, self.max
        )
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn mib(kib: u64) -> f64 {
    kib as f64 / 1024.0
}

/// Resident set size of this process in KiB, or 0 when it cannot be read. Unix: `ps`; Windows:
/// `tasklist` (not tried yet, see docs/perf/M0.md).
fn rss_kib() -> u64 {
    let pid = std::process::id().to_string();
    let output = if cfg!(windows) {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
    } else {
        Command::new("ps").args(["-o", "rss=", "-p", &pid]).output()
    };
    let Ok(output) = output else { return 0 };
    let text = String::from_utf8_lossy(&output.stdout);
    let field = if cfg!(windows) {
        // "name","pid","session","number","123,456 K": the last field is the memory.
        text.rsplit(',').next().unwrap_or("").to_owned()
    } else {
        text.into_owned()
    };
    let digits: String = field.chars().filter(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

/// Polls the resident set size in the background and keeps the maximum.
struct RssSampler {
    stop: Arc<AtomicBool>,
    peak: Arc<AtomicU64>,
    thread: std::thread::JoinHandle<()>,
}

impl RssSampler {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let peak = Arc::new(AtomicU64::new(0));
        let (s, p) = (Arc::clone(&stop), Arc::clone(&peak));
        let thread = std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                p.fetch_max(rss_kib(), Ordering::Relaxed);
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        Self { stop, peak, thread }
    }

    /// Stops sampling and returns the highest value seen in KiB.
    fn stop(self) -> u64 {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.thread.join();
        self.peak.load(Ordering::Relaxed)
    }
}
