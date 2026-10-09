//! Autosave of projects that were never saved (NFR-REL-01, D-28).
//!
//! A saved project journals next to its file ([`crate::journal`]). A new project has no file
//! yet, so the desktop app gives it an autosave base file in a folder of the app data
//! directory: `<dir>/<id>.dimo` with the initial state, `<id>.dimo.journal` with the changes and
//! `<id>.dimo.lock` while an instance has it open ([`crate::lock`]).
//!
//! Saving the project or closing it without saving deletes these files. If they are still
//! there at the next start, the app crashed: [`recover_unsaved`] restores the newest one that
//! no other running instance holds, as an unsaved project.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::journal;
use crate::lock::{LockOwner, ProjectLock};
use crate::project::{EXTENSION, OpenReport, ProjectError, ProjectSession};

/// The autosave base file for the project `id` in `dir`.
pub fn base_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.{EXTENSION}"))
}

/// Autosave base files in `dir`, most recently changed first. A missing folder has none.
pub fn candidates(dir: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(ProjectError::io(dir, e)),
    };
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in listing {
        let path = entry.map_err(|e| ProjectError::io(dir, e))?.path();
        if path.extension().and_then(|e| e.to_str()) != Some(EXTENSION) || !path.is_file() {
            continue;
        }
        let changed = [path.clone(), journal::journal_path(&path)]
            .iter()
            .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        found.push((changed, path));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(found.into_iter().map(|(_, path)| path).collect())
}

/// An unsaved project restored after a crash.
#[derive(Debug)]
pub struct RecoveredUnsaved {
    /// The project, without a location, with the journal open for further changes.
    pub session: ProjectSession,
    /// What was replayed.
    pub report: OpenReport,
    /// The lock on the autosave base file, held while the project is open.
    pub lock: ProjectLock,
}

/// Restores the newest unsaved project left in `dir` by a crash, or `None`.
///
/// Base files that another running instance holds are skipped. A base file without journaled
/// changes held nothing worth keeping and is deleted. A base file that cannot be read is moved
/// aside to `<file>.stale`, so it is not tried again; one that cannot be accessed is skipped.
pub fn recover_unsaved(
    dir: &Path,
    owner: &LockOwner,
) -> Result<Option<RecoveredUnsaved>, ProjectError> {
    for base in candidates(dir)? {
        let lock = match ProjectLock::acquire(&base, owner) {
            Ok(lock) => lock,
            Err(ProjectError::InUse { .. }) => continue,
            Err(e) => return Err(e),
        };
        let (mut session, report) = match ProjectSession::open_autosaved(&base) {
            Ok(opened) => opened,
            // Perhaps readable later, for example after a permission fix.
            Err(ProjectError::Io { .. }) => continue,
            Err(_) => {
                let stale = journal::stale_path(&base);
                fs::rename(&base, &stale).map_err(|e| ProjectError::io(&base, e))?;
                continue;
            }
        };
        if report.recovered_entries == 0 && report.discarded_journal.is_none() {
            session.discard_journal()?;
            continue;
        }
        return Ok(Some(RecoveredUnsaved {
            session,
            report,
            lock,
        }));
    }
    Ok(None)
}
