//! Errors that reach the frontend, and the one place that maps the errors of the crates onto
//! them (MF F3, rule 2).
//!
//! Every error carries a machine readable `kind` and, where a kind has several causes, a
//! `reason`. The frontend translates kind and reason; `message` is English diagnostic text for
//! the "Details" line, logs and bug reports, never the text the user reads first.
//!
//! The TypeScript twins are generated into `apps/desktop/src/lib/ipc/bindings.ts`, and a
//! frontend test fails if a kind or reason lacks a translation.

use std::fmt::Display;
use std::path::Path;

use dimo_io::project::{CreateError, ProjectError};
use dimo_pdf::PdfError;
use dimo_report::ReportError;
use dimo_tolerance::{ContextError, TableError};
use serde::{Deserialize, Serialize};
use specta::Type;

pub use dimo_core::RejectReason;

/// Error of a command: a machine readable kind (and reason) and a diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandError {
    /// A file or folder could not be read or written.
    #[error("file access failed: {message}")]
    Io {
        /// Why, as far as the operating system tells.
        reason: IoReason,
        /// The operating system error with the path.
        message: String,
    },
    /// The drawing file cannot be opened.
    #[error("cannot open the drawing: {message}")]
    InvalidDocument {
        /// Why.
        reason: DocumentReason,
        /// The reader error.
        message: String,
    },
    /// An argument is malformed, for example a document id that is not a content hash.
    #[error("invalid argument: {message}")]
    InvalidArgument {
        /// Which argument.
        reason: ArgumentReason,
        /// What is wrong.
        message: String,
    },
    /// The action needs an open project.
    #[error("no project is open")]
    NoProject,
    /// The open project has unsaved changes and the caller did not allow discarding them.
    #[error("the project has unsaved changes")]
    UnsavedChanges,
    /// Another DimoMark instance has the project open.
    #[error("the project is open in another DimoMark instance")]
    InUse {
        /// User name of the other instance, if known.
        user: Option<String>,
    },
    /// The project was written by a newer DimoMark (NFR-REL-04). The file is left untouched.
    #[error("written by a newer version (schema {found}, supported {supported})")]
    NewerVersion {
        /// Schema version in the file.
        found: u32,
        /// Highest schema version this build reads.
        supported: u32,
    },
    /// The project file could not be read or used.
    #[error("project file: {message}")]
    Project {
        /// Why.
        reason: ProjectReason,
        /// What went wrong.
        message: String,
    },
    /// An export could not be produced, written or opened; no file was left behind.
    #[error("export failed: {message}")]
    Export {
        /// Why.
        reason: ExportReason,
        /// What went wrong.
        message: String,
    },
    /// The document command was refused; the project is unchanged.
    #[error("command refused: {message}")]
    Rejected {
        /// Machine readable reason, for the frontend to translate.
        reason: RejectReason,
        /// Why, in English, for logs and bug reports.
        message: String,
    },
    /// An entry of the recent projects list cannot be opened.
    #[error("recent project: {message}")]
    Recent {
        /// Why.
        reason: RecentReason,
        /// The entry.
        message: String,
    },
    /// A background task stopped before it answered (it panicked or was cancelled).
    #[error("background task failed: {message}")]
    TaskFailed {
        /// The join error.
        message: String,
    },
    /// Something inside the app is broken: a bundled resource, the window, or a bug.
    #[error("internal error: {message}")]
    Internal {
        /// What broke.
        reason: InternalReason,
        /// What went wrong.
        message: String,
    },
}

/// Why a file could not be read or written, as far as the operating system tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IoReason {
    /// The file or folder does not exist.
    NotFound,
    /// The user may not read or write it, or the drive is read only.
    PermissionDenied,
    /// Another program has the file open and locked it (Windows), or it is busy.
    TargetLocked,
    /// The disk or the user's quota is full.
    DiskFull,
    /// Any other operating system error.
    Other,
}

