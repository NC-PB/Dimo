//! Fails if `unsafe` code appears anywhere but the Windows opener module (owner decision
//! 2026-10-15, `docs/dev/rust.md`). The workspace lint is `deny`, so this test is the second
//! line: an `#![allow(unsafe_code)]` elsewhere would not pass it.

use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to contain `unsafe`, relative to the repository root.
const ALLOWED: &str = "apps/desktop/src-tauri/src/opener/windows.rs";

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// True if a line of `source` uses `unsafe` as a keyword: `unsafe {`, `unsafe fn`,
/// `unsafe impl`, `unsafe trait`, `unsafe extern`. Line comments are skipped.
fn has_unsafe_keyword(source: &str) -> bool {
    source.lines().any(|line| {
        let code = line.split("//").next().unwrap_or("");
        let mut rest = code;
        let mut found = false;
        while let Some(i) = rest.find("unsafe") {
            let before = rest[..i].chars().next_back();
            let after = rest[i + 6..].trim_start();
            let boundary = !before.is_some_and(|c| c.is_alphanumeric() || c == '_');
            let keyword_use = ["{", "fn", "impl", "trait", "extern"]
                .iter()
                .any(|k| after.starts_with(k));
            if boundary && keyword_use {
                found = true;
            }
            rest = &rest[i + 6..];
        }
        found
    })
}

#[test]
fn unsafe_code_only_in_the_windows_opener() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    rust_files(&root.join("apps/desktop/src-tauri/src"), &mut files);
    assert!(files.len() > 20, "scan found only {} files", files.len());
    let mut offenders = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if relative.ends_with("crates/dimo-cli/tests/unsafe_guard.rs") || relative == ALLOWED {
            continue;
        }
        if has_unsafe_keyword(&fs::read_to_string(&file).unwrap()) {
            offenders.push(relative);
        }
    }
    assert!(
        offenders.is_empty(),
        "unsafe outside {ALLOWED}: {offenders:?}"
    );
}

#[test]
fn the_detector_finds_the_keyword_forms() {
    for yes in [
        "unsafe { x }",
        "pub unsafe fn f()",
        "unsafe impl Send for T {}",
    ] {
        assert!(has_unsafe_keyword(yes), "{yes}");
    }
    for no in [
        "// unsafe { x }",
        "let unsafe_code = 1;",
        "#![forbid(unsafe_code)]",
    ] {
        assert!(!has_unsafe_keyword(no), "{no}");
    }
}
