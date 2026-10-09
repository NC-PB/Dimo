//! Helpers shared by the table tests.
#![allow(dead_code, reason = "each test crate uses a different subset")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use std::path::PathBuf;
use std::str::FromStr;

use rust_decimal::Decimal;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn tables_dir() -> PathBuf {
    repo_root().join("data/tolerances")
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn d(text: &str) -> Decimal {
    Decimal::from_str(text).unwrap()
}

/// File names in `data/tolerances` with the given suffix, sorted.
pub fn files_with_suffix(suffix: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(tables_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(suffix))
        .collect();
    names.sort();
    names
}
