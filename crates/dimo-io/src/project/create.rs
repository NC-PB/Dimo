//! Creating a project from a drawing file, and opening the drawing of a project (FR-DOC-07,
//! MF F1, ADR 0010).
//!
//! [`create_from_drawing_file`] is the one way a new project comes into being: it reads the
//! file, finds out what it is from its first bytes, opens it through a
//! [`RasterOpener`] (the tile service in the app, the PDF engine in tools), reads the sheets,
//! imports the revision and starts the [`ProjectSession`], autosaved if an autosave folder is
//! given. Callers never touch PDFium for this. PDFs and JPEG, PNG and TIFF images (R.19) are
//! supported; an image starts with the default edit settings on every page. HEIC photos are
//! recognized and refused with [`CreateError::HeicNotSupported`] (R01 decision 7).
//!
//! [`open_drawing`] opens the drawing of the current revision of a project through the source
//! its revision names.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dimo_core::{
    DrawingSource, Environment, ImageEdit, ImageFormat, ProjectInfo, ReportInfo, Size,
};
use dimo_pdf::{Opened, PdfError, RasterOpener, RasterSource};

use super::Limits;
use super::error::ProjectError;
use super::import::{ImportedDrawing, SheetInfo, import_drawing};
use super::session::{CreateOptions, ProjectSession};
use crate::autosave;
use crate::lock::{LockOwner, ProjectLock};

/// What a drawing file is, from its first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingType {
    /// A PDF: `%PDF-` within the first 1024 bytes, as PDF readers accept it.
    Pdf,
    /// A JPEG, PNG or TIFF image.
    Image(ImageFormat),
    /// A HEIC or HEIF photo, which Dimo does not read (R01 decision 7).
    Heic,
}

/// ISO base media file brands of HEIC and HEIF still images.
const HEIF_BRANDS: [&[u8; 4]; 8] = [
    b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1", b"msf1",
];

/// The type of a drawing file from its first bytes, `None` for anything else.
pub fn sniff(bytes: &[u8]) -> Option<DrawingType> {
    let head = &bytes[..bytes.len().min(1024)];
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(DrawingType::Image(ImageFormat::Jpeg))
    } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(DrawingType::Image(ImageFormat::Png))
    } else if head.starts_with(b"II*\0") || head.starts_with(b"MM\0*") {
        Some(DrawingType::Image(ImageFormat::Tiff))
    } else if head.get(4..8) == Some(b"ftyp")
        && head
            .get(8..12)
            .is_some_and(|brand| HEIF_BRANDS.iter().any(|b| b.as_slice() == brand))
    {
        Some(DrawingType::Heic)
    } else if head.windows(5).any(|w| w == b"%PDF-") {
        Some(DrawingType::Pdf)
    } else {
        None
    }
}

/// Where a new project journals before its first save (NFR-REL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosaveTarget {
    /// The autosave folder in the app data folder; created if missing.
    pub dir: PathBuf,
    /// Who locks the autosave base file.
    pub owner: LockOwner,
}

/// How a new project starts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewProject {
    /// Part and order data.
    pub info: ProjectInfo,
    /// Report info the project starts with (M3 decision 15): initial state, no undo step.
    pub report_info: ReportInfo,
    /// Autosave folder; `None` leaves the project unprotected until its first save.
    pub autosave: Option<AutosaveTarget>,
}

/// A new project and the opened drawing.
#[derive(Debug)]
pub struct CreatedProject {
    /// The project, not saved yet.
    pub session: ProjectSession,
    /// The drawing of its revision, open in the opener (the tile service in the app).
    pub source: Arc<dyn RasterSource>,
    /// Lock of the autosave base file, if the project is autosaved.
    pub lock: Option<ProjectLock>,
    /// Why the project is not autosaved although an autosave folder was given. The project is
    /// created anyway; its changes are not protected until the first save.
    pub autosave_error: Option<ProjectError>,
}

