//! Errors of reading, writing and importing projects.

use std::path::PathBuf;

use dimo_core::Sha256Hex;

use crate::lock::LockOwner;

/// Why a project could not be read, written or a drawing not imported.
///
/// A failed read never changes the file on disk (NFR-REL-04).
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// The file system refused a read or write.
    #[error("cannot access {path}: {source}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The file is no ZIP container or the container is damaged.
    #[error("not a readable project container: {0}")]
    Container(String),
    /// The manifest does not name the Dimo project format.
    #[error("not a Dimo project: {0}")]
    NotAProject(String),
    /// Written by a newer Dimo (NFR-REL-04). The file is left untouched.
    #[error(
        "the project was written by a newer version of Dimo (schema version {found}, this \
         version reads up to {supported}). Update Dimo to open it"
    )]
    NewerVersion {
        /// Schema version in the file.
        found: u32,
        /// Highest schema version this build reads.
        supported: u32,
    },
    /// Schema version 0 or another value no Dimo ever wrote.
    #[error("unknown schema version {0}")]
    UnknownVersion(u32),
    /// A required entry is missing.
    #[error("the project has no {0}")]
    MissingEntry(&'static str),
    /// An entry name could escape the container (zip slip) or is malformed (NFR-SEC-05).
    #[error("unsafe entry name {0:?}")]
    UnsafeEntry(String),
    /// An entry appears twice.
    #[error("duplicate entry {0:?}")]
    DuplicateEntry(String),
    /// An entry is a symbolic link (NFR-SEC-05).
    #[error("entry {0:?} is a symbolic link")]
    Symlink(String),
    /// An entry is encrypted.
    #[error("entry {0:?} is encrypted")]
    Encrypted(String),
    /// An entry or the whole project is larger than the read limits.
    #[error("{name} is larger than the limit of {limit} bytes")]
    TooLarge {
        /// Entry name, or `project` for the total.
        name: String,
        /// The limit that was exceeded.
        limit: u64,
    },
    /// The container has more entries than the read limits allow.
    #[error("the container has more than {0} entries")]
    TooManyEntries(usize),
    /// An entry is not valid JSON for its type.
    #[error("{entry}: {message}")]
    Json {
        /// Entry name, with line number for JSON Lines.
        entry: String,
        /// Parser message.
        message: String,
    },
    /// A migration step failed.
    #[error("migration from schema version {from} failed: {message}")]
    Migration {
        /// Version the failed step started from.
        from: u32,
        /// What went wrong.
        message: String,
    },
    /// A drawing revision refers to a file that is not in the project.
    #[error("drawing {0} is missing from the project")]
    MissingDrawing(Sha256Hex),
    /// A stored or imported drawing does not have the expected SHA-256 (FR-DOC-07).
    #[error("drawing hash mismatch: expected {expected}, the bytes hash to {actual}")]
    HashMismatch {
        /// The hash the drawing should have.
        expected: Sha256Hex,
        /// The hash of the bytes.
        actual: Sha256Hex,
    },
    /// The tolerance settings list a custom table that is not in the project (M2 decision 4).
    #[error("custom tolerance table {0} is missing from the project")]
    MissingTable(String),
    /// A stored custom table does not have the SHA-256 the settings expect.
    #[error(
        "custom tolerance table {id} was changed: expected {expected}, the bytes hash to {actual}"
    )]
    TableHashMismatch {
        /// Table id.
        id: String,
        /// The hash the settings expect.
        expected: Sha256Hex,
        /// The hash of the stored bytes.
        actual: Sha256Hex,
    },
    /// A custom table id is not a valid file name or appears twice.
    #[error("invalid or duplicate custom tolerance table id {0:?}")]
    InvalidTable(String),
    /// The data handed to the drawing import is not usable.
    #[error("cannot import the drawing: {0}")]
    InvalidDrawing(String),
    /// [`ProjectSession::save`](crate::project::ProjectSession::save) on a project that was
    /// never saved. Use `save_as`.
    #[error("the project has no file yet")]
    NoPath,
    /// Another Dimo instance has the project open ([`crate::lock`]).
    #[error("the project is open in another Dimo instance{}", holder.as_ref().map(|h| format!(" (user {})", h.user)).unwrap_or_default())]
    InUse {
        /// Who holds the lock, if the lock file could be read.
        holder: Option<LockOwner>,
    },
}

impl ProjectError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn json(entry: impl Into<String>, error: &serde_json::Error) -> Self {
        Self::Json {
            entry: entry.into(),
            message: error.to_string(),
        }
    }
}
