//! Project settings and part data (M2 decision 2, FR-TOL-06, FR-TOL-09).

use super::{CommandError, Tx};
use crate::balloon::BalloonStyle;
use crate::patch::Change;
use crate::project::{
    DecimalPlaceRule, ProjectInfoField, ProjectSettings, ToleranceField, ToleranceSettings,
};

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

/// Sets single tolerance settings and checks the result (MF F4).
pub(super) fn update_tolerance(
    tx: &mut Tx<'_>,
    values: &[ToleranceField],
) -> Result<(), CommandError> {
    edit_tolerance(tx, |t| values.iter().for_each(|v| t.set(v)))
}

/// Adds a decimal place rule; refused if one for the same places exists (MF F4).
pub(super) fn add_decimal_rule(
    tx: &mut Tx<'_>,
    rule: DecimalPlaceRule,
) -> Result<(), CommandError> {
    edit_tolerance(tx, |t| t.add_decimal_rule(rule))
}

/// Removes the decimal place rule for `places`, if there is one (MF F4).
pub(super) fn remove_decimal_rule(tx: &mut Tx<'_>, places: u32) -> Result<(), CommandError> {
    edit_tolerance(tx, |t| t.decimal_rules.retain(|r| r.places != places))
}

/// Edits the current tolerance settings and checks the result.
fn edit_tolerance(
    tx: &mut Tx<'_>,
    edit: impl FnOnce(&mut ToleranceSettings),
) -> Result<(), CommandError> {
    let mut settings = tx.project.settings.tolerance.clone();
    edit(&mut settings);
    set_tolerance(tx, &settings)
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

/// Sets fields of the part and order data (MF F4).
pub(super) fn update_project_info(
    tx: &mut Tx<'_>,
    values: &[ProjectInfoField],
) -> Result<(), CommandError> {
    let mut info = tx.project.info.clone();
    for v in values {
        info.set(v);
    }
    if tx.project.info != info {
        tx.push(Change::InfoChanged {
            before: tx.project.info.clone(),
            after: info,
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
