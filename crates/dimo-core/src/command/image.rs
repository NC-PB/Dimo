//! Image edit settings of photo and scan drawings and the sheet geometry that follows them
//! (R01 decisions 8 to 11, MF F1, R.20).

use super::{CommandError, Tx};
use crate::id::{RevisionId, SheetId};
use crate::patch::Change;
use crate::project::Project;
use crate::sheet::Sheet;
use crate::sheet_map::SheetMap;
use crate::source::{DrawingSource, ImageEdit};

pub(super) fn set_image_edit(
    tx: &mut Tx<'_>,
    revision: RevisionId,
    page: u32,
    edit: &ImageEdit,
    geometry: Option<&SheetMap>,
) -> Result<(), CommandError> {
    let current = tx
        .project
        .revisions
        .iter()
        .find(|r| r.id == revision)
        .ok_or(CommandError::UnknownRevision(revision))?;
    let DrawingSource::Image { format, pages } = &current.source else {
        return Err(CommandError::NotAnImage(revision));
    };
    let index = usize::try_from(page).unwrap_or(usize::MAX);
    let Some(old) = pages.get(index) else {
        return Err(CommandError::InvalidImageEdit("the image has no such page"));
    };
    edit.validate().map_err(CommandError::InvalidImageEdit)?;
    let sheet = match geometry {
        Some(map) => {
            if !map.is_valid() {
                return Err(CommandError::InvalidImageEdit(
                    "the sheet of the edit needs a positive size and an invertible map",
                ));
            }
            let sheet = current
                .sheets
                .iter()
                .find(|s| s.index == page)
                .ok_or(CommandError::InvalidImageEdit("the page has no sheet"))?;
            Some(sheet.clone())
        }
        None => None,
    };
    if old != edit {
        let mut new_pages = pages.clone();
        new_pages[index] = edit.clone();
        tx.push(Change::RevisionChanged {
            revision,
            before: current.source.clone(),
            after: DrawingSource::Image {
                format: *format,
                pages: new_pages,
            },
        })?;
    }
    if let (Some(sheet), Some(map)) = (sheet, geometry) {
        move_sheet_geometry(tx, &sheet, map)?;
    }
    Ok(())
}

/// Gives the sheet its new size and moves everything placed on it with the map.
fn move_sheet_geometry(tx: &mut Tx<'_>, sheet: &Sheet, map: &SheetMap) -> Result<(), CommandError> {
    let mut after = sheet.clone();
    after.size = map.size;
    after.raster_dpi = map.raster_dpi;
    after.views = sheet
        .views
        .iter()
        .filter_map(|v| {
            map.rect(v.rect).map(|rect| crate::sheet::SheetView {
                label: v.label.clone(),
                rect,
            })
        })
        .collect();
    after.zone_grid = sheet.zone_grid.as_ref().and_then(|grid| {
        map.rect(grid.frame).map(|frame| crate::sheet::ZoneGrid {
            frame,
            ..grid.clone()
        })
    });
    if after != *sheet {
        tx.push(Change::SheetChanged {
            before: Box::new(sheet.clone()),
            after: Box::new(after),
        })?;
    }
    let balloons: Vec<_> = tx
        .project
        .balloons
        .iter()
        .filter(|b| b.sheet == sheet.id)
        .cloned()
        .collect();
    for before in balloons {
        let mut after = before.clone();
        after.position = map.point_on_sheet(before.position);
        after.anchor = map.point_on_sheet(before.anchor);
        if after != before {
            tx.push(Change::BalloonChanged {
                before: Box::new(before),
                after: Box::new(after),
            })?;
        }
    }
    let characteristics: Vec<_> = tx
        .project
        .characteristics
        .iter()
        .filter(|c| c.sources.iter().any(|s| s.sheet == sheet.id))
        .cloned()
        .collect();
    for before in characteristics {
        let mut after = before.clone();
        after.sources.retain_mut(|source| {
            if source.sheet != sheet.id {
                return true;
            }
            match map.oriented_box(source.region) {
                Some(region) => {
                    source.region = region;
                    true
                }
                // A region the map loses cannot be shown any more.
                None => false,
            }
        });
        if after != before {
            tx.push(Change::CharacteristicChanged {
                before: Box::new(before),
                after: Box::new(after),
            })?;
        }
    }
    Ok(())
}

/// Number of balloons on `sheet` whose center or leader anchor `map` puts outside the new
/// sheet. The app asks before such an edit (R01 decision 10); applied anyway, they are put on
/// the sheet edge.
pub fn off_sheet_balloons(project: &Project, sheet: SheetId, map: &SheetMap) -> u32 {
    let off = project
        .balloons
        .iter()
        .filter(|b| b.sheet == sheet)
        .filter(|b| {
            [b.position, b.anchor]
                .into_iter()
                .any(|p| map.point(p).is_none_or(|q| !map.on_sheet(q)))
        })
        .count();
    u32::try_from(off).unwrap_or(u32::MAX)
}
