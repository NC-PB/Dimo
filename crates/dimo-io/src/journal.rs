//! Autosave journal and crash recovery (NFR-REL-01, D-28).
//!
//! While a saved project is edited, every audit entry is appended to a journal file next to
//! it (`part.dimo` gets `part.dimo.journal`). The journal is written after each command batch
//! and at the latest every [`AUTOSAVE_INTERVAL`]. Saving the project writes everything into
//! the project file and deletes the journal (compaction). After a crash the journal is still
//! there, and opening the project replays it.
//!
//! # Format
//!
//! JSON Lines. The first line is a header with the format name, the schema version and the
//! SHA-256 of the project it continues ([`base_hash`]). Every further line is one
//! [`AuditEntry`]. Replay applies each entry's `changes` with [`Project::apply_change`];
//! re-running the commands would create new IDs.
//!
//! Each append is one write of complete lines followed by `fsync`. A crash during a write can
//! only leave an incomplete last line, which recovery drops. A journal whose header does not
//! match the project (another base, another schema version) or whose entries do not apply is
//! never replayed: it is moved aside to `<journal>.stale` and reported.

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dimo_core::{AuditEntry, Project, Sha256Hex};
use serde::{Deserialize, Serialize};

use crate::project::{ProjectError, SCHEMA_VERSION, sha256};

/// Longest time between journal writes (D-28). Also written after each command batch.
pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(30);

/// The `format` value in the journal header.
pub const JOURNAL_FORMAT: &str = "dimo-journal";

/// The journal of the project at `project_path`: the same name plus `.journal`, in the same
/// folder. Works for ZIP files and project folders.
pub fn journal_path(project_path: &Path) -> PathBuf {
    let mut name = project_path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".journal");
    project_path.with_file_name(name)
}

/// Where a journal that cannot be replayed is moved.
pub fn stale_path(journal: &Path) -> PathBuf {
    let mut name = journal.as_os_str().to_os_string();
    name.push(".stale");
    PathBuf::from(name)
}

/// Identity of the project state a journal continues: SHA-256 of its compact JSON.
pub fn base_hash(project: &Project) -> Sha256Hex {
    // Serializing the project cannot fail: it has no maps with non string keys.
    sha256(&serde_json::to_vec(project).unwrap_or_default())
}

/// First line of a journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Header {
    format: String,
    schema_version: u32,
    base_sha256: Sha256Hex,
}

/// An open journal, appending audit entries.
#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    file: File,
    /// Length of the valid content, to cut off a failed append.
    len: u64,
}

impl Journal {
    /// Starts a journal for the saved state `base`, replacing any existing file atomically.
    pub fn create(path: &Path, base: &Project) -> Result<Self, ProjectError> {
        let header = Header {
            format: JOURNAL_FORMAT.to_owned(),
            schema_version: SCHEMA_VERSION,
            base_sha256: base_hash(base),
        };
        let mut line =
            serde_json::to_vec(&header).map_err(|e| ProjectError::json("journal", &e))?;
        line.push(b'\n');
        crate::project::write_atomic(path, &line)?;
        Self::open(path, line.len() as u64)
    }

    /// Opens an existing journal for appending after its first `valid_len` bytes. Anything
    /// after them (an incomplete last line) is cut off.
    fn open(path: &Path, valid_len: u64) -> Result<Self, ProjectError> {
        let io = |e| ProjectError::io(path, e);
        let file = OpenOptions::new().write(true).open(path).map_err(io)?;
        file.set_len(valid_len).map_err(io)?;
        file.sync_all().map_err(io)?;
        let mut journal = Self {
            path: path.to_owned(),
            file,
            len: valid_len,
        };
        journal.seek_end()?;
        Ok(journal)
    }

    fn seek_end(&mut self) -> Result<(), ProjectError> {
        use std::io::{Seek, SeekFrom};
        self.file
            .seek(SeekFrom::Start(self.len))
            .map_err(|e| ProjectError::io(&self.path, e))?;
        Ok(())
    }