impl IoReason {
    /// The reason of an operating system error.
    pub fn of(error: &std::io::Error) -> Self {
        use std::io::ErrorKind as K;
        // Windows: ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION, ERROR_USER_MAPPED_FILE. A file
        // open in a spreadsheet program gives one of these.
        if cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33 | 1224)) {
            return Self::TargetLocked;
        }
        match error.kind() {
            K::NotFound => Self::NotFound,
            K::PermissionDenied | K::ReadOnlyFilesystem => Self::PermissionDenied,
            K::ResourceBusy | K::ExecutableFileBusy => Self::TargetLocked,
            K::StorageFull | K::QuotaExceeded | K::FileTooLarge => Self::DiskFull,
            _ => Self::Other,
        }
    }
}

/// Why a drawing file cannot be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DocumentReason {
    /// The file is neither a PDF nor an image.
    NotAPdf,
    /// The file is a HEIC or HEIF photo, which DimoMark does not read (R01 decision 7).
    HeicNotSupported,
    /// The PDF reader cannot read the file: damaged, password protected or unusual.
    Damaged,
    /// A photo or scan cannot be read: damaged, too large or an unsupported colour type.
    ImageDamaged,
    /// The file is larger than the limit for drawings.
    TooLarge,
    /// The PDF component (PDFium) is not loaded, so no PDF can be opened.
    NoPdfEngine,
}

/// Which argument of a command is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentReason {
    /// The shorter leg is not a positive number (T2.9a).
    ShorterLeg,
    /// A box needs a finite position and a size above zero.
    Region,
    /// A document id is not a content hash.
    DocumentId,
    /// Tauri could not read the arguments of the call (set by the frontend, which receives
    /// that error as plain text).
    Unreadable,
    /// Any other malformed request, for example a dev script.
    Request,
}

/// Why a project file cannot be read or used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ProjectReason {
    /// The file is no DimoMark project.
    NotAProject,
    /// The container or one of its entries is damaged or unsafe.
    DamagedContainer,
    /// The project is larger than the read limits.
    TooLarge,
    /// Converting the project from an older format failed.
    MigrationFailed,
    /// The drawing file of a revision is not in the project.
    MissingDrawing,
    /// A company table the settings name is not in the project.
    MissingTable,
    /// A company table in the project was changed outside DimoMark.
    ChangedTable,
    /// The project was never saved, so it has no file yet.
    NoFile,
}

/// Why an export failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportReason {
    /// Another program has the target file open (Windows), or it is busy.
    TargetLocked,
    /// The user may not write the target folder or file.
    PermissionDenied,
    /// The disk is full.
    DiskFull,
    /// Writing the file failed for another reason, for example a missing folder.
    WriteFailed,
    /// The target is not a file name.
    InvalidFileName,
    /// The ballooned PDF needs the PDF component (PDFium), which is not loaded.
    NoPdfEngine,
    /// The PDF component could not write the ballooned PDF.
    PdfFailed,
    /// The report could not be built or rendered.
    ReportFailed,
    /// The characteristic list (CSV, Excel) could not be written.
    ListFailed,
    /// The written file was not opened: its extension is not the one of the format.
    WrongExtension,
    /// The written file could not be opened with the default application.
    OpenFailed,
}

impl ExportReason {
    /// The reason of an operating system error while writing the export.
    pub fn of_io(error: &std::io::Error) -> Self {
        match IoReason::of(error) {
            IoReason::TargetLocked => Self::TargetLocked,
            IoReason::PermissionDenied => Self::PermissionDenied,
            IoReason::DiskFull => Self::DiskFull,
            IoReason::NotFound | IoReason::Other => Self::WriteFailed,
        }
    }
}

/// Why an entry of the recent projects list cannot be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecentReason {
    /// The path is not on the list; only listed paths are opened.
    NotListed,
    /// The file no longer exists. It stays on the list until the user removes it.
    Missing,
}

