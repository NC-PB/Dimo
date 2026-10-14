//! Sheets: view rotation, unit, scale, zone grid and views (FR-DOC-05, M2 decision 1).

use super::{CommandError, Tx};
use crate::characteristic::Unit;
use crate::id::SheetId;
use crate::patch::Change;
use crate::sheet::{Rotation, Scale, Sheet, SheetView, ZoneGrid};

/// Sets or removes the zone grid.
pub(super) fn set_zone_grid(
    tx: &mut Tx<'_>,
    sheet: SheetId,
    grid: Option<&ZoneGrid>,
) -> Result<(), CommandError> {
    if let Some(grid) = grid {
        grid.validate().map_err(CommandError::InvalidSheetSetting)?;
    }
    change_sheet(tx, sheet, |s| s.zone_grid = grid.cloned())
}

/// Replaces the view rectangles.
pub(super) fn set_views(
    tx: &mut Tx<'_>,
    sheet: SheetId,
    views: &[SheetView],
) -> Result<(), CommandError> {
    if views.len() > MAX_VIEWS {
        return Err(CommandError::InvalidSheetSetting(
            "a sheet has at most 1000 views",
        ));
    }
    for view in views {
        view.validate().map_err(CommandError::InvalidSheetSetting)?;
    }
    change_sheet(tx, sheet, |s| s.views = views.to_vec())
}

/// Most view rectangles per sheet.
const MAX_VIEWS: usize = 1000;

/// Edits one sheet; records a change only if it differs.
fn change_sheet(
    tx: &mut Tx<'_>,
    id: SheetId,
    edit: impl FnOnce(&mut Sheet),
) -> Result<(), CommandError> {
    let current = tx.project.sheet(id).ok_or(CommandError::UnknownSheet(id))?;
    let before = Box::new(current.clone());
    let mut after = before.clone();
    edit(&mut after);
    if after != before {
        tx.push(Change::SheetChanged { before, after })?;
    }
    Ok(())
}

pub(super) fn update(
    tx: &mut Tx<'_>,
    id: SheetId,
    rotation: Option<Rotation>,
    unit: Option<Unit>,
    scale: Option<Scale>,
) -> Result<(), CommandError> {
    if unit.is_some_and(|u| !u.is_length()) {
        return Err(CommandError::InvalidSheetSetting("unit must be mm or in"));
    }
    if scale.is_some_and(|s| !s.is_valid()) {
        return Err(CommandError::InvalidSheetSetting(
            "scale parts must be at least 1",
        ));
    }
    change_sheet(tx, id, |s| {
        s.rotation = rotation.unwrap_or(s.rotation);
        s.unit = unit.unwrap_or(s.unit);
        s.scale = scale.unwrap_or(s.scale);
    })
}
