//! Tauri shell of the Dimo desktop app. Business logic lives in the `dimo-*` crates (ADR 0001).

// Hide the console window of release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
