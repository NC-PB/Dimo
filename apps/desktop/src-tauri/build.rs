//! Generates the Tauri context (config, capabilities, icons).
//!
//! App commands are declared in the app manifest, so each one gets an `allow-<command>`
//! permission and the webview can call only the commands its capability lists (NFR-SEC-01).

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "app_info",
        "set_tile_interest",
        "project_state",
        "new_project",
        "open_project",
        "save_project",
        "save_project_as",
        "confirm_close",
        "execute",
        "undo",
        "redo",
        "dev_startup",
        "dev_report_frame_times",
        "dev_log",
    ]);
    if let Err(err) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("tauri build failed: {err:#}");
    }
}
