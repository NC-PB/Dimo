//! Recent projects of the user (T3.16, ADR 0007 File > Open): the project files opened or saved
//! last, most recent first, at most [`MAX_RECENT`].
//!
//! Stored as `recent-projects.json` in the app config directory, next to `settings.json`. Only
//! paths are kept, nothing of the projects themselves, and nothing leaves the computer
//! (rule 9). Rust adds an entry after a successful open or "save as"; the webview reads the list
//! and may remove entries or open one of them, but it can only name paths that are already on
//! the list (NFR-SEC-01: the webview never chooses a file by itself).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::fsio::{JsonStore, Stored};
use crate::ipc::{CommandError, RecentReason, tile_service};
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
}

impl Stored for RecentList {
    /// Drops entries that are not absolute paths and keeps at most [`MAX_RECENT`].
    fn repaired(mut self) -> Self {
        self.projects.retain(|p| p.is_absolute());
        self.projects.truncate(MAX_RECENT);
        self
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
    store: JsonStore<RecentList>,
}

impl RecentState {
    /// Loads the list from `path`; without a path the list lives for this run only.
    pub fn load(path: Option<PathBuf>) -> Self {
        Self {
            store: JsonStore::load(path),
        }
    }

    fn change<T>(&self, change: impl FnOnce(&mut RecentList) -> T) -> T {
        let (result, written) = self.store.update(change);
        if let (Err(error), Some(path)) = (written, self.store.path()) {
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
        self.store.read(RecentList::entries)
    }

    /// The stored path that the webview named, if it is on the list.
    fn find(&self, path: &str) -> Option<PathBuf> {
        self.store.read(|list| list.find(path).cloned())
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
    work: impl FnOnce(&RecentState) -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    crate::ipc::blocking(move || work(&app.state::<RecentState>())).await
}

/// The recent projects, most recent first.
#[tauri::command]
#[specta::specta]
pub async fn recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, CommandError> {
    blocking(app, |state| Ok(state.entries())).await
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
        Ok(state.entries())
    })
    .await
}

/// Opens a project of the recent list without a file dialog. `path` must be an entry of the
/// list exactly as [`recent_projects`] returned it; other paths are refused with `recent`
/// `not_listed`. `discard` as for `open_project`. A file that no longer exists is refused with
/// `recent` `missing` and stays on the list, so the user sees it greyed and can remove it.
#[tauri::command]
#[specta::specta]
pub async fn open_recent_project(
    app: AppHandle,
    path: String,
    discard: bool,
) -> Result<bool, CommandError> {
    let entry = blocking(app.clone(), move |state| {
        let entry = state.find(&path).ok_or_else(|| CommandError::Recent {
            reason: RecentReason::NotListed,
            message: format!("{path} is not a recent project"),
        })?;
        if entry.exists() {
            Ok(entry)
        } else {
            Err(CommandError::Recent {
                reason: RecentReason::Missing,
                message: format!("{} no longer exists", entry.display()),
            })
        }
    })
    .await?;
    if !discard && app.state::<SessionState>().has_unsaved_changes() {
        return Err(CommandError::UnsavedChanges);
    }
    let tiles = tile_service(&app.state::<TileState>())?;
    crate::project::open_path(app, tiles, entry, discard).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsio::{load_json, save_json};

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
        assert_eq!(load_json::<RecentList>(&store), RecentList::default());
        let mut list = RecentList::default();
        list.add(&temp_file(dir.path(), "a.dimo"));
        save_json(&store, &list).unwrap();
        assert_eq!(load_json::<RecentList>(&store), list);
        assert_eq!(
            std::fs::read_dir(store.parent().unwrap()).unwrap().count(),
            1
        );
        std::fs::write(&store, "{ nope").unwrap();
        assert_eq!(load_json::<RecentList>(&store), RecentList::default());
        // An absolute path on every platform (`/abs/b.dimo` is not absolute on Windows).
        let absolute = dir.path().join("b.dimo");
        std::fs::write(
            &store,
            serde_json::json!({ "projects": ["relative.dimo", absolute], "x": 1 }).to_string(),
        )
        .unwrap();
        let loaded = load_json::<RecentList>(&store);
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
        assert_eq!(load_json::<RecentList>(&store).projects, vec![file.clone()]);
        let key = state.entries()[0].path.clone();
        assert_eq!(state.find(&key), Some(file.clone()));
        state.remove(&file);
        assert_eq!(load_json::<RecentList>(&store), RecentList::default());
        let memory = RecentState::load(None);
        memory.add(&file);
        assert_eq!(memory.entries().len(), 1);
    }
}
