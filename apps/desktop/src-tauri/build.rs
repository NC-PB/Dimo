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
        "export_project",
        "propose_from_region",
        "read_callout_text",
        "preview_numbering",
        "zone_grid_form",
        "app_settings",
        "set_app_settings",
        "dev_startup",
        "dev_report_frame_times",
        "dev_log",
    ]);
    if let Err(err) = tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("tauri build failed: {err:#}");
    }
    embed_test_manifest();
}

/// Windows (MSVC) test executables get the Common Controls v6 manifest that tauri-build only
/// embeds into the app executable; without it they fail to start with
/// `STATUS_ENTRYPOINT_NOT_FOUND`. Applies to test targets only, so the app keeps its manifest.
fn embed_test_manifest() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let manifest = std::path::Path::new(&dir).join("windows-test-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
