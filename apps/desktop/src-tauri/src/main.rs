//! Entry point of the Dimo desktop app. See the library crate for the wiring.

// Hide the console window of release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> tauri::Result<()> {
    dimo_desktop::run()
}
