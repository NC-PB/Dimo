//! The IPC facing types export to TypeScript with one shape for both directions (rule 6).
//!
//! Runs when the `specta` feature is on, which the desktop shell enables, so the workspace test
//! run covers it.
#![cfg(feature = "specta")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use dimo_core::{AuditEntry, Command, Patch, Project};

fn export() -> String {
    let types = specta::Types::default()
        .register::<Command>()
        .register::<Patch>()
        .register::<Project>()
        .register::<AuditEntry>();
    // The unified format fails if a type would need separate serialize and deserialize shapes.
    specta_typescript::Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap()
}

#[test]
fn domain_types_export_to_typescript() {
    let ts = export();
    for expected in [
        "export type Command =",
        "export type Change =",
        "export type CharId = string",
        "nominal: string | null",
        "export type Timestamp = string",
        "export type Color = string",
        "\"field\": \"nominal\"",
    ] {
        assert!(ts.contains(expected), "missing {expected:?} in:\n{ts}");
    }
    assert!(!ts.contains("_Serialize"), "phase split types in:\n{ts}");
}
