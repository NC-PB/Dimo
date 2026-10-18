//! The image edit command with the sheet geometry that follows from it (R.20, ADR 0010).
//!
//! The frontend sends only the new settings. [`plan_image_edit`] derives from the open image
//! the sheet the settings give and the map that keeps balloons on their features, and counts
//! the balloons it would move off the sheet, so the app can ask first (R01 decision 10).

use dimo_core::{Command, ImageEdit, Project, off_sheet_balloons};
use dimo_pdf::{PdfError, RasterSource};

/// What applying an image edit would do.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageEditPlan {
    /// The command, with its geometry.
    pub command: Command,
    /// Balloons on the page's sheet whose center or anchor would land off the new sheet.
    pub off_sheet: u32,
}

/// Plans new settings for one page of the current revision, which must be the image `source`
/// was opened from with the revision's current settings.
pub fn plan_image_edit(
    project: &Project,
    source: &dyn RasterSource,
    page: u32,
    edit: &ImageEdit,
) -> Result<ImageEditPlan, PdfError> {
    let revision = project
        .current_revision()
        .ok_or_else(|| PdfError::ImageEdit("the project has no drawing revision".to_owned()))?;
    let pages = revision
        .source
        .image_pages()
        .ok_or_else(|| PdfError::ImageEdit("the drawing is not a photo or scan".to_owned()))?;
    let image = source
        .as_image()
        .filter(|image| image.edits() == pages)
        .ok_or_else(|| {
            PdfError::ImageEdit("the open drawing does not match the project".to_owned())
        })?;
    edit.validate()
        .map_err(|e| PdfError::ImageEdit(e.to_owned()))?;
    let index = usize::try_from(page).unwrap_or(usize::MAX);
    let old = pages
        .get(index)
        .ok_or_else(|| PdfError::ImageEdit("the image has no such page".to_owned()))?;
    let same_sheet = ImageEdit {
        clean_up: edit.clean_up,
        ..old.clone()
    } == *edit;
    if same_sheet {
        // Only the clean up changes: the sheet and everything on it stay exactly as they are.
        return Ok(ImageEditPlan {
            command: Command::SetImageEdit {
                revision: revision.id,
                page,
                edit: Box::new(edit.clone()),
                geometry: None,
            },
            off_sheet: 0,
        });
    }
    let map = image.sheet_map(index, edit)?;
    let sheet = revision
        .sheets
        .iter()
        .find(|s| s.index == page)
        .ok_or_else(|| PdfError::ImageEdit("the page has no sheet".to_owned()))?;
    Ok(ImageEditPlan {
        command: Command::SetImageEdit {
            revision: revision.id,
            page,
            edit: Box::new(edit.clone()),
            geometry: Some(Box::new(map)),
        },
        off_sheet: off_sheet_balloons(project, sheet.id, &map),
    })
}
