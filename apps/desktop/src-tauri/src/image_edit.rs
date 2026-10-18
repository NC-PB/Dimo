//! The image editor of photo and scan drawings (R.20, R01 decisions 8 to 11).
//!
//! The frontend holds the settings being edited as view state and asks Rust for everything
//! else (rule 2): previews of the editor stages drawn by the tile service, the corners to start
//! from, and the apply, which derives the sheet size and the balloon positions in Rust and asks
//! back before balloons would leave the sheet.

use std::sync::{Arc, Mutex, PoisonError};

use dimo_core::{Homography, ImageEdit, Rotation, Sha256Hex, SheetCorners};
use dimo_pdf::image_source::EditStage;
use dimo_pdf::tiles::TileService;
use dimo_pdf::{ImageSource, RasterSource};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::ipc::{CommandError, DocumentInfo, InternalReason, tile_service};
use crate::project::{emit_patched, with_session};
use crate::session::{AppSession, ImageEditOutcome};
use crate::tiles::TileState;

/// Which step of the edit a preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStage {
    /// The page with its quarter turns only, for the corner handles.
    Corners,
    /// The corrected page without trim and paper, for the trim rectangle.
    Trim,
    /// The sheet as the settings make it.
    Result,
}

impl From<PreviewStage> for EditStage {
    fn from(stage: PreviewStage) -> Self {
        match stage {
            PreviewStage::Corners => Self::Corners,
            PreviewStage::Trim => Self::Trim,
            PreviewStage::Result => Self::Result,
        }
    }
}

/// A preview of one page, ready for the `dimo://tile/...` route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ImagePreview {
    /// The preview drawing; the page is its sheet with the same index.
    pub doc: DocumentInfo,
    /// From pixels of the stored page to the preview sheet, so corner handles can be drawn.
    pub to_view: Homography,
    /// From the preview sheet back to pixels of the stored page.
    pub from_view: Homography,
}

/// Where the corner handles start (R01 decision 8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct CornerStart {
    /// The sheet edges found in the photo, if they were easy to find.
    pub proposed: Option<SheetCorners>,
    /// The corners of the whole page.
    pub page: SheetCorners,
}

/// The preview drawing open in the tile service, closed when the next one opens.
#[derive(Debug, Default)]
pub struct ImagePreviewState {
    open: Mutex<Option<Sha256Hex>>,
}

impl ImagePreviewState {
    /// Notes `key` as the open preview and closes the previous one, unless it is `keep` (the
    /// project's drawing, which a preview of unchanged settings shares).
    pub fn replace(&self, tiles: &TileService, key: Option<Sha256Hex>, keep: Option<Sha256Hex>) {
        let old = {
            let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *open, key)
        };
        if let Some(old) = old
            && Some(old) != key
            && Some(old) != keep
        {
            tiles.close_document(&old);
        }
    }
}

/// Registers a preview of `edit` on `page` at `stage` in `tiles`.
pub fn preview(
    tiles: &TileService,
    source: &dyn RasterSource,
    page: u32,
    edit: &ImageEdit,
    stage: PreviewStage,
) -> Result<ImagePreview, CommandError> {
    let image = as_image(source)?;
    let index = page as usize;
    let stage = EditStage::from(stage);
    let geometry = image.stage_geometry(index, edit, stage)?;
    let mut edits = image.edits().to_vec();
    let slot = edits.get_mut(index).ok_or_else(|| no_page(page))?;
    *slot = stage.edit(edit);
    let preview: Arc<dyn RasterSource> = Arc::new(image.with_edits(&edits)?);
    let registered = tiles.insert_source(preview);
    let from_view = geometry.to_sheet.inverse().ok_or_else(|| {
        CommandError::internal(InternalReason::Bug, "the preview map cannot be inverted")
    })?;
    Ok(ImagePreview {
        doc: DocumentInfo::of(registered.as_ref(), "")?,
        to_view: geometry.to_sheet,
        from_view,
    })
}