/// What inside the app is broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InternalReason {
    /// The tolerance tables shipped with the app cannot be loaded.
    ShippedTables,
    /// The report template shipped with the app cannot be loaded.
    ReportTemplate,
    /// The window refused an action, for example closing.
    Window,
    /// An inconsistency that should never happen.
    Bug,
}

/// Why the autosave journal is not written (NFR-REL-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AutosaveReason {
    /// There is no autosave folder and the project was never saved.
    NoFolder,
    /// The disk is full.
    DiskFull,
    /// The user may not write the autosave folder or the project folder.
    PermissionDenied,
    /// Writing failed for another reason.
    WriteFailed,
}

impl AutosaveReason {
    /// The reason of a failed journal write.
    pub fn of(error: &ProjectError) -> Self {
        match error {
            ProjectError::Io { source, .. } => match IoReason::of(source) {
                IoReason::DiskFull => Self::DiskFull,
                IoReason::PermissionDenied => Self::PermissionDenied,
                _ => Self::WriteFailed,
            },
            _ => Self::WriteFailed,
        }
    }
}

/// Why a company table file was refused at import (FR-TOL-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TableImportReason {
    /// The file is larger than 16 MiB.
    TooLarge,
    /// The file cannot be read.
    Unreadable,
    /// The file is not UTF-8 text.
    NotText,
    /// The file is not valid TOML or does not match the table format.
    Syntax,
    /// The file matches the format but breaks a rule of tables (values, rows, header).
    InvalidContent,
    /// The table is not of kind `custom`.
    NotCompanyTable,
    /// The table id is the id of a shipped table.
    DuplicateId,
    /// The table does not fit the tolerance settings of the project, for example a class they
    /// use is missing.
    SettingsConflict,
}

impl TableImportReason {
    /// The reason of a refused import.
    pub fn of(error: &ContextError) -> Self {
        match error {
            ContextError::Table(TableError::Io { .. }) => Self::Unreadable,
            ContextError::Table(TableError::Parse { .. }) => Self::Syntax,
            ContextError::Table(TableError::Invalid { .. }) => Self::InvalidContent,
            ContextError::Table(TableError::Duplicate { .. }) => Self::DuplicateId,
            ContextError::WrongKind { .. } => Self::NotCompanyTable,
            ContextError::Table(TableError::Mismatch { .. })
            | ContextError::Settings(_)
            | ContextError::MissingTable(_)
            | ContextError::Version { .. }
            | ContextError::UnknownClass { .. } => Self::SettingsConflict,
        }
    }
}

impl CommandError {
    /// An operating system error on `path`.
    pub fn io(path: &Path, error: &std::io::Error) -> Self {
        Self::Io {
            reason: IoReason::of(error),
            message: format!("{}: {error}", path.display()),
        }
    }

    /// A malformed argument.
    pub fn argument(reason: ArgumentReason, message: impl Display) -> Self {
        Self::InvalidArgument {
            reason,
            message: message.to_string(),
        }
    }

    /// An export failure.
    pub fn export(reason: ExportReason, message: impl Display) -> Self {
        Self::Export {
            reason,
            message: message.to_string(),
        }
    }

    /// An operating system error while writing the export file `path`.
    pub fn export_io(path: &Path, error: &std::io::Error) -> Self {
        Self::export(
            ExportReason::of_io(error),
            format_args!("{}: {error}", path.display()),
        )
    }

    /// The PDF component failed while writing the ballooned PDF.
    pub fn export_pdf(error: &PdfError) -> Self {
        let reason = if is_engine_error(error) {
            ExportReason::NoPdfEngine
        } else {
            ExportReason::PdfFailed
        };
        Self::export(reason, error)
    }

    /// Something inside the app is broken.
    pub fn internal(reason: InternalReason, message: impl Display) -> Self {
        Self::Internal {
            reason,
            message: message.to_string(),
        }
    }

    /// A background task that did not answer; `error` is the join error.
    pub fn task_failed(error: impl Display) -> Self {
        Self::TaskFailed {
            message: error.to_string(),
        }
    }

