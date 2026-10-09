//! `dimo-core` has no IO, no clock and no randomness (05 Architecture, rust.md, rule 11).
//!
//! Scans the library sources for APIs that touch the outside world. Tests may use them;
//! library code gets IDs, time and the user name from an `Environment`.

use std::fs;
use std::path::PathBuf;

const FORBIDDEN: &[&str] = &[
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "std::thread",
    "std::io::stdin",
    "SystemTime",
    "Instant::now",
    "new_v4",
    "new_v7",
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

#[test]
fn library_sources_do_no_io() {
    let mut violations = Vec::new();
    let mut files = 0;
    for entry in fs::read_dir(src_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        files += 1;
        let text = fs::read_to_string(&path).unwrap();
        // Unit test modules at the end of a file may do anything.
        let library = text.split("#[cfg(test)]").next().unwrap_or_default();
        for word in FORBIDDEN {
            if library.contains(word) {
                violations.push(format!("{}: {word}", path.display()));
            }
        }
    }
    assert!(files > 5, "no sources found in {}", src_dir().display());
    assert!(violations.is_empty(), "IO in dimo-core:\n{violations:#?}");
}
