//! Development helpers for `tauri dev` (T0.8). Not a feature: release builds ignore all of this.
//!
//! The native file dialog cannot be driven by scripts, so a debug build can open a drawing at
//! startup from the environment and run the viewport performance check without any clicks:
//!
//! - `DIMO_DEV_OPEN=<path>`: at startup, create a new project from this PDF, or open this
//!   project (`.dimo` file or project folder). Done once per process, not again on reload.
//! - `DIMO_DEV_SCRIPT=<path>`: after `DIMO_DEV_OPEN`, run the steps of this JSON file through
//!   the same session functions as the IPC commands (see [`DevStep`]).
//! - `DIMO_DEV_BALLOONS=<n>`: number of dummy balloons to place on every sheet.
//! - `DIMO_DEV_ANCHORS=<x,y;x,y;...>`: dummy balloons at these sheet points instead of random
//!   positions, for checking that balloons stay aligned with the drawing.
//! - `DIMO_DEV_SHEET=<n>`: zero based sheet to show first.
//! - `DIMO_DEV_VIEW=<percent>@<x>,<y>`: start zoomed to `percent` of the printed size with the
//!   sheet point `x`, `y` in the viewport center, for screenshots at a known zoom.
//! - `DIMO_DEV_PAN_CHECK=1`: run the scripted pan after loading and report the frame times.
//! - `DIMO_DEV_UI_SCRIPT=<path>`: after loading, the webview plays the pointer and key steps of
//!   this JSON file on the drawing (see `src/lib/dev/ui-script.ts` of the frontend) and writes
//!   its marks to the terminal through [`dev_log`], so window captures can be timed. Played
//!   once per process: a script that switches the UI language (which reloads the window) does
//!   not start again.
//! - `DIMO_DEV_EXPORT_DIR=<dir>`: exports skip the save dialog and write their suggested file
//!   name into this directory (T1.9), so a UI script can click the export buttons.
//!
//! - `DIMO_DEV_SAVE_AS=<path>`: "save as" (and "save" of a project without a file) skips the
//!   save dialog and saves to this path (T1.10).
//! - `DIMO_DEV_HOME=<dir>`: the cache, data and config folders of the app (tiles, autosave,
//!   `settings.json`) are `cache`, `data` and `config` below this folder instead of the
//!   user's, so a scripted run neither reads nor changes the settings of the person running it
//!   (T1.10).
//!
//! While `DIMO_DEV_OPEN` is set, a debug build does not restore unsaved projects at startup, so
//! a crashed autosave of real work is left for the next normal start.
//!
//! The path comes from the environment of the Rust process, never from the webview
//! (NFR-SEC-01). In release builds [`dev_startup`] returns nothing and
//! [`dev_report_frame_times`] does nothing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use dimo_core::Command;
use dimo_pdf::tiles::TileService;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::ipc::{CommandError, tile_service};
use crate::project::{SessionState, emit};
use crate::session::{AppSession, ProjectLoaded};
use crate::tiles::TileState;

/// True if a debug build was started with `DIMO_DEV_OPEN`.
pub fn dev_open_requested() -> bool {
    cfg!(debug_assertions) && var("DIMO_DEV_OPEN").is_some()
}

/// Debug builds started with `DIMO_DEV_EXPORT_DIR`: exports skip the save dialog and write
/// their suggested file name into this directory. `None` in release builds.
pub fn dev_export_dir() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        var("DIMO_DEV_EXPORT_DIR").map(PathBuf::from)
    } else {
        None
    }
}

/// Debug builds started with `DIMO_DEV_SAVE_AS`: the save dialog is skipped and the project is
/// saved to this path. `None` in release builds.
pub fn dev_save_as() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        var("DIMO_DEV_SAVE_AS").map(PathBuf::from)
    } else {
        None
    }
}

