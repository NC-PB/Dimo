//! Verdicts and summaries of the measurement grid for the main window (T3.3, FR-MEA-03).
//!
//! The rules live in `dimo_core::evaluation`; this module only reads the open project. The
//! frontend calls [`measurement_state`] after each patch: verdicts are computed, never stored
//! (M3 decision 2), and never decided in TypeScript (rule 2).

use dimo_core::{CharId, DeleteCheck, MeasurementState};
use tauri::AppHandle;

use crate::ipc::CommandError;
use crate::project::with_session;
use crate::session::AppSession;

/// Verdicts per cell and summaries per characteristic, per sample and for the report of the
/// open project (FR-MEA-03, M3 decisions 3 and 5): measured characteristics in number order,
/// samples in column order.
#[tauri::command]
#[specta::specta]
pub async fn measurement_state(app: AppHandle) -> Result<MeasurementState, CommandError> {
    with_session(app, |_, session| state_of(session)).await
}

/// The measurement state of the project open in `session`; `no_project` without one.
pub fn state_of(session: &AppSession) -> Result<MeasurementState, CommandError> {
    session
        .project()
        .map(dimo_core::measurement_state)
        .ok_or(CommandError::NoProject)
}

/// Characteristics whose limits or result type changed after the last issued report, in
/// placement order; empty without an issued report (M3 decision 16). The measurement grid marks
/// them with a hint. Computed from the audit log in Rust (rule 2).
#[tauri::command]
#[specta::specta]
pub async fn changed_since_issued_report(app: AppHandle) -> Result<Vec<CharId>, CommandError> {
    with_session(app, |_, session| session.changed_since_issued_report()).await
}

/// The results that deleting the characteristics `ids` would remove (M3 decision 16): the
/// frontend asks before such a deletion and shows the count (rule 2, MF F8).
#[tauri::command]
#[specta::specta]
pub async fn delete_check(app: AppHandle, ids: Vec<CharId>) -> Result<DeleteCheck, CommandError> {
    with_session(app, move |_, session| {
        session
            .project()
            .map(|p| p.delete_check(&ids))
            .ok_or(CommandError::NoProject)
    })
    .await
}
