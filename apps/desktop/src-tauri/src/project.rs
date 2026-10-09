//! Project commands and events of the main window (T1.5). The logic lives in
//! [`crate::session`] and the `dimo-*` crates; this module only adds dialogs, threads and events.
//!
//! File dialogs run in Rust (`rfd`), so the webview never names a path (NFR-SEC-01). Every
//! command that touches the session runs on the blocking pool and emits its events while it
//! still holds the session, so events arrive in the order they happened. The main thread
//! (window and quit events) never waits for the session: it reads a flag that every unlock
//! updates.

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use dimo_core::Command;
use dimo_io::journal::AUTOSAVE_INTERVAL;
use dimo_io::project::EXTENSION;
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_specta::Event;

use crate::ipc::{CommandError, tile_service};
use crate::session::{AppSession, CloseRequested, ProjectLoaded, ProjectPatched};
use crate::tiles::TileState;

/// Label of the main window, as in `tauri.conf.json`.
pub const MAIN_WINDOW: &str = "main";

/// The session of the app, shared by commands, the autosave timer and window events.
pub struct SessionState {
    session: Mutex<AppSession>,
    /// [`AppSession::has_unsaved_changes`] as of the last unlock, for the main thread.
    unsaved: AtomicBool,
}

impl SessionState {
    /// A session without a project. New projects journal in `autosave_dir`.
    pub fn new(autosave_dir: Option<PathBuf>) -> Self {
        Self {
            session: Mutex::new(AppSession::new(autosave_dir)),
            unsaved: AtomicBool::new(false),
        }
    }

    /// The session. A panic in another thread does not make the project unreachable. Blocks
    /// while a command runs: never call it on the main thread.
    pub fn lock(&self) -> SessionGuard<'_> {
        SessionGuard {
            guard: self.session.lock().unwrap_or_else(PoisonError::into_inner),
            unsaved: &self.unsaved,
        }
    }

    /// Whether the project had unsaved changes when the session was last unlocked. Never
    /// blocks.
    pub fn has_unsaved_changes(&self) -> bool {
        self.unsaved.load(Ordering::SeqCst)
    }
}

/// The locked session; unlocking updates [`SessionState::has_unsaved_changes`].
pub struct SessionGuard<'a> {
    guard: MutexGuard<'a, AppSession>,
    unsaved: &'a AtomicBool,
}

impl Deref for SessionGuard<'_> {
    type Target = AppSession;

    fn deref(&self) -> &AppSession {
        &self.guard
    }
}

impl DerefMut for SessionGuard<'_> {
    fn deref_mut(&mut self) -> &mut AppSession {
        &mut self.guard
    }
}

impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        self.unsaved
            .store(self.guard.has_unsaved_changes(), Ordering::SeqCst);
    }
}

/// Emits an event; a failure is logged, the command result does not depend on it.
pub(crate) fn emit<R: Runtime, E: Event + Serialize + Clone>(app: &AppHandle<R>, event: &E) {
    if let Err(error) = event.emit(app) {
        tracing::warn!("cannot emit event: {error}");
    }
}

/// Runs `work` on the blocking pool with the session locked.
pub(crate) async fn with_session<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&AppHandle, &mut AppSession) -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<SessionState>();
        let mut session = state.lock();
        work(&app, &mut session)
    })
    .await
    .map_err(|e| CommandError::Io {
        message: e.to_string(),
    })?
}

/// Refuses early, before a file dialog, when unsaved changes would be dropped.
fn check_unsaved(state: &SessionState, discard: bool) -> Result<(), CommandError> {
    if !discard && state.has_unsaved_changes() {
        return Err(CommandError::UnsavedChanges);
    }
    Ok(())
}

/// The current project, for a frontend that just started listening to the project events.
/// Also hands out, once, the notice of a project restored at startup.
#[tauri::command]
#[specta::specta]
pub async fn project_state(app: AppHandle) -> Result<ProjectLoaded, CommandError> {
    with_session(app, |_, session| Ok(session.state())).await
}