    /// The journal file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends entries as complete lines and flushes them to disk. On error the file is cut
    /// back to its previous length, so a retry never leaves a broken line in the middle.
    pub fn append(&mut self, entries: &[AuditEntry]) -> Result<(), ProjectError> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut lines = Vec::new();
        for entry in entries {
            serde_json::to_writer(&mut lines, entry)
                .map_err(|e| ProjectError::json("journal", &e))?;
            lines.push(b'\n');
        }
        let written = self
            .file
            .write_all(&lines)
            .and_then(|()| self.file.sync_data());
        match written {
            Ok(()) => {
                self.len += lines.len() as u64;
                Ok(())
            }
            Err(error) => {
                // Best effort: if even this fails, recovery drops at most an incomplete line.
                let _ = self.file.set_len(self.len);
                let _ = self.seek_end();
                Err(ProjectError::io(&self.path, error))
            }
        }
    }

    /// Closes and deletes the journal (after a save, or when changes are discarded).
    pub fn remove(self) -> Result<(), ProjectError> {
        let Self { path, file, .. } = self;
        drop(file);
        remove_file(&path)
    }
}

/// Deletes a file, ignoring that it does not exist.
pub(crate) fn remove_file(path: &Path) -> Result<(), ProjectError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ProjectError::io(path, e)),
    }
}

/// Result of [`recover`].
#[derive(Debug)]
pub enum Recovery {
    /// There is no journal, or it holds no complete entry.
    Nothing,
    /// Entries were replayed onto the project. `journal` continues the file.
    Replayed {
        /// The replayed entries, oldest first, for the audit log.
        entries: Vec<AuditEntry>,
        /// True if an incomplete last line (a write cut by the crash) was dropped.
        dropped_incomplete_line: bool,
        /// The journal, open for further appends.
        journal: Journal,
    },
    /// The journal does not fit the project and was moved aside. The project is unchanged.
    Discarded {
        /// Why it was not replayed.
        reason: String,
        /// Where the journal is now.
        moved_to: PathBuf,
    },
}

/// Replays the journal at `path` onto `project`, the state loaded from the project file.
///
/// Either every complete entry is applied or none. Only file system errors are returned as
/// errors; a journal that does not fit is moved aside ([`Recovery::Discarded`]).
pub fn recover(path: &Path, project: &mut Project) -> Result<Recovery, ProjectError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Recovery::Nothing),
        Err(e) => return Err(ProjectError::io(path, e)),
    };
    // Only lines ending in a newline were written completely.
    let complete = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let dropped_incomplete_line = complete < bytes.len();
    let mut lines = bytes[..complete]
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty());

    let Some(header) = lines.next() else {
        // The crash hit while the header was written: nothing was journaled yet.
        remove_file(path)?;
        return Ok(Recovery::Nothing);
    };
    let discard = |reason: String| -> Result<Recovery, ProjectError> {
        let moved_to = stale_path(path);
        fs::rename(path, &moved_to).map_err(|e| ProjectError::io(path, e))?;
        Ok(Recovery::Discarded { reason, moved_to })
    };
    let header: Header = match serde_json::from_slice(header) {
        Ok(header) => header,
        Err(e) => return discard(format!("unreadable header: {e}")),
    };
    if header.format != JOURNAL_FORMAT {
        return discard(format!("format {:?} is no journal", header.format));
    }
    if header.schema_version != SCHEMA_VERSION {
        return discard(format!(
            "written for schema version {}, this version uses {SCHEMA_VERSION}",
            header.schema_version
        ));
    }
    if header.base_sha256 != base_hash(project) {
        return discard("written for another state of the project".into());
    }

    let mut entries = Vec::new();
    for (index, line) in lines.enumerate() {
        match serde_json::from_slice::<AuditEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => return discard(format!("entry {} is unreadable: {e}", index + 1)),
        }
    }
    if entries.is_empty() {
        let journal = Journal::open(path, complete as u64)?;
        journal.remove()?;
        return Ok(Recovery::Nothing);
    }

    let mut replayed = project.clone();
    for (index, entry) in entries.iter().enumerate() {
        for change in &entry.changes {
            if let Err(e) = replayed.apply_change(change) {
                return discard(format!("entry {} does not apply: {e}", index + 1));
            }
        }
    }
    let journal = Journal::open(path, complete as u64)?;
    *project = replayed;
    Ok(Recovery::Replayed {
        entries,
        dropped_incomplete_line,
        journal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_sits_next_to_the_project() {
        assert_eq!(
            journal_path(Path::new("/a/b/part.dimo")),
            PathBuf::from("/a/b/part.dimo.journal")
        );
        assert_eq!(
            journal_path(Path::new("part.dimo")),
            PathBuf::from("part.dimo.journal")
        );
        assert_eq!(
            stale_path(Path::new("/a/part.dimo.journal")),
            PathBuf::from("/a/part.dimo.journal.stale")
        );
    }
}
