//! R.11: the app version is the same in the Cargo workspace, `tauri.conf.json` and
//! `package.json`. The About pane, the File page and the installers each read one of them.

// Test code (rust.md): unwrap is fine.
#![allow(clippy::unwrap_used)]

use std::path::Path;

fn version_in(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["version"].as_str().unwrap().to_owned()
}

#[test]
fn workspace_tauri_and_package_versions_agree() {
    let cargo = env!("CARGO_PKG_VERSION");
    assert_eq!(version_in("tauri.conf.json"), cargo, "tauri.conf.json");
    assert_eq!(
        version_in("../package.json"),
        cargo,
        "apps/desktop/package.json"
    );
}