/// Lets the user pick a PDF and creates a new, unsaved project from it (FR-DOC-07).
/// Returns `false` if the dialog was cancelled. Unsaved changes of the open project are
/// refused with `unsaved_changes` unless `discard` is set; ask the user first.
#[tauri::command]
#[specta::specta]
pub async fn new_project(
    app: AppHandle,
    window: WebviewWindow,
    discard: bool,
) -> Result<bool, CommandError> {
    check_unsaved(&app.state::<SessionState>(), discard)?;
    let tiles = tile_service(&app.state::<TileState>())?;
    let picked = rfd::AsyncFileDialog::new()
        .add_filter("PDF", &["pdf", "PDF"])
        .set_parent(&window)
        .pick_file()
        .await;
    let Some(file) = picked else {
        return Ok(false);
    };
    let path = file.path().to_path_buf();
    with_session(app, move |app, session| {
        let loaded = session.create_from_drawing(&tiles, &path, discard)?;
        emit(app, &loaded);
        Ok(true)
    })
    .await
}

/// Lets the user pick a project file and opens it. Returns `false` if the dialog was
/// cancelled. `discard` as for [`new_project`].
#[tauri::command]
#[specta::specta]
pub async fn open_project(
    app: AppHandle,
    window: WebviewWindow,
    discard: bool,
) -> Result<bool, CommandError> {
    check_unsaved(&app.state::<SessionState>(), discard)?;
    let tiles = tile_service(&app.state::<TileState>())?;
    let picked = rfd::AsyncFileDialog::new()
        .add_filter("Dimo", &[EXTENSION])
        .set_parent(&window)
        .pick_file()
        .await;
    let Some(file) = picked else {
        return Ok(false);
    };
    let path = file.path().to_path_buf();
    with_session(app, move |app, session| {
        let before = session.session();
        match session.open_file(&tiles, &path, discard) {
            Ok(loaded) => {
                emit(app, &loaded);
                Ok(true)
            }
            Err(error) => {
                // Reopening the open file closed it first; tell the frontend.
                if session.session() != before {
                    emit(app, &session.state());
                }
                Err(error)
            }
        }
    })
    .await
}

/// Saves the project. A project without a file asks for one like [`save_project_as`].
/// Returns `false` if that dialog was cancelled.
#[tauri::command]
#[specta::specta]
pub async fn save_project(app: AppHandle, window: WebviewWindow) -> Result<bool, CommandError> {
    let has_file = with_session(app.clone(), |_, session| {
        if session.is_open() {
            Ok(session.has_file())
        } else {
            Err(CommandError::NoProject)
        }
    })
    .await?;
    if !has_file {
        return save_project_as(app, window).await;
    }
    with_session(app, |app, session| {
        let status = session.save()?;
        emit(app, &status);
        Ok(true)
    })
    .await
}

/// Lets the user choose a file and saves the project there; it becomes the project file.
/// Returns `false` if the dialog was cancelled.
#[tauri::command]
#[specta::specta]
pub async fn save_project_as(app: AppHandle, window: WebviewWindow) -> Result<bool, CommandError> {
    let suggested = with_session(app.clone(), |_, session| {
        if session.is_open() {
            Ok(session.suggested_file_name())
        } else {
            Err(CommandError::NoProject)
        }
    })
    .await?;
    let mut path = if let Some(path) = crate::dev::dev_save_as() {
        path
    } else {
        let picked = rfd::AsyncFileDialog::new()
            .add_filter("Dimo", &[EXTENSION])
            .set_file_name(suggested)
            .set_parent(&window)
            .save_file()
            .await;
        let Some(file) = picked else {
            return Ok(false);
        };
        file.path().to_path_buf()
    };
    if path.extension().is_none() {
        path.set_extension(EXTENSION);
    }
    with_session(app, move |app, session| {
        let status = session.save_as(&path)?;
        emit(app, &status);
        Ok(true)
    })
    .await
}