/// The folder for `kind` (`cache`, `data` or `config`) below `DIMO_DEV_HOME` in debug builds,
/// else `default`.
pub fn dev_folder(kind: &str, default: Option<PathBuf>) -> Option<PathBuf> {
    if cfg!(debug_assertions)
        && let Some(home) = var("DIMO_DEV_HOME")
    {
        return Some(PathBuf::from(home).join(kind));
    }
    default
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// One step of a `DIMO_DEV_SCRIPT` file, a JSON array such as
///
/// ```json
/// [
///   { "execute": { "type": "add_characteristic", "sheet": "$SHEET0", "position": {"x": 1, "y": 2},
///                  "anchor": {"x": 3, "y": 4}, "region": null, "values": [] } },
///   "undo", "redo",
///   { "save_as": "/tmp/part.dimo" }, "close", { "open": "/tmp/part.dimo" }
/// ]
/// ```
///
/// `$SHEET<n>` is replaced by the ID of sheet `n` of the current drawing revision.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevStep {
    /// Executes a document command.
    Execute(Command),
    /// Undoes the last command.
    Undo,
    /// Redoes the last undone command.
    Redo,
    /// Saves to the project file.
    Save,
    /// Saves to this path.
    SaveAs(PathBuf),
    /// Closes the project, discarding unsaved changes.
    Close,
    /// Opens this project, discarding unsaved changes of the open one.
    Open(PathBuf),
}

/// Replaces `$SHEET<n>` in a script with the sheet IDs of the current revision. Higher indexes
/// first, so `$SHEET1` is not taken for `$SHEET10`.
pub fn substitute_sheets(script: &str, project: &dimo_core::Project) -> String {
    let sheets = project
        .current_revision()
        .map(|r| r.sheets.as_slice())
        .unwrap_or_default();
    let mut text = script.to_owned();
    for (index, sheet) in sheets.iter().enumerate().rev() {
        text = text.replace(&format!("$SHEET{index}"), &sheet.id.to_string());
    }
    text
}

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
    /// The project after `DIMO_DEV_OPEN` and `DIMO_DEV_SCRIPT`, `None` if `DIMO_DEV_OPEN` is
    /// unset or was handled by an earlier call.
    pub project: Option<ProjectLoaded>,
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
    /// Text of the `DIMO_DEV_UI_SCRIPT` file, played by the webview after loading.
    pub ui_script: Option<String>,
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

/// `DIMO_DEV_OPEN` runs once per process, so reloading the webview keeps the project.
static DEV_OPEN_DONE: AtomicBool = AtomicBool::new(false);

/// `DIMO_DEV_UI_SCRIPT` is played once per process, also when the window reloads.
static DEV_UI_SCRIPT_DONE: AtomicBool = AtomicBool::new(false);

/// Debug builds: the startup actions from the `DIMO_DEV_*` environment variables. Release
/// builds: always the empty default.
#[tauri::command]
#[specta::specta]
pub async fn dev_startup(app: AppHandle) -> Result<DevStartup, CommandError> {
    if !cfg!(debug_assertions) {
        return Ok(DevStartup::default());
    }
    let project = match var("DIMO_DEV_OPEN") {
        Some(path) if !DEV_OPEN_DONE.swap(true, Ordering::SeqCst) => {
            let tiles = tile_service(&app.state::<TileState>())?;
            let script = var("DIMO_DEV_SCRIPT");
            let handle = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = handle.state::<SessionState>();
                let mut session = state.lock();
                dev_open(
                    &handle,
                    &mut session,
                    &tiles,
                    Path::new(&path),
                    script.as_deref(),
                )
            })
            .await
            .map_err(|e| CommandError::Io {
                message: e.to_string(),
            })??;
            Some(app.state::<SessionState>().lock().state())
        }
        _ => None,
    };
    Ok(DevStartup {
        project,
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
        ui_script: match var("DIMO_DEV_UI_SCRIPT") {
            Some(path) if !DEV_UI_SCRIPT_DONE.swap(true, Ordering::SeqCst) => Some(
                std::fs::read_to_string(&path).map_err(|e| CommandError::Io {
                    message: format!("{path}: {e}"),
                })?,
            ),
            _ => None,
        },
    })
}

/// True if `path` is a project (file with the project extension or a folder), not a drawing.
fn is_project(path: &Path) -> bool {
    path.is_dir()
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(dimo_io::project::EXTENSION))
}

