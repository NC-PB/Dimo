//! `dimo-core` has no IO, no clock and no randomness (05 Architecture, rust.md, rule 11).
//!
//! Scans the library sources for APIs that touch the outside world. Tests may use them;
//! library code gets IDs, time and the user name from an `Environment`.

#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "std::thread",
    "std::io",
    "SystemTime",
    "Instant::now",
    "new_v4",
    "new_v7",
    "now_v7",
    "rand::",
    "println!",
    "eprintln!",
];

fn src_dir() -> PathBuf {
    // Resolved at run time so a shared target directory never points to a stale checkout.
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(manifest).join("src")
}

/// All `.rs` files below `dir`, recursively, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

#[test]
fn library_sources_do_no_io() {
    let files = rust_files(&src_dir());
    assert!(
        files.len() > 5,
        "no sources found in {}",
        src_dir().display()
    );
    let mut violations = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        // The unit test module at the end of a file may do anything.
        let library = text
            .split("#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or_default();
        for word in FORBIDDEN {
            if library.contains(word) {
                violations.push(format!("{}: {word}", path.display()));
            }
        }
    }
    assert!(violations.is_empty(), "IO in dimo-core:\n{violations:#?}");
}