/// Answer to `close-requested`: closes the project, dropping unsaved changes if `discard` is
/// set, then closes the window. Without `discard`, unsaved changes are refused.
#[tauri::command]
#[specta::specta]
pub async fn confirm_close(
    app: AppHandle,
    window: WebviewWindow,
    discard: bool,
) -> Result<(), CommandError> {
    let tiles = tile_service(&app.state::<TileState>()).ok();
    with_session(app, move |app, session| {
        let loaded = session.close(tiles.as_deref(), discard)?;
        emit(app, &loaded);
        Ok(())
    })
    .await?;
    window.close().map_err(|e| CommandError::Io {
        message: e.to_string(),
    })
}

/// Executes a document command as one undo step (05 Architecture, principle 1). The result is
/// also sent as `project-patched`; the store applies whichever arrives first (same revision).
/// A command that changes nothing returns an empty patch with the unchanged revision.
#[tauri::command]
#[specta::specta]
pub async fn execute(app: AppHandle, command: Command) -> Result<ProjectPatched, CommandError> {
    with_session(app, move |app, session| {
        Ok(emit_patched(app, session.execute(command)?))
    })
    .await
}

/// Undoes the last command (NFR-REL-02). Also sent as `project-patched`.
#[tauri::command]
#[specta::specta]
pub async fn undo(app: AppHandle) -> Result<ProjectPatched, CommandError> {
    with_session(app, |app, session| Ok(emit_patched(app, session.undo()?))).await
}

/// Applies the last undone command again. Also sent as `project-patched`.
#[tauri::command]
#[specta::specta]
pub async fn redo(app: AppHandle) -> Result<ProjectPatched, CommandError> {
    with_session(app, |app, session| Ok(emit_patched(app, session.redo()?))).await
}

pub(crate) fn emit_patched(app: &AppHandle, patched: ProjectPatched) -> ProjectPatched {
    if !patched.patch.is_empty() {
        emit(app, &patched);
    }
    patched
}

/// Starts the autosave timer (D-28): every [`AUTOSAVE_INTERVAL`] the journal is written, which
/// retries writes that failed after a command.
pub fn start_autosave_timer(app: AppHandle) {
    let spawned = std::thread::Builder::new()
        .name("dimo-autosave".into())
        .spawn(move || {
            loop {
                std::thread::sleep(AUTOSAVE_INTERVAL);
                let state = app.state::<SessionState>();
                let mut session = state.lock();
                if let Some(status) = session.autosave_tick() {
                    emit(&app, &status);
                }
            }
        });
    if let Err(error) = spawned {
        tracing::warn!("cannot start the autosave timer: {error}");
    }
}

/// Window close (D-28, NFR-REL-01): with unsaved changes the close is stopped and the frontend
/// asks the user, who answers through [`confirm_close`].
pub fn on_close_requested(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    let state = window.state::<SessionState>();
    if state.has_unsaved_changes() {
        api.prevent_close();
        emit(window.app_handle(), &CloseRequested);
    }
}

/// Quit from the menu or the dock (macOS): handled like closing the window.
pub fn on_exit_requested(app: &AppHandle, api: &tauri::ExitRequestApi, code: Option<i32>) {
    let state = app.state::<SessionState>();
    if code.is_none()
        && state.has_unsaved_changes()
        && app.get_webview_window(MAIN_WINDOW).is_some()
    {
        api.prevent_exit();
        emit(app, &CloseRequested);
    }
}

/// App exit: releases the project. Unsaved changes the user did not discard stay in the
/// journal for recovery.
pub fn on_exit(app: &AppHandle) {
    let tiles = app
        .try_state::<TileState>()
        .and_then(|t| tile_service(&t).ok());
    if let Some(state) = app.try_state::<SessionState>() {
        state.lock().shutdown(tiles.as_deref());
    }
}
