//! Project settings and part data (M2 decision 2, FR-TOL-06, FR-TOL-09).

use super::{CommandError, Tx};
use crate::balloon::BalloonStyle;
use crate::patch::Change;
use crate::project::{ProjectInfo, ProjectSettings, ToleranceSettings};

/// Sets the tolerance rules. Limits of existing characteristics do not change.
pub(super) fn set_tolerance(
    tx: &mut Tx<'_>,
    settings: &ToleranceSettings,
) -> Result<(), CommandError> {
    settings
        .validate()
        .map_err(CommandError::InvalidProjectSetting)?;
    change_settings(tx, |s| s.tolerance.clone_from(settings))
}

/// Sets the project default balloon style.
pub(super) fn set_default_balloon_style(
    tx: &mut Tx<'_>,
    style: &BalloonStyle,
) -> Result<(), CommandError> {
    if !style.is_valid() {
        return Err(CommandError::InvalidStyle);
    }
    change_settings(tx, |s| s.balloon_style = style.clone())
}

/// Replaces part and order data.
pub(super) fn update_project_info(tx: &mut Tx<'_>, info: &ProjectInfo) -> Result<(), CommandError> {
    if tx.project.info != *info {
        tx.push(Change::InfoChanged {
            before: tx.project.info.clone(),
            after: info.clone(),
        })?;
    }
    Ok(())
}

/// Edits the project settings; records a change only if they differ.
pub(super) fn change_settings(
    tx: &mut Tx<'_>,
    edit: impl FnOnce(&mut ProjectSettings),
) -> Result<(), CommandError> {
    let before = Box::new(tx.project.settings.clone());
    let mut after = before.clone();
    edit(&mut after);
    if after != before {
        tx.push(Change::SettingsChanged { before, after })?;
    }
    Ok(())
}
