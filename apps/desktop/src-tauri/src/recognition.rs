//! Box select and typed callouts (T2.6, FR-REC-01, FR-REC-02, M2 decision 5).
//!
//! Both commands only read: they return proposals or field values and change nothing (ADR 0006).
//! The frontend accepts proposals with the document command `accept_proposals` through
//! `execute`, which is one undo step, and applies typed values with `update_fields`.
//!
//! Box select answers directly instead of as a job: on `test_drawing_1` it takes about a
//! millisecond (a region query on the cached page plus parsing), far below the 50 ms where a
//! job with progress events would pay off. The proposals still carry a job ID from the job
//! counter, so a later job based box select fits the same data.

use dimo_core::characteristic::FieldValue;
use dimo_core::geometry::OrientedBox;
use dimo_core::proposal::{BalloonPlacement, ParseIssue, Proposal};
use dimo_core::{ProjectSettings, SheetId};
use dimo_detect::{BoxSelectContext, CalloutOnly, Interpreter, box_select, read_callout};
use dimo_pdf::tiles::TileService;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::export::next_job_id;
use crate::ipc::{CommandError, tile_service};
use crate::project::with_session;
use crate::session::RecognitionTarget;
use crate::tiles::TileState;

/// The interpreter for a project: turns parsed callouts into limits (spec 08 stage 7).
///
/// Until the tolerance engine of `dimo-tolerance` is wired in, this is the fallback
/// [`CalloutOnly`], which fills only what the callout writes. The engine plugs in here, built
/// from `settings` (general tolerance, decimal rules, custom tables).
pub fn interpreter(settings: &ProjectSettings) -> impl Interpreter + use<> {
    let _ = settings;
    CalloutOnly
}

/// Values for text typed into the value field (M2 decision 5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct TypedCallout {
    /// Fields to set in one `update_fields` command: the requirement text, and when the text
    /// parses, kind, nominal, unit, deviations, limits, fit, quantity, inspect and derivation.
    pub values: Vec<FieldValue>,
    /// Why the text could not be parsed; then `values` holds only the requirement text.
    pub parse_error: Option<ParseIssue>,
}

fn check_region(region: &OrientedBox) -> Result<(), CommandError> {
    let finite = [
        region.center.x,
        region.center.y,
        region.size.width,
        region.size.height,
        region.angle,
    ]
    .iter()
    .all(|v| v.is_finite());
    if finite && region.size.width > 0.0 && region.size.height > 0.0 {
        Ok(())
    } else {
        Err(CommandError::InvalidArgument {
            message: "the region needs a finite position and a size above zero".to_owned(),
        })
    }
}

/// Proposals for the text inside `region` of the open drawing (FR-REC-01, FR-REC-02).
/// `placement` is where the balloon of the box goes. Pure apart from PDFium; no project access.
pub fn propose(
    tiles: &TileService,
    target: &RecognitionTarget,
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
) -> Result<Vec<Proposal>, CommandError> {
    check_region(&region)?;
    let doc = tiles
        .document(&target.drawing)
        .ok_or_else(|| CommandError::InvalidDocument {
            message: "the drawing of the project is not open".to_owned(),
        })?;
    let started = std::time::Instant::now();
    let text = doc.region_text(target.page, region)?;
    let interpreter = interpreter(&target.settings);
    let proposals = box_select(
        sheet,
        &region,
        &text,
        &BoxSelectContext {
            interpreter: &interpreter,
            drawing_unit: target.unit,
            placement,
            job_id: Some(next_job_id().0),
        },
    );
    tracing::debug!(
        proposals = proposals.len(),
        micros = started.elapsed().as_micros(),
        "box select"
    );
    Ok(proposals)
}

/// Reads typed text like a box selection (M2 decision 5). `drawing_unit` is the sheet unit.
pub fn read_typed(target: &RecognitionTarget, text: &str) -> TypedCallout {
    let interpreter = interpreter(&target.settings);
    let reading = read_callout(text, false, target.unit, &interpreter);
    TypedCallout {
        values: reading.field_values(),
        parse_error: reading.parse_error,
    }
}

/// Box select: proposals for the text inside `region` of `sheet` (FR-REC-01, FR-REC-02).
/// Changes nothing; accept them with the `accept_proposals` document command (ADR 0006).
/// An empty list means the region holds no PDF text.
#[tauri::command]
#[specta::specta]
pub async fn propose_from_region(
    app: AppHandle,
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
) -> Result<Vec<Proposal>, CommandError> {
    let tiles = tile_service(&app.state::<TileState>())?;
    let target = with_session(app, move |_, session| session.recognition_target(sheet)).await?;
    tauri::async_runtime::spawn_blocking(move || propose(&tiles, &target, sheet, region, placement))
        .await
        .map_err(|e| CommandError::Io {
            message: e.to_string(),
        })?
}

/// Parses and interprets text typed for a characteristic on `sheet` (M2 decision 5). Changes
/// nothing; the frontend sets the returned values with one `update_fields` command.
#[tauri::command]
#[specta::specta]
pub async fn read_callout_text(
    app: AppHandle,
    sheet: SheetId,
    text: String,
) -> Result<TypedCallout, CommandError> {
    let target = with_session(app, move |_, session| session.recognition_target(sheet)).await?;
    Ok(read_typed(&target, &text))
}
