//! Tauri shell of the Dimo desktop app. Business logic lives in the `dimo-*` crates (ADR 0001).
//!
//! Commands and events are registered once in [`specta_builder`]. The same builder drives the Tauri
//! invoke handler and the generated TypeScript bindings, so both always agree (NFR-MNT-03).

pub mod ipc;

use std::path::{Path, PathBuf};

use tauri_specta::{collect_commands, collect_events};

/// Header written above the generated bindings. The file is excluded from eslint and prettier.
const BINDINGS_HEADER: &str = "// Source: apps/desktop/src-tauri/src/ipc.rs. \
Regenerate with `cargo test -p dimo-desktop --test bindings` or `tauri dev`.";

/// Location of the committed TypeScript bindings, `apps/desktop/src/lib/ipc/bindings.ts`.
pub fn bindings_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/ipc/bindings.ts")
}

/// Collects every command and event the frontend may use.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![ipc::app_info])
        .events(collect_events![ipc::JobProgress])
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

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
}