/// Corners to start the handles at, named as the sheet reads after `rotation`.
pub fn corner_start(
    source: &dyn RasterSource,
    page: u32,
    rotation: Rotation,
) -> Result<CornerStart, CommandError> {
    let image = as_image(source)?;
    let index = page as usize;
    Ok(CornerStart {
        proposed: image.propose_corners(index, rotation)?,
        page: image.page_corners(index, rotation)?,
    })
}

fn as_image(source: &dyn RasterSource) -> Result<&ImageSource, CommandError> {
    source.as_image().ok_or_else(|| CommandError::Rejected {
        reason: dimo_core::RejectReason::NotAnImage,
        message: "the drawing is not a photo or scan".to_owned(),
    })
}

fn no_page(page: u32) -> CommandError {
    CommandError::Rejected {
        reason: dimo_core::RejectReason::InvalidImageEdit,
        message: format!("the image has no page {page}"),
    }
}

/// The source and render key of the open image drawing.
fn open_image(session: &AppSession) -> Result<(Arc<dyn RasterSource>, Sha256Hex), CommandError> {
    let source = session.image_source()?;
    let key = source.render_key();
    Ok((source, key))
}

/// Previews `edit` on `page` of the open image drawing at `stage` (R.20). Returns the preview
/// drawing for the tile route and the map between stored pixels and the preview sheet.
#[tauri::command]
#[specta::specta]
pub async fn preview_image_edit(
    app: AppHandle,
    page: u32,
    edit: ImageEdit,
    stage: PreviewStage,
) -> Result<ImagePreview, CommandError> {
    let tiles = tile_service(&app.state::<TileState>())?;
    let (source, drawing) = with_session(app.clone(), |_, session| open_image(session)).await?;
    crate::ipc::blocking(move || {
        let preview = preview(&tiles, source.as_ref(), page, &edit, stage)?;
        let key = Sha256Hex::from_hex(&preview.doc.doc);
        app.state::<ImagePreviewState>()
            .replace(&tiles, key, Some(drawing));
        Ok(preview)
    })
    .await
}

/// Closes the preview of the image editor.
#[tauri::command]
#[specta::specta]
pub async fn close_image_preview(app: AppHandle) -> Result<(), CommandError> {
    let tiles = tile_service(&app.state::<TileState>())?;
    let drawing = with_session(app.clone(), |_, session| Ok(session.drawing_key())).await?;
    app.state::<ImagePreviewState>()
        .replace(&tiles, None, drawing);
    Ok(())
}

/// Where the corner handles of `page` start for the quarter turns `rotation`: the sheet edges
/// found in the photo, and the page corners (R01 decision 8).
#[tauri::command]
#[specta::specta]
pub async fn image_corner_start(
    app: AppHandle,
    page: u32,
    rotation: Rotation,
) -> Result<CornerStart, CommandError> {
    let (source, _) = with_session(app, |_, session| open_image(session)).await?;
    crate::ipc::blocking(move || corner_start(source.as_ref(), page, rotation)).await
}

/// Applies `edit` to `page` of the open image drawing as one undo step (R.20): the sheet gets
/// the size the settings give and balloons move with the image. If balloons would leave the
/// sheet, nothing changes unless `move_off_sheet` is set (they are then put on the sheet edge).
/// The patch is also sent as `project-patched`.
#[tauri::command]
#[specta::specta]
pub async fn apply_image_edit(
    app: AppHandle,
    page: u32,
    edit: ImageEdit,
    move_off_sheet: bool,
) -> Result<ImageEditOutcome, CommandError> {
    let tiles = tile_service(&app.state::<TileState>())?;
    with_session(app, move |app, session| {
        let outcome = session.apply_image_edit(&tiles, page, &edit, move_off_sheet)?;
        if let ImageEditOutcome::Applied { patched } = outcome {
            return Ok(ImageEditOutcome::Applied {
                patched: emit_patched(app, patched),
            });
        }
        Ok(outcome)
    })
    .await
}