    /// The PDF component is not loaded.
    pub fn no_pdf_engine(message: impl Display) -> Self {
        Self::InvalidDocument {
            reason: DocumentReason::NoPdfEngine,
            message: message.to_string(),
        }
    }
}

/// Whether `error` means that PDFium is missing or stopped, rather than a problem of the file.
fn is_engine_error(error: &PdfError) -> bool {
    matches!(
        error,
        PdfError::LibraryNotFound { .. }
            | PdfError::LibraryLoad { .. }
            | PdfError::NoPdfium
            | PdfError::EngineStopped
    )
}

impl From<ProjectError> for CommandError {
    fn from(error: ProjectError) -> Self {
        use ProjectReason as R;
        let reason = match &error {
            ProjectError::InUse { holder } => {
                return Self::InUse {
                    user: holder.as_ref().map(|h| h.user.clone()),
                };
            }
            ProjectError::NewerVersion { found, supported } => {
                return Self::NewerVersion {
                    found: *found,
                    supported: *supported,
                };
            }
            ProjectError::Io { path, source } => return Self::io(path, source),
            ProjectError::InvalidDrawing(_) => {
                return Self::InvalidDocument {
                    reason: DocumentReason::Damaged,
                    message: error.to_string(),
                };
            }
            ProjectError::NotAProject(_) => R::NotAProject,
            ProjectError::Container(_)
            | ProjectError::UnknownVersion(_)
            | ProjectError::MissingEntry(_)
            | ProjectError::UnsafeEntry(_)
            | ProjectError::DuplicateEntry(_)
            | ProjectError::Symlink(_)
            | ProjectError::Encrypted(_)
            | ProjectError::TooManyEntries(_)
            | ProjectError::Json { .. }
            | ProjectError::HashMismatch { .. }
            | ProjectError::InvalidTable(_) => R::DamagedContainer,
            ProjectError::TooLarge { .. } => R::TooLarge,
            ProjectError::Migration { .. } => R::MigrationFailed,
            ProjectError::MissingDrawing(_) => R::MissingDrawing,
            ProjectError::MissingTable(_) => R::MissingTable,
            ProjectError::TableHashMismatch { .. } => R::ChangedTable,
            ProjectError::NoPath => R::NoFile,
        };
        Self::Project {
            reason,
            message: error.to_string(),
        }
    }
}

impl From<CreateError> for CommandError {
    fn from(error: CreateError) -> Self {
        let message = error.to_string();
        let document = |reason| Self::InvalidDocument {
            reason,
            message: message.clone(),
        };
        match error {
            CreateError::Read { path, source } => Self::io(&path, &source),
            CreateError::Project(ProjectError::TooLarge { .. }) => {
                document(DocumentReason::TooLarge)
            }
            CreateError::Project(error) => error.into(),
            CreateError::Open(error) => error.into(),
            CreateError::UnknownType => document(DocumentReason::NotAPdf),
            CreateError::HeicNotSupported => document(DocumentReason::HeicNotSupported),
        }
    }
}

impl From<PdfError> for CommandError {
    fn from(error: PdfError) -> Self {
        if matches!(error, PdfError::ImageEdit(_)) {
            // Settings of the image editor that give no sheet (R.20).
            return Self::Rejected {
                reason: RejectReason::InvalidImageEdit,
                message: error.to_string(),
            };
        }
        let reason = if is_engine_error(&error) {
            DocumentReason::NoPdfEngine
        } else if matches!(error, PdfError::Image(_)) {
            DocumentReason::ImageDamaged
        } else {
            DocumentReason::Damaged
        };
        Self::InvalidDocument {
            reason,
            message: error.to_string(),
        }
    }
}

impl From<dimo_core::CommandError> for CommandError {
    fn from(error: dimo_core::CommandError) -> Self {
        Self::Rejected {
            reason: error.reject_reason(),
            message: error.to_string(),
        }
    }
}