/// Why no project could be created from a drawing file.
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// The operating system error.
        source: std::io::Error,
    },
    /// Neither a PDF nor a supported image.
    #[error("not a PDF or image drawing")]
    UnknownType,
    /// A HEIC or HEIF photo (R01 decision 7).
    #[error("HEIC photos are not supported; save the photo as JPEG")]
    HeicNotSupported,
    /// The reader could not open the file or read its sheets.
    #[error(transparent)]
    Open(#[from] PdfError),
    /// The drawing or the project could not be stored.
    #[error(transparent)]
    Project(#[from] ProjectError),
}

/// Creates a new, unsaved project from the drawing file at `path` (FR-DOC-07). See the module
/// docs. IDs, times and the autosave file name come from `env` (rule 11).
pub fn create_from_drawing_file(
    path: &Path,
    opener: &dyn RasterOpener,
    options: NewProject,
    env: &mut dyn Environment,
) -> Result<CreatedProject, CreateError> {
    let read = |source| CreateError::Read {
        path: path.to_owned(),
        source,
    };
    let limit = Limits::DEFAULT.max_drawing;
    if fs::metadata(path).map_err(read)?.len() > limit {
        return Err(ProjectError::TooLarge {
            name: path.display().to_string(),
            limit,
        }
        .into());
    }
    let bytes = fs::read(path).map_err(read)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    create_from_drawing(bytes, &name, opener, options, env)
}

/// [`create_from_drawing_file`] for bytes already read; `file_name` is stored with the
/// revision (only its last path component).
pub fn create_from_drawing(
    bytes: Vec<u8>,
    file_name: &str,
    opener: &dyn RasterOpener,
    options: NewProject,
    env: &mut dyn Environment,
) -> Result<CreatedProject, CreateError> {
    let kind = sniff(&bytes);
    let handle = match kind {
        Some(DrawingType::Pdf) => opener.open_pdf(bytes.clone())?,
        Some(DrawingType::Image(format)) => opener.open_image(bytes.clone(), format, None)?,
        Some(DrawingType::Heic) => return Err(CreateError::HeicNotSupported),
        None => return Err(CreateError::UnknownType),
    };
    let source = &handle.source;
    let created = (|| {
        let sheets = sheets_of(source.as_ref())?;
        let source_kind = match kind {
            Some(DrawingType::Image(format)) => DrawingSource::Image {
                format,
                pages: vec![ImageEdit::default(); sheets.len()],
            },
            _ => DrawingSource::Pdf,
        };
        let hash = *source.content_hash();
        let drawing = import_drawing(bytes, file_name, source_kind, &hash, &sheets, env)?;
        Ok::<_, CreateError>(start_session(drawing, options, env)?)
    })();
    match created {
        Ok((session, lock, autosave_error)) => Ok(CreatedProject {
            session,
            source: handle.source,
            lock,
            autosave_error,
        }),
        Err(error) => {
            // Every failure after opening gives a source this call opened back.
            if handle.newly_opened {
                opener.release(&source.render_key());
            }
            Err(error)
        }
    }
}

/// What the reader finds on each sheet.
fn sheets_of(source: &dyn RasterSource) -> Result<Vec<SheetInfo>, PdfError> {
    (0..source.sheet_count())
        .map(|sheet| {
            let size = source.sheet_size(sheet)?;
            Ok(SheetInfo {
                size: Size {
                    width: size.width,
                    height: size.height,
                },
                kind: source.sheet_kind(sheet)?,
                raster_dpi: source.raster_dpi(sheet),
            })
        })
        .collect()
}

/// The session, autosaved against a new locked base file if possible. An autosave failure
/// does not stop the project: it starts without autosave and the error is handed back.
fn start_session(
    drawing: ImportedDrawing,
    options: NewProject,
    env: &mut dyn Environment,
) -> Result<(ProjectSession, Option<ProjectLock>, Option<ProjectError>), ProjectError> {
    let NewProject {
        info,
        report_info,
        autosave,
    } = options;
    let mut autosave_error = None;
    if let Some(target) = autosave {
        let autosaved = (|| {
            fs::create_dir_all(&target.dir).map_err(|e| ProjectError::io(&target.dir, e))?;
            let base = autosave::base_path(&target.dir, &env.new_uuid().to_string());
            let lock = ProjectLock::acquire(&base, &target.owner)?;
            let options = CreateOptions {
                info: info.clone(),
                report_info: report_info.clone(),
                autosave_base: Some(base),
            };
            Ok::<_, ProjectError>((ProjectSession::create(drawing.clone(), options)?, lock))
        })();
        match autosaved {
            Ok((session, lock)) => return Ok((session, Some(lock), None)),
            Err(error) => {
                tracing::warn!("new project without autosave: {error}");
                autosave_error = Some(error);
            }
        }
    }
    let options = CreateOptions {
        info,
        report_info,
        autosave_base: None,
    };
    Ok((
        ProjectSession::create(drawing, options)?,
        None,
        autosave_error,
    ))
}

/// Opens the drawing of the current revision of `project` through the source its revision
/// names (MF F1).
pub fn open_drawing(
    project: &ProjectSession,
    opener: &dyn RasterOpener,
) -> Result<Opened, CreateError> {
    let revision = project.project().current_revision().ok_or_else(|| {
        ProjectError::InvalidDrawing("the project has no current drawing revision".into())
    })?;
    let bytes = project
        .drawing(&revision.sha256)
        .ok_or(ProjectError::MissingDrawing(revision.sha256))?;
    match &revision.source {
        DrawingSource::Pdf => Ok(opener.open_pdf(bytes.to_vec())?),
        DrawingSource::Image { format, pages } => {
            let image = opener.open_image(bytes.to_vec(), *format, Some(pages))?;
            // Since R.20 an edit sets the sheet sizes with the settings (`plan_image_edit`).
            // Projects edited before then can still differ; their balloons keep the sheet
            // space of the project.
            let stored: Vec<Size> = revision.sheets.iter().map(|s| s.size).collect();
            if image.source.sheet_sizes()? != stored {
                tracing::warn!(
                    revision = %revision.id,
                    "image sheet sizes differ from the project sheets after an edit"
                );
            }
            Ok(image)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_reads_the_first_bytes() {
        assert_eq!(sniff(b"%PDF-1.7\n"), Some(DrawingType::Pdf));
        // PDF readers accept junk before the header.
        assert_eq!(sniff(b"\r\n\x00junk%PDF-1.4"), Some(DrawingType::Pdf));
        let mut late = vec![b' '; 1024];
        late.extend_from_slice(b"%PDF-1.4");
        assert_eq!(sniff(&late), None, "only the first 1024 bytes count");
        assert_eq!(
            sniff(b"\xFF\xD8\xFF\xDB"),
            Some(DrawingType::Image(ImageFormat::Jpeg))
        );
        assert_eq!(
            sniff(b"\x89PNG\r\n\x1a\n"),
            Some(DrawingType::Image(ImageFormat::Png))
        );
        assert_eq!(sniff(b"II*\0"), Some(DrawingType::Image(ImageFormat::Tiff)));
        assert_eq!(sniff(b"MM\0*"), Some(DrawingType::Image(ImageFormat::Tiff)));
        assert_eq!(
            sniff(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"),
            Some(DrawingType::Heic)
        );
        // mif1 is the generic HEIF brand; an AVIF file can carry it too and then gets the
        // HEIC message, which also fits (neither is read).
        assert_eq!(sniff(b"\0\0\0\x18ftypmif1"), Some(DrawingType::Heic));
        // Other ISO media files (a video) are not drawings.
        assert_eq!(sniff(b"\0\0\0\x18ftypisom"), None);
        assert_eq!(sniff(b""), None);
    }
}
