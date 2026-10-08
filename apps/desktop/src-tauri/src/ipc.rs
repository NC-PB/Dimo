//! IPC types and commands of the desktop shell (NFR-MNT-03).
//!
//! Every type here derives `specta::Type`, so its TypeScript twin is generated into
//! `apps/desktop/src/lib/ipc/bindings.ts`. Never write those types by hand in the frontend.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

/// Cargo profile the running binary was built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfile {
    /// Built without optimizations, with debug assertions (`tauri dev`).
    Debug,
    /// Optimized release build.
    Release,
}

impl BuildProfile {
    /// Profile of the current binary, derived from `debug_assertions`.
    pub const fn current() -> Self {
        if cfg!(debug_assertions) {
            Self::Debug
        } else {
            Self::Release
        }
    }
}

/// Static facts about the running app, shown in the UI and in bug reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct AppInfo {
    /// App version from the workspace manifest, for example `0.1.0`.
    pub version: String,
    /// Build profile of the running binary.
    pub build_profile: BuildProfile,
    /// Whether the PDFium library could be loaded. Without it no drawing can be opened.
    pub pdfium_available: bool,
}

impl AppInfo {
    /// Collects the facts about the running app.
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            build_profile: BuildProfile::current(),
            // TODO(T0.5): ask dimo-pdf whether PDFium loads once the binding exists.
            pdfium_available: false,
        }
    }
}

/// Identifier of a background job within one app session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct JobId(pub u32);

/// Progress of a long running job, emitted as a Tauri event (05 Architecture, IPC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "snake_case")]
pub struct JobProgress {
    /// Job the progress belongs to.
    pub id: JobId,
    /// Completed share of the work, from 0.0 to 1.0.
    pub fraction: f64,
    /// Short description of the current step, shown next to the progress bar.
    pub message: String,
}

/// Returns version, build profile and PDFium availability of the running app.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_workspace_version_and_profile() {
        let info = app_info();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        // Tests run with debug assertions unless built with --release.
        assert_eq!(
            info.build_profile == BuildProfile::Debug,
            cfg!(debug_assertions)
        );
        assert!(!info.pdfium_available, "PDFium is not wired up before T0.5");
    }
}
