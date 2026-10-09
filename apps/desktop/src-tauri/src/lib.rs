//! Tauri shell of the Dimo desktop app. Business logic lives in the `dimo-*` crates (ADR 0001).
//!
//! Commands and events are registered once in [`specta_builder`]. The same builder drives the Tauri
//! invoke handler and the generated TypeScript bindings, so both always agree (NFR-MNT-03).

pub mod dev;
pub mod env;
pub mod ipc;
pub mod project;
pub mod session;
pub mod tiles;

use std::path::{Path, PathBuf};

use dimo_pdf::tiles::{MAX_ZOOM, MIN_ZOOM, TILE_SIZE};
use tauri::Manager;
use tauri_specta::{collect_commands, collect_events};

/// Header written above the generated bindings. The file is excluded from eslint and prettier.
const BINDINGS_HEADER: &str = "// Source: apps/desktop/src-tauri/src (ipc.rs, project.rs, session.rs, dev.rs). \
Regenerate with `cargo test -p dimo-desktop --test bindings` or `tauri dev`.";

/// Location of the committed TypeScript bindings, `apps/desktop/src/lib/ipc/bindings.ts`.
pub fn bindings_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/ipc/bindings.ts")
}

/// Collects every command, event, type and constant the frontend may use.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        // One TypeScript type per Rust type. The domain types serialize and deserialize alike
        // (checked by `dimo-core/tests/ipc_types.rs`), so phase split aliases only add noise.
        .disable_serde_phases()
        .commands(collect_commands![
            ipc::app_info,
            ipc::set_tile_interest,
            project::project_state,
            project::new_project,
            project::open_project,
            project::save_project,
            project::save_project_as,
            project::confirm_close,
            project::execute,
            project::undo,
            project::redo,
            dev::dev_startup,
            dev::dev_report_frame_times
        ])
        .events(collect_events![
            ipc::JobProgress,
            session::ProjectLoaded,
            session::ProjectPatched,
            session::ProjectStatusChanged,
            session::CloseRequested
        ])
        .typ::<ipc::TileAddress>()
        .constant("TILE_SIZE", TILE_SIZE)
        .constant("MIN_TILE_ZOOM", MIN_ZOOM)
        .constant("MAX_TILE_ZOOM", MAX_ZOOM)
}

/// Writes the TypeScript bindings of `builder` to `path`.
pub fn export_bindings(
    builder: &tauri_specta::Builder<tauri::Wry>,
    path: &Path,
) -> Result<(), specta_typescript::Error> {
    builder.export(
        specta_typescript::Typescript::default().header(BINDINGS_HEADER),
        path,
    )
}

/// Starts the desktop app. In debug builds the bindings are regenerated first.
#[allow(
    clippy::print_stderr,
    reason = "binding export warning in debug builds, no tracing subscriber yet"
)]
pub fn run() -> tauri::Result<()> {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    if let Err(err) = export_bindings(&builder, &bindings_path()) {
        // Not fatal: the app still works, and the bindings test fails until they are regenerated.
        eprintln!("failed to export TypeScript bindings: {err}");
    }

    let app = tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .register_asynchronous_uri_scheme_protocol(tiles::SCHEME, tiles::handle)
        .setup(move |app| {
            builder.mount_events(app);
            // Tile cache in the user cache directory, never in the project (07 Data model).
            let cache_dir = app.path().app_cache_dir().ok().map(|dir| dir.join("tiles"));
            app.manage(tiles::TileState::start(cache_dir));
            // Autosave of never saved projects in the app data directory (NFR-REL-01).
            let autosave_dir = app
                .path()
                .app_data_dir()
                .ok()
                .map(|dir| dir.join(AUTOSAVE_FOLDER));
            app.manage(project::SessionState::new(autosave_dir));
            recover_unsaved(app.handle());
            project::start_autosave_timer(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                project::on_close_requested(window, api);
            }
        })
        .build(tauri::generate_context!())?;
    app.run(|handle, event| match event {
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            project::on_exit_requested(handle, &api, code);
        }
        tauri::RunEvent::Exit => project::on_exit(handle),
        _ => {}
    });
    Ok(())
}

/// Folder in the app data directory for projects that were never saved.
pub const AUTOSAVE_FOLDER: &str = "autosave";

/// Restores an unsaved project left by a crash (NFR-REL-01). The frontend gets it, with the
/// recovery notice, from `project_state`.
fn recover_unsaved(app: &tauri::AppHandle) {
    if dev::dev_open_requested() {
        return;
    }
    let tiles = app.state::<tiles::TileState>();
    let Ok(service) = tiles.service() else {
        return;
    };
    let state = app.state::<project::SessionState>();
    if let Err(error) = state.lock().recover_unsaved(service) {
        tracing::warn!("cannot restore an unsaved project: {error}");
    }
}
