//! Guard against two Dimo instances editing the same project (T1.5).
//!
//! While a project is open, the app holds an exclusive operating system lock on a lock file
//! next to it: `part.dimo` gets `part.dimo.lock`. The file names the holder (user, process,
//! time) so a second instance can say who has the project open.
//!
//! # Stale locks
//!
//! The operating system releases the lock when the process ends, also after a crash. A lock
//! file that exists but is not locked was left by a process that died; [`ProjectLock::acquire`]
//! takes it over. No time limits or process ID checks are needed.
//!
//! # Limits
//!
//! The lock is advisory between Dimo instances, it does not stop other programs. On file
//! systems without lock support (some network shares) acquiring succeeds without a guard.
//! On Windows the lock also blocks reading the file, so the holder is unknown there.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use dimo_core::Timestamp;
use serde::{Deserialize, Serialize};

use crate::project::ProjectError;

/// Who holds a project lock, written into the lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LockOwner {
    /// Audit user name of the holder (D-27).
    pub user: String,
    /// Process ID of the holder.
    pub pid: u32,
    /// When the lock was taken.
    pub since: Timestamp,
}

/// The lock file of the project at `project_path`: the same name plus `.lock`.
pub fn lock_path(project_path: &Path) -> PathBuf {
    let mut name = project_path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".lock");
    project_path.with_file_name(name)
}

/// An exclusive lock on a project. Dropping it deletes the lock file and releases the lock.
#[derive(Debug)]
pub struct ProjectLock {
    path: PathBuf,
    file: File,
}

/// Attempts to take over a lock file that a releasing holder deletes at the same moment.
const ATTEMPTS: usize = 3;

impl ProjectLock {
    /// Locks the project at `project_path` for `owner`.
    ///
    /// Fails with [`ProjectError::InUse`] if another instance holds the lock. A lock file left
    /// by a crashed instance is taken over (see the module docs).
    pub fn acquire(project_path: &Path, owner: &LockOwner) -> Result<Self, ProjectError> {
        let path = lock_path(project_path);
        let io = |e| ProjectError::io(&path, e);
        for _ in 0..ATTEMPTS {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .map_err(io)?;
            match file.try_lock() {
                Ok(()) => {}
                Err(TryLockError::WouldBlock) => {
                    return Err(ProjectError::InUse {
                        holder: read_owner(&mut file),
                    });
                }
                // No lock support on this file system: continue without a guard.
                Err(TryLockError::Error(e)) if e.kind() == ErrorKind::Unsupported => {}
                Err(TryLockError::Error(e)) => return Err(io(e)),
            }
            if !still_at(&file, &path) {
                // The previous holder deleted the file between our open and lock.
                continue;
            }
            let mut text = serde_json::to_vec(owner).map_err(|e| ProjectError::json("lock", &e))?;
            text.push(b'\n');
            file.set_len(0)
                .and_then(|()| file.seek(SeekFrom::Start(0)))
                .and_then(|_| file.write_all(&text))
                .and_then(|()| file.sync_data())
                .map_err(io)?;
            return Ok(Self { path, file });
        }
        Err(ProjectError::InUse { holder: None })
    }

    /// The lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The holder of the lock on `project_path`, if a lock file exists and is readable.
    pub fn holder(project_path: &Path) -> Option<LockOwner> {
        let mut file = File::open(lock_path(project_path)).ok()?;
        read_owner(&mut file)
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        // Delete while still locked, so nobody locks a file that is about to disappear. The
        // lock is released when `file` closes right after.
        let _ = fs::remove_file(&self.path);
        let _ = self.file.unlock();
    }
}

fn read_owner(file: &mut File) -> Option<LockOwner> {
    let mut text = String::new();
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_to_string(&mut text).ok()?;
    serde_json::from_str(text.trim()).ok()
}

/// True if `path` still names the open `file`.
#[cfg(unix)]
fn still_at(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    match (file.metadata(), fs::metadata(path)) {
        (Ok(open), Ok(named)) => open.dev() == named.dev() && open.ino() == named.ino(),
        _ => false,
    }
}

/// Windows keeps a deleted file that is still open in a "delete pending" state, where opening
/// it again fails, so a lock can never be taken on a file that is about to disappear.
#[cfg(not(unix))]
fn still_at(_file: &File, path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(user: &str) -> LockOwner {
        LockOwner {
            user: user.to_owned(),
            pid: 42,
            since: Timestamp::parse("2026-10-09T10:00:00Z").unwrap(),
        }
    }

    #[test]
    fn lock_file_sits_next_to_the_project() {
        assert_eq!(
            lock_path(Path::new("/a/part.dimo")),
            PathBuf::from("/a/part.dimo.lock")
        );
    }

    #[test]
    fn second_lock_fails_and_names_the_holder() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("part.dimo");
        let lock = ProjectLock::acquire(&project, &owner("anna")).unwrap();
        assert!(lock.path().exists());
        let second = ProjectLock::acquire(&project, &owner("ben"));
        match second {
            Err(ProjectError::InUse { holder }) => {
                if cfg!(unix) {
                    assert_eq!(holder, Some(owner("anna")));
                }
            }
            other => panic!("expected InUse, got {other:?}"),
        }
        drop(lock);
        assert!(!lock_path(&project).exists());
        let again = ProjectLock::acquire(&project, &owner("ben")).unwrap();
        if cfg!(unix) {
            assert_eq!(ProjectLock::holder(&project), Some(owner("ben")));
        }
        drop(again);
    }

    #[test]
    fn stale_lock_file_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("part.dimo");
        // What a crashed instance leaves: a lock file nobody holds.
        fs::write(lock_path(&project), b"{\"user\":\"gone\"}\n").unwrap();
        let lock = ProjectLock::acquire(&project, &owner("anna")).unwrap();
        if cfg!(unix) {
            assert_eq!(ProjectLock::holder(&project), Some(owner("anna")));
        }
        drop(lock);
        assert!(!lock_path(&project).exists());
    }
}
