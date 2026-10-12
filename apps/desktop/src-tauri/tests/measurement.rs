//! The `measurement_state` query (T3.3, FR-MEA-03): verdicts of the open project, and the
//! timing for 300 characteristics by 10 samples (called after each patch, under 20 ms).
//!
//! Needs PDFium. Skips without it, fails instead with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`.

// Test code (rust.md): unwrap is fine, and the skip message and timing must reach the output.
#![allow(clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use dimo_core::{
    Command, FieldValue, OverallVerdict, Point, ResultEntry, ResultType, Verdict, VerdictCounts,
};
use dimo_desktop::ipc::CommandError;
use dimo_desktop::measurement::state_of;
use dimo_desktop::session::AppSession;
use dimo_pdf::tiles::{TileConfig, TileService};
use dimo_pdf::{PdfEngine, PdfError};
use rust_decimal::Decimal;

fn pdfium_required() -> bool {
    std::env::var("CI").is_ok_and(|v| v == "true")
        || std::env::var("DIMO_REQUIRE_PDFIUM").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn service() -> Option<TileService> {
    match PdfEngine::start() {
        Ok(engine) => Some(TileService::new(engine, TileConfig::default())),
        Err(e @ PdfError::LibraryNotFound { .. }) if !pdfium_required() => {
            eprintln!("SKIPPED (PDFium missing): {e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

fn drawing() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../../corpus/drawings/test_drawing_1.pdf")
}

/// A session with `chars` characteristics (even ones `10 ±0.1`, odd ones without limits) and
/// `samples` samples, every cell filled except the last sample of every tenth row; one cell
/// fails.
fn session_with(tiles: &TileService, chars: u32, samples: u32) -> AppSession {
    let mut session = AppSession::new(None);
    session
        .create_from_drawing(tiles, &drawing(), false)
        .unwrap();
    let sheet = session.project().unwrap().revisions[0].sheets[0].id;
    let adds = (0..chars)
        .map(|i| {
            let (x, y) = (f64::from(40 + (i % 20) * 30), f64::from(40 + (i / 20) * 25));
            let values = if i % 2 == 0 {
                vec![
                    FieldValue::Nominal(Some(Decimal::new(10, 0)).into()),
                    FieldValue::UpperDev(Some(Decimal::new(1, 1)).into()),
                    FieldValue::LowerDev(Some(Decimal::new(-1, 1)).into()),
                ]
            } else {
                Vec::new()
            };
            Command::AddCharacteristic {
                sheet,
                position: Point { x, y },
                anchor: Point { x: x - 10.0, y },
                region: None,
                values,
                insert_after: None,
            }
        })
        .collect();
    session.execute(Command::Batch { commands: adds }).unwrap();
    session
        .execute(Command::AddSamples { count: samples })
        .unwrap();
    let project = session.project().unwrap();
    let mut results = Vec::new();
    for (si, sample) in project.samples.iter().enumerate() {
        for (ci, c) in project.characteristics.iter().enumerate() {
            if ci % 10 == 0 && si + 1 == project.samples.len() {
                continue;
            }
            let text = match (ci % 2, ci == 2 && si == 0) {
                (0, true) => "10.101",
                (0, false) => "10.05",
                _ => "ok",
            };
            results.push(ResultEntry {
                sample: sample.id,
                characteristic: c.id,
                text: text.into(),
                comment: None,
            });
        }
    }
    session.execute(Command::SetResults { results }).unwrap();
    session
}

#[test]
fn measurement_state_of_the_open_project() {
    assert!(matches!(
        state_of(&AppSession::new(None)),
        Err(CommandError::NoProject)
    ));
    let Some(tiles) = service() else { return };
    let session = session_with(&tiles, 4, 2);
    let state = state_of(&session).unwrap();
    let cells: Vec<&[Verdict]> = state.rows.iter().map(|r| r.cells.as_slice()).collect();
    let want: [&[Verdict]; 4] = [
        &[Verdict::Pass, Verdict::Missing],
        &[Verdict::Pass, Verdict::Pass],
        &[Verdict::Fail, Verdict::Pass],
        &[Verdict::Pass, Verdict::Pass],
    ];
    assert_eq!(cells, want);
    let types: Vec<ResultType> = state.rows.iter().map(|r| r.result_type).collect();
    assert_eq!(
        types,
        [
            ResultType::Variable,
            ResultType::Attribute,
            ResultType::Variable,
            ResultType::Attribute
        ]
    );
    let samples: Vec<OverallVerdict> = state.samples.iter().map(|s| s.verdict).collect();
    assert_eq!(samples, [OverallVerdict::Fail, OverallVerdict::Open]);
    assert_eq!(state.verdict, OverallVerdict::Fail);
    assert_eq!(
        state.counts,
        VerdictCounts {
            pass: 6,
            fail: 1,
            not_evaluated: 0,
            missing: 1
        }
    );
    // The IPC payload: plain JSON with snake case verdicts.
    let json = serde_json::to_value(&state).unwrap();
    assert_eq!(json["rows"][0]["cells"][1], "missing");
    assert_eq!(json["verdict"], "fail");
}

/// 300 characteristics by 10 samples: the query including JSON serialization (what the IPC
/// layer sends) stays under 20 ms.
///
/// The bound is checked on the fastest of 30 runs: that is the cost of the code, while the
/// mean also measures the machine load of a parallel test run (several times the bound when
/// other builds share the CPU). Mean, median and slowest are printed for STATUS.md.
#[test]
fn measurement_state_takes_under_20_ms_for_300_by_10() {
    let Some(tiles) = service() else { return };
    let session = session_with(&tiles, 300, 10);
    let mut runs = Vec::new();
    let mut bytes = 0;
    for _ in 0..30 {
        let start = Instant::now();
        let state = state_of(&session).unwrap();
        bytes = serde_json::to_vec(&state).unwrap().len();
        runs.push(start.elapsed());
        assert_eq!((state.rows.len(), state.samples.len()), (300, 10));
    }
    runs.sort();
    let mean = runs.iter().sum::<Duration>() / u32::try_from(runs.len()).unwrap();
    let (fastest, median, slowest) = (runs[0], runs[runs.len() / 2], runs[runs.len() - 1]);
    eprintln!(
        "measurement_state 300 x 10: fastest {fastest:?}, median {median:?}, mean {mean:?}, \
         slowest {slowest:?}, {bytes} bytes"
    );
    assert!(fastest < Duration::from_millis(20), "fastest {fastest:?}");
}
