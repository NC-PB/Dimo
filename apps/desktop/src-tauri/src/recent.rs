//! Recent projects of the user (T3.16, ADR 0007 File > Open): the project files opened or saved
//! last, most recent first, at most [`MAX_RECENT`].
//!
//! Stored as `recent-projects.json` in the app config directory, next to `settings.json`. Only
//! paths are kept, nothing of the projects themselves, and nothing leaves the computer
//! (rule 9). Rust adds an entry after a successful open or "save as"; the webview reads the list
//! and may remove entries or open one of them, but it can only name paths that are already on
//! the list (NFR-SEC-01: the webview never chooses a file by itself).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::ipc::{CommandError, tile_service};
use crate::project::SessionState;
use crate::tiles::TileState;

/// File name of the list in the app config directory.
pub const RECENT_FILE: &str = "recent-projects.json";

/// Most entries kept.
pub const MAX_RECENT: usize = 10;

/// The stored list, most recent first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentList {
    /// Absolute paths of project files or folders.
    #[serde(default)]
    pub projects: Vec<PathBuf>,
}

impl RecentList {
    /// Puts `path` first, drops an older entry of the same file and keeps [`MAX_RECENT`].
    pub fn add(&mut self, path: &Path) {
        let path = absolute(path);
        self.projects.retain(|p| !same_entry(p, &path));
        self.projects.insert(0, path);
        self.projects.truncate(MAX_RECENT);
    }

    /// Removes `path`. Returns whether it was on the list.
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.projects.len();
        self.projects.retain(|p| p != path);
        self.projects.len() != before
    }

    /// The entry that is exactly `path` as the webview got it from [`RecentList::entries`].
    pub fn find(&self, path: &str) -> Option<&PathBuf> {
        self.projects.iter().find(|p| p.to_string_lossy() == path)
    }

    /// The entries as the frontend shows them, with whether each file still exists.
    pub fn entries(&self) -> Vec<RecentProject> {
        self.projects.iter().map(|p| RecentProject::of(p)).collect()
    }

    /// Reads the list. A missing or broken file gives an empty list (with a warning when
    /// broken); entries that are not absolute paths are dropped.
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!("cannot read {}: {error}", path.display());
                return Self::default();
            }
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(mut list) => {
                list.projects.retain(|p| p.is_absolute());
                list.projects.truncate(MAX_RECENT);
                list
            }
            Err(error) => {
                tracing::warn!("ignoring malformed {}: {error}", path.display());
                Self::default()
            }
        }
    }

    /// Writes the list through a temporary file and a rename, like the settings.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        text.push('\n');
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, path)
    }
}

/// `path` made absolute and, if it exists, with links resolved, so one file has one entry.
fn absolute(path: &Path) -> PathBuf {
    path.canonicalize()
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn same_entry(a: &Path, b: &Path) -> bool {
    a == b
        || a.canonicalize()
            .is_ok_and(|a| b.canonicalize().is_ok_and(|b| a == b))
}

/// One recent project as the File page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct RecentProject {
    /// Full path, also the key for opening or removing the entry.
    pub path: String,
    /// File name, for example `bracket.dimo`.
    pub name: String,
    /// Folder that holds the file.
    pub folder: String,
    /// False if the file or folder is gone (moved, renamed or on a drive that is not there).
    pub exists: bool,
}

impl RecentProject {
    fn of(path: &Path) -> Self {
        Self {
            path: path.to_string_lossy().into_owned(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            folder: path
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            exists: path.exists(),
        }
    }
}

/// The list of the running app and where it is stored.
#[derive(Debug)]
pub struct RecentState {
    path: Option<PathBuf>,
    list: Mutex<RecentList>,
}

impl RecentState {
    /// Loads the list from `path`; without a path the list lives for this run only.
    pub fn load(path: Option<PathBuf>) -> Self {
        let list = path.as_deref().map(RecentList::load).unwrap_or_default();
        Self {
            path,
            list: Mutex::new(list),
        }
    }

    fn change<T>(&self, change: impl FnOnce(&mut RecentList) -> T) -> T {
        let mut list = self.list.lock().unwrap_or_else(PoisonError::into_inner);
        let result = change(&mut list);
        if let Some(path) = &self.path
            && let Err(error) = list.save(path)
        {
            // The list is a convenience: a write error must not fail the open or save.
            tracing::warn!("cannot write {}: {error}", path.display());
        }
        result
    }

    /// Puts `path` first and writes the file.
    pub fn add(&self, path: &Path) {
        self.change(|list| list.add(path));
    }

    /// Removes `path` and writes the file.
    pub fn remove(&self, path: &Path) {
        self.change(|list| list.remove(path));
    }

    /// The entries for the frontend.
    pub fn entries(&self) -> Vec<RecentProject> {
        self.list
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entries()
    }

    /// The stored path that the webview named, if it is on the list.
    fn find(&self, path: &str) -> Option<PathBuf> {
        self.list
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .find(path)
            .cloned()
    }
}

/// Adds the project file of the open project to the list, after a successful open or save as.
pub fn remember(app: &AppHandle, path: &Path) {
    if let Some(state) = app.try_state::<RecentState>() {
        state.add(path);
    }
}

/// Runs `work` with the list on the blocking pool: checking whether files exist can wait for a
/// network drive, which must never block the main thread or the async runtime.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&RecentState) -> T + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<RecentState>()))
        .await
        .map_err(|e| CommandError::Io {
            message: e.to_string(),
        })
}