impl From<ReportError> for CommandError {
    fn from(error: ReportError) -> Self {
        Self::export(ExportReason::ReportFailed, error)
    }
}

impl From<dimo_io::export::ExportError> for CommandError {
    fn from(error: dimo_io::export::ExportError) -> Self {
        use dimo_io::export::ExportError;
        let reason = match &error {
            ExportError::Io(source) => ExportReason::of_io(source),
            ExportError::Csv(_) | ExportError::Xlsx(_) | ExportError::Zip(_) => {
                ExportReason::ListFailed
            }
        };
        Self::export(reason, error)
    }
}

impl From<dimo_export::ExportError> for CommandError {
    fn from(error: dimo_export::ExportError) -> Self {
        use dimo_export::ExportError as E;
        match error {
            E::Report(e) => e.into(),
            E::Template(e) => Self::internal(InternalReason::ReportTemplate, e),
            E::List(e) => e.into(),
            E::Pdf(e) => Self::export_pdf(&e),
            E::NoPdfEngine => Self::export(ExportReason::NoPdfEngine, "no PDF engine"),
            E::NoDrawing | E::NotAReport(_) | E::NotPrepared | E::InvalidTime(_) => {
                Self::internal(InternalReason::Bug, error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn io_errors_get_a_reason() {
        let reason = |kind: ErrorKind| IoReason::of(&Error::from(kind));
        assert_eq!(reason(ErrorKind::NotFound), IoReason::NotFound);
        assert_eq!(
            reason(ErrorKind::PermissionDenied),
            IoReason::PermissionDenied
        );
        assert_eq!(
            reason(ErrorKind::ReadOnlyFilesystem),
            IoReason::PermissionDenied
        );
        assert_eq!(reason(ErrorKind::ResourceBusy), IoReason::TargetLocked);
        assert_eq!(reason(ErrorKind::StorageFull), IoReason::DiskFull);
        assert_eq!(reason(ErrorKind::InvalidData), IoReason::Other);
        assert_eq!(
            ExportReason::of_io(&Error::from(ErrorKind::NotFound)),
            ExportReason::WriteFailed
        );
        assert_eq!(
            ExportReason::of_io(&Error::from(ErrorKind::StorageFull)),
            ExportReason::DiskFull
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_file_open_in_another_program_is_locked() {
        assert_eq!(
            IoReason::of(&Error::from_raw_os_error(32)),
            IoReason::TargetLocked
        );
    }

    #[test]
    fn refused_commands_carry_a_machine_readable_reason() {
        let locked = CommandError::from(dimo_core::CommandError::NumberingLocked);
        assert_eq!(
            locked,
            CommandError::Rejected {
                reason: RejectReason::NumberingLocked,
                message: "numbering is locked".to_owned(),
            }
        );
        let json = serde_json::to_value(&locked).unwrap();
        assert_eq!(json["kind"], "rejected");
        assert_eq!(json["reason"], "numbering_locked");
    }

    #[test]
    fn project_errors_map_to_kind_and_reason() {
        let io = CommandError::from(ProjectError::Io {
            path: PathBuf::from("a.dimo"),
            source: Error::from(ErrorKind::PermissionDenied),
        });
        assert!(matches!(
            io,
            CommandError::Io { reason: IoReason::PermissionDenied, ref message } if message.starts_with("a.dimo: ")
        ));
        let json = serde_json::to_value(&io).unwrap();
        assert_eq!(json["kind"], "io");
        assert_eq!(json["reason"], "permission_denied");
        assert!(matches!(
            CommandError::from(ProjectError::Container("bad zip".to_owned())),
            CommandError::Project {
                reason: ProjectReason::DamagedContainer,
                ..
            }
        ));
        assert!(matches!(
            CommandError::from(ProjectError::MissingTable("shop".to_owned())),
            CommandError::Project {
                reason: ProjectReason::MissingTable,
                ..
            }
        ));
        assert_eq!(
            CommandError::from(ProjectError::NewerVersion {
                found: 9,
                supported: 4
            }),
            CommandError::NewerVersion {
                found: 9,
                supported: 4
            }
        );
        assert_eq!(
            CommandError::from(ProjectError::InUse { holder: None }),
            CommandError::InUse { user: None }
        );
    }

    #[test]
    fn drawing_errors_map_to_document_reasons() {
        let reason = |error: CreateError| match CommandError::from(error) {
            CommandError::InvalidDocument { reason, .. } => Some(reason),
            _ => None,
        };
        assert_eq!(
            reason(CreateError::UnknownType),
            Some(DocumentReason::NotAPdf)
        );
        assert_eq!(
            reason(CreateError::HeicNotSupported),
            Some(DocumentReason::HeicNotSupported)
        );
        assert_eq!(
            reason(CreateError::Project(ProjectError::TooLarge {
                name: "big.pdf".to_owned(),
                limit: 1
            })),
            Some(DocumentReason::TooLarge)
        );
        assert_eq!(
            reason(CreateError::Open(PdfError::Open("broken".to_owned()))),
            Some(DocumentReason::Damaged)
        );
        assert_eq!(
            reason(CreateError::Open(PdfError::NoPdfium)),
            Some(DocumentReason::NoPdfEngine)
        );
        assert_eq!(
            reason(CreateError::Open(PdfError::Image("cut off".to_owned()))),
            Some(DocumentReason::ImageDamaged)
        );
        assert!(matches!(
            CommandError::from(CreateError::Read {
                path: PathBuf::from("x.pdf"),
                source: Error::from(ErrorKind::NotFound),
            }),
            CommandError::Io {
                reason: IoReason::NotFound,
                ..
            }
        ));
    }

    #[test]
    fn engine_errors_in_exports_are_no_pdf_engine() {
        assert!(matches!(
            CommandError::export_pdf(&PdfError::EngineStopped),
            CommandError::Export {
                reason: ExportReason::NoPdfEngine,
                ..
            }
        ));
        assert!(matches!(
            CommandError::export_pdf(&PdfError::Write("x".to_owned())),
            CommandError::Export {
                reason: ExportReason::PdfFailed,
                ..
            }
        ));
    }

    #[test]
    fn registry_errors_keep_their_reasons() {
        use dimo_export::ExportError as E;
        let reason = |error: E| match CommandError::from(error) {
            CommandError::Export { reason, .. } => Some(reason),
            _ => None,
        };
        assert_eq!(reason(E::NoPdfEngine), Some(ExportReason::NoPdfEngine));
        assert_eq!(
            reason(E::Pdf(PdfError::Write("x".to_owned()))),
            Some(ExportReason::PdfFailed)
        );
        assert_eq!(
            reason(E::Report(ReportError::InvalidOptions("x".to_owned()))),
            Some(ExportReason::ReportFailed)
        );
        assert!(matches!(
            CommandError::from(E::Template(ReportError::UnknownTemplate("x".to_owned()))),
            CommandError::Internal {
                reason: InternalReason::ReportTemplate,
                ..
            }
        ));
        assert!(matches!(
            CommandError::from(E::NoDrawing),
            CommandError::Internal {
                reason: InternalReason::Bug,
                ..
            }
        ));
    }

    #[test]
    fn table_import_errors_get_a_reason() {
        let parse = ContextError::Table(TableError::Parse {
            origin: "a.toml".to_owned(),
            message: "expected =".to_owned(),
            line: Some(3),
        });
        assert_eq!(TableImportReason::of(&parse), TableImportReason::Syntax);
        assert_eq!(
            TableImportReason::of(&ContextError::WrongKind {
                id: "iso".to_owned(),
                usage: "custom table"
            }),
            TableImportReason::NotCompanyTable
        );
        assert_eq!(
            AutosaveReason::of(&ProjectError::Io {
                path: PathBuf::from("j"),
                source: Error::from(ErrorKind::StorageFull),
            }),
            AutosaveReason::DiskFull
        );
    }
}
