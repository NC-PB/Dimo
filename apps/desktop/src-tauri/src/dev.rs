//! Development helpers for `tauri dev` (T0.8). Not a feature: release builds ignore all of this.
//!
//! The native file dialog cannot be driven by scripts, so a debug build can open a drawing at
//! startup from the environment and run the viewport performance check without any clicks:
//!
//! - `DIMO_DEV_OPEN=<path>`: PDF to open at startup.
//! - `DIMO_DEV_BALLOONS=<n>`: number of dummy balloons to place on every sheet.
//! - `DIMO_DEV_ANCHORS=<x,y;x,y;...>`: dummy balloons at these sheet points instead of random
//!   positions, for checking that balloons stay aligned with the drawing.
//! - `DIMO_DEV_SHEET=<n>`: zero based sheet to show first.
//! - `DIMO_DEV_VIEW=<percent>@<x>,<y>`: start zoomed to `percent` of the printed size with the
//!   sheet point `x`, `y` in the viewport center, for screenshots at a known zoom.
//! - `DIMO_DEV_PAN_CHECK=1`: run the scripted pan after loading and report the frame times.
//!
//! The path comes from the environment of the Rust process, never from the webview
//! (NFR-SEC-01). In release builds [`dev_startup`] returns nothing and
//! [`dev_report_frame_times`] does nothing.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::ipc::{CommandError, DocumentInfo};
use crate::tiles::TileState;

/// A point in sheet space (PDF user units, origin top left).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct SheetPoint {
    /// Distance from the left sheet edge.
    pub x: f64,
    /// Distance from the top sheet edge.
    pub y: f64,
}

/// What a debug build should do at startup, read from the `DIMO_DEV_*` environment variables.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct DevStartup {
    /// Document opened from `DIMO_DEV_OPEN`, if set.
    pub document: Option<DocumentInfo>,
    /// Number of dummy balloons per sheet, zero for none.
    pub balloons: u32,
    /// Fixed balloon anchors in sheet space; empty for random positions.
    pub anchors: Vec<SheetPoint>,
    /// Zero based sheet to show first, from `DIMO_DEV_SHEET`.
    pub sheet: u32,
    /// Start view from `DIMO_DEV_VIEW`; `None` fits the sheet.
    pub view: Option<DevView>,
    /// Whether to run the scripted pan and report frame times.
    pub pan_check: bool,
}

/// A start view for screenshots: zoom in percent of the printed size and the sheet point shown
/// in the viewport center.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct DevView {
    /// Zoom in percent of the printed size.
    pub percent: f64,
    /// Sheet point in the viewport center.
    pub center: SheetPoint,
}

/// Parses `DIMO_DEV_VIEW`: `<percent>@<x>,<y>`.
pub fn parse_view(text: &str) -> Option<DevView> {
    let (percent, center) = text.split_once('@')?;
    let percent = percent.trim().parse::<f64>().ok()?;
    let center = parse_anchors(center).into_iter().next()?;
    (percent.is_finite() && percent > 0.0).then_some(DevView { percent, center })
}

/// Frame times measured by the viewport during a scripted pan (NFR-PERF-02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct FrameTimeReport {
    /// Dummy balloons on the sheet during the measurement.
    pub balloons: u32,
    /// Number of measured frames.
    pub frames: u32,
    /// Mean frame time in milliseconds.
    pub mean_ms: f64,
    /// 95th percentile frame time in milliseconds.
    pub p95_ms: f64,
    /// Longest frame in milliseconds.
    pub max_ms: f64,
    /// Device pixel ratio of the window.
    pub device_pixel_ratio: f64,
    /// Viewport size in CSS pixels, width and height.
    pub viewport: [f64; 2],
}

/// Parses `DIMO_DEV_ANCHORS`: points `x,y` separated by `;`. Malformed points are skipped.
pub fn parse_anchors(text: &str) -> Vec<SheetPoint> {
    text.split(';')
        .filter_map(|point| {
            let (x, y) = point.trim().split_once(',')?;
            let x = x.trim().parse::<f64>().ok()?;
            let y = y.trim().parse::<f64>().ok()?;
            (x.is_finite() && y.is_finite()).then_some(SheetPoint { x, y })
        })
        .collect()
}

/// Debug builds: the startup actions from the `DIMO_DEV_*` environment variables. Release
/// builds: always the empty default.
#[tauri::command]
#[specta::specta]
pub async fn dev_startup(tiles: State<'_, TileState>) -> Result<DevStartup, CommandError> {
    if !cfg!(debug_assertions) {
        return Ok(DevStartup::default());
    }
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    let document = match var("DIMO_DEV_OPEN") {
        Some(path) => {
            let service = crate::ipc::tile_service(&tiles)?;
            Some(crate::ipc::open_path(service, path.into()).await?)
        }
        None => None,
    };
    Ok(DevStartup {
        document,
        balloons: var("DIMO_DEV_BALLOONS")
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0),
        anchors: var("DIMO_DEV_ANCHORS")
            .map(|a| parse_anchors(&a))
            .unwrap_or_default(),
        sheet: var("DIMO_DEV_SHEET")
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0),
        view: var("DIMO_DEV_VIEW").and_then(|v| parse_view(&v)),
        pan_check: var("DIMO_DEV_PAN_CHECK").is_some_and(|v| v != "0"),
    })
}

/// Debug builds: writes the frame times of a scripted pan to the terminal of `tauri dev`.
/// Release builds: does nothing.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
#[allow(
    clippy::print_stderr,
    reason = "dev only output for the tauri dev terminal, no tracing subscriber yet"
)]
pub fn dev_report_frame_times(report: FrameTimeReport) {
    if cfg!(debug_assertions) {
        eprintln!(
            "[dimo dev] pan frame times: {} balloons, {} frames, mean {:.2} ms, p95 {:.2} ms, \
             max {:.2} ms, dpr {}, viewport {}x{} css px",
            report.balloons,
            report.frames,
            report.mean_ms,
            report.p95_ms,
            report.max_ms,
            report.device_pixel_ratio,
            report.viewport[0],
            report.viewport[1],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_parse_and_skip_malformed_points() {
        assert_eq!(
            parse_anchors("237.51,166.1; 413.62 , 158.96;;x,1;3"),
            vec![
                SheetPoint {
                    x: 237.51,
                    y: 166.1
                },
                SheetPoint {
                    x: 413.62,
                    y: 158.96
                },
            ]
        );
        assert_eq!(parse_anchors(""), Vec::new());
        assert_eq!(parse_anchors("NaN,1"), Vec::new());
    }

    #[test]
    fn view_parses_percent_and_center() {
        assert_eq!(
            parse_view("800@237.5,166"),
            Some(DevView {
                percent: 800.0,
                center: SheetPoint { x: 237.5, y: 166.0 }
            })
        );
        assert_eq!(parse_view("800"), None);
        assert_eq!(parse_view("-5@1,2"), None);
    }
}