/// Opens the `DIMO_DEV_OPEN` path and runs the script, emitting the same events as the commands.
#[allow(
    clippy::print_stderr,
    reason = "dev only output for the tauri dev terminal, no tracing subscriber yet"
)]
fn dev_open(
    app: &AppHandle,
    session: &mut AppSession,
    tiles: &TileService,
    path: &Path,
    script: Option<&str>,
) -> Result<(), CommandError> {
    let loaded = if is_project(path) {
        session.open_file(tiles, path, true)?
    } else {
        session.create_from_drawing(tiles, path, true)?
    };
    emit(app, &loaded);
    eprintln!(
        "[dimo dev] opened {}: session {}, {} characteristics",
        path.display(),
        loaded.session,
        session.project().map_or(0, |p| p.characteristics.len())
    );
    let Some(script) = script else {
        return Ok(());
    };
    let text = std::fs::read_to_string(script).map_err(|e| CommandError::Io {
        message: format!("{script}: {e}"),
    })?;
    let text = session
        .project()
        .map_or(text.clone(), |p| substitute_sheets(&text, p));
    let steps: Vec<DevStep> =
        serde_json::from_str(&text).map_err(|e| CommandError::InvalidArgument {
            message: format!("{script}: {e}"),
        })?;
    for (index, step) in steps.into_iter().enumerate() {
        let what = format!("{step:?}");
        let result = run_step(app, session, tiles, step);
        let number = index + 1;
        match &result {
            Ok(summary) => eprintln!("[dimo dev] step {number}: {summary}"),
            Err(error) => eprintln!("[dimo dev] step {number} failed: {error} ({what})"),
        }
        result?;
    }
    Ok(())
}

fn run_step(
    app: &AppHandle,
    session: &mut AppSession,
    tiles: &TileService,
    step: DevStep,
) -> Result<String, CommandError> {
    let patched = |p: &crate::session::ProjectPatched| {
        format!(
            "{} changes, revision {}, {} characteristics",
            p.patch.changes.len(),
            p.revision,
            "{count}"
        )
    };
    let count = |s: &AppSession| s.project().map_or(0, |p| p.characteristics.len());
    let summary = match step {
        DevStep::Execute(command) => {
            let p = session.execute(command)?;
            if !p.patch.is_empty() {
                emit(app, &p);
            }
            patched(&p)
        }
        DevStep::Undo => {
            let p = session.undo()?;
            emit(app, &p);
            patched(&p)
        }
        DevStep::Redo => {
            let p = session.redo()?;
            emit(app, &p);
            patched(&p)
        }
        DevStep::Save => {
            let s = session.save()?;
            emit(app, &s);
            format!("saved {:?}", s.status.file_name)
        }
        DevStep::SaveAs(path) => {
            let s = session.save_as(&path)?;
            emit(app, &s);
            format!("saved as {}", path.display())
        }
        DevStep::Close => {
            let l = session.close(Some(tiles), true)?;
            emit(app, &l);
            "closed".to_owned()
        }
        DevStep::Open(path) => {
            let l = session.open_file(tiles, &path, true)?;
            emit(app, &l);
            format!("opened {}, notice {:?}", path.display(), l.notice)
        }
    };
    Ok(summary.replace("{count}", &count(session).to_string()))
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

/// Debug builds: writes one line from the webview's UI script to the terminal of `tauri dev`.
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
pub fn dev_log(message: String) {
    if cfg!(debug_assertions) {
        eprintln!("[dimo dev] ui: {message}");
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
    fn script_steps_parse() {
        let steps: Vec<DevStep> = serde_json::from_str(
            r#"["undo", "redo", "save", "close", {"save_as": "/tmp/a.dimo"},
                {"open": "/tmp/a.dimo"}, {"execute": {"type": "unlock_numbering"}}]"#,
        )
        .unwrap();
        assert_eq!(steps.len(), 7);
        assert_eq!(steps[4], DevStep::SaveAs("/tmp/a.dimo".into()));
        assert_eq!(steps[6], DevStep::Execute(Command::UnlockNumbering));
    }

    #[test]
    fn sheet_placeholders_become_sheet_ids() {
        use dimo_core::{
            DrawingRevision, Project, ProjectInfo, RevisionId, Sha256Hex, Sheet, SheetId,
            SheetKind, Size, Timestamp,
        };
        let size = Size {
            width: 1.0,
            height: 1.0,
        };
        let sheets: Vec<Sheet> = (0..11u32)
            .map(|i| {
                let id = SheetId::from_uuid(uuid::Uuid::from_u128(u128::from(i) + 1));
                Sheet::new(id, i, size, SheetKind::VectorText)
            })
            .collect();
        let project = Project::new(
            ProjectInfo::default(),
            DrawingRevision {
                id: RevisionId::from_uuid(uuid::Uuid::from_u128(100)),
                label: String::new(),
                file_name: "a.pdf".into(),
                sha256: Sha256Hex::parse(&"0".repeat(64)).unwrap(),
                imported_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
                sheets: sheets.clone(),
            },
        );
        let text = substitute_sheets("$SHEET0 $SHEET10 $SHEET1", &project);
        assert_eq!(
            text,
            format!("{} {} {}", sheets[0].id, sheets[10].id, sheets[1].id)
        );
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
