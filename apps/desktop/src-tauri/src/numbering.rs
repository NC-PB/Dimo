//! Numbering preview of the main window (T2.7, FR-BAL-05). The rules live in
//! [`dimo_core::numbering`]; applying a strategy is the document command `apply_numbering`,
//! sent through [`crate::project::execute`].

use dimo_core::{NumberingPreview, NumberingStrategy};
use tauri::AppHandle;

use crate::ipc::CommandError;
use crate::project::with_session;
use crate::session::AppSession;

/// The order and numbers `strategy` would give, without changing the project (FR-BAL-05).
/// Refused with `numbering_locked` while numbering is locked (D-23).
pub fn preview(
    session: &AppSession,
    strategy: NumberingStrategy,
) -> Result<NumberingPreview, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    Ok(dimo_core::numbering::preview(project, strategy)?)
}

/// The order and numbers `strategy` would give, without changing the project (FR-BAL-05).
#[tauri::command]
#[specta::specta]
pub async fn preview_numbering(
    app: AppHandle,
    strategy: NumberingStrategy,
) -> Result<NumberingPreview, CommandError> {
    with_session(app, move |_, session| preview(session, strategy)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_needs_a_project() {
        let session = AppSession::new(None);
        assert_eq!(
            preview(&session, NumberingStrategy::SheetZone),
            Err(CommandError::NoProject)
        );
    }
}