/// The recent projects, most recent first.
#[tauri::command]
#[specta::specta]
pub async fn recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, CommandError> {
    blocking(app, RecentState::entries).await
}

/// Removes one entry from the list (the file itself is not touched). Returns the new list.
#[tauri::command]
#[specta::specta]
pub async fn remove_recent_project(
    app: AppHandle,
    path: String,
) -> Result<Vec<RecentProject>, CommandError> {
    blocking(app, move |state| {
        if let Some(entry) = state.find(&path) {
            state.remove(&entry);
        }
        state.entries()
    })
    .await
}

/// Opens a project of the recent list without a file dialog. `path` must be an entry of the
/// list exactly as [`recent_projects`] returned it; other paths are refused. `discard` as for
/// `open_project`. A file that no longer exists is refused with `io` and stays on the list, so
/// the user sees it greyed and can remove it.
#[tauri::command]
#[specta::specta]
pub async fn open_recent_project(
    app: AppHandle,
    path: String,
    discard: bool,
) -> Result<bool, CommandError> {
    let entry = blocking(app.clone(), move |state| {
        let entry = state
            .find(&path)
            .ok_or_else(|| CommandError::InvalidArgument {
                message: "not a recent project".to_owned(),
            })?;
        if entry.exists() {
            Ok(entry)
        } else {
            Err(CommandError::Io {
                message: format!("{} no longer exists", entry.display()),
            })
        }
    })
    .await??;
    if !discard && app.state::<SessionState>().has_unsaved_changes() {
        return Err(CommandError::UnsavedChanges);
    }
    let tiles = tile_service(&app.state::<TileState>())?;
    crate::project::open_path(app, tiles, entry, discard).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"x").unwrap();
        path.canonicalize().unwrap()
    }

    #[test]
    fn newest_first_without_duplicates_and_at_most_ten() {
        let dir = tempfile::tempdir().unwrap();
        let mut list = RecentList::default();
        let files: Vec<PathBuf> = (0..12)
            .map(|i| temp_file(dir.path(), &format!("p{i}.dimo")))
            .collect();
        for file in &files {
            list.add(file);
        }
        assert_eq!(list.projects.len(), MAX_RECENT);
        assert_eq!(list.projects[0], files[11]);
        assert_eq!(list.projects[9], files[2]);
        // Opening an older one again moves it to the top, once.
        list.add(&files[5]);
        assert_eq!(list.projects[0], files[5]);
        assert_eq!(list.projects.len(), MAX_RECENT);
        assert_eq!(list.projects.iter().filter(|p| **p == files[5]).count(), 1);
    }

    #[test]
    fn missing_files_stay_listed_until_removed() {
        let dir = tempfile::tempdir().unwrap();
        let mut list = RecentList::default();
        let kept = temp_file(dir.path(), "kept.dimo");
        let gone = temp_file(dir.path(), "gone.dimo");
        list.add(&kept);
        list.add(&gone);
        std::fs::remove_file(&gone).unwrap();
        let entries = list.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "gone.dimo");
        assert!(!entries[0].exists);
        assert!(entries[1].exists);
        assert_eq!(
            entries[1].folder,
            kept.parent().unwrap().to_string_lossy().as_ref()
        );
        let key = entries[0].path.clone();
        assert_eq!(list.find(&key), Some(&gone));
        assert!(list.remove(&gone));
        assert!(!list.remove(&gone));
        assert_eq!(list.find(&key), None);
        assert_eq!(list.projects, vec![kept]);
    }

    #[test]
    fn only_listed_paths_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut list = RecentList::default();
        let file = temp_file(dir.path(), "a.dimo");
        list.add(&file);
        assert!(list.find("/etc/passwd").is_none());
        assert!(list.find("a.dimo").is_none());
        assert!(list.find(&file.to_string_lossy()).is_some());
    }

    #[test]
    fn file_round_trip_and_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("config").join(RECENT_FILE);
        assert_eq!(RecentList::load(&store), RecentList::default());
        let mut list = RecentList::default();
        list.add(&temp_file(dir.path(), "a.dimo"));
        list.save(&store).unwrap();
        assert_eq!(RecentList::load(&store), list);
        assert!(!store.with_extension("json.tmp").exists());
        std::fs::write(&store, "{ nope").unwrap();
        assert_eq!(RecentList::load(&store), RecentList::default());
        std::fs::write(
            &store,
            r#"{"projects": ["relative.dimo", "/abs/b.dimo"], "x": 1}"#,
        )
        .unwrap();
        let loaded = RecentList::load(&store);
        assert_eq!(loaded.projects.len(), 1);
        assert!(loaded.projects[0].is_absolute());
    }

    #[test]
    fn state_writes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join(RECENT_FILE);
        let state = RecentState::load(Some(store.clone()));
        let file = temp_file(dir.path(), "a.dimo");
        state.add(&file);
        assert_eq!(RecentList::load(&store).projects, vec![file.clone()]);
        let key = state.entries()[0].path.clone();
        assert_eq!(state.find(&key), Some(file.clone()));
        state.remove(&file);
        assert_eq!(RecentList::load(&store), RecentList::default());
        let memory = RecentState::load(None);
        memory.add(&file);
        assert_eq!(memory.entries().len(), 1);
    }
}
