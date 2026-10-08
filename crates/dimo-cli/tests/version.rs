//! `dimo --version` prints the package version (T0.1).

use std::process::Command;

#[test]
fn version_flag_prints_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_dimo-cli"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.trim(), format!("dimo {}", env!("CARGO_PKG_VERSION")));
}
