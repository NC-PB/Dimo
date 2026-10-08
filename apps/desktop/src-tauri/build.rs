//! Generates the Tauri context (config, capabilities, icons).
//!
//! App commands are declared in the app manifest, so each one gets an `allow-<command>`
//! permission and the webview can call only the commands its capability lists (NFR-SEC-01).

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "app_info",
        "open_document",
        "set_tile_interest",
    ]);
    if let Err(err) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("tauri build failed: {err:#}");
    }
}
