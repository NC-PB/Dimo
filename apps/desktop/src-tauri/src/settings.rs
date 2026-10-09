//! Settings of the user, independent of any project (T1.9): theme (D-51), UI language
//! (FR-SET-04), audit user name (D-27) and the last used export options (D-32, D-33).
//!
//! Stored as `settings.json` in the app config directory. Only Rust reads and writes the file;
//! the webview has no file system access (NFR-SEC-01). Missing fields take their defaults and
//! unknown fields are ignored, so older and newer files still load; a file that cannot be read is ignored with a
//! warning and replaced at the next change.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::env::os_user_name;
use crate::ipc::CommandError;
use crate::project::{MAIN_WINDOW, SessionState};

/// File name of the settings in the app config directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// Color theme of the window (D-51).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light or dark as the operating system says.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    /// The window theme for Tauri; `None` follows the operating system.
    pub fn window_theme(self) -> Option<tauri::Theme> {
        match self {
            Self::System => None,
            Self::Light => Some(tauri::Theme::Light),
            Self::Dark => Some(tauri::Theme::Dark),
        }
    }
}

/// Language of column headers and fixed texts in exports, chosen per export (D-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ReportLanguage {
    /// English.
    #[default]
    En,
    /// German.
    De,
}

impl From<ReportLanguage> for dimo_io::export::Language {
    fn from(language: ReportLanguage) -> Self {
        match language {
            ReportLanguage::En => Self::English,
            ReportLanguage::De => Self::German,
        }
    }
}

/// How balloons are written into the ballooned PDF (D-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PdfBalloons {
    /// Vector graphics in the page content (default).
    #[default]
    PageContent,
    /// One annotation per balloon, which PDF viewers can hide.
    Annotations,
}

/// The export options used last, offered again in the export view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ExportPreferences {
    /// Header language of the characteristic list.
    pub language: ReportLanguage,
    /// Balloons as page content or annotations.
    pub pdf_balloons: PdfBalloons,
}

/// Settings of the user.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct AppSettings {
    /// Color theme.
    pub theme: Theme,
    /// UI language tag such as `en` or `de`; `null` follows the operating system.
    pub locale: Option<String>,
    /// Name recorded in the audit log (D-27); empty uses the operating system user name.
    pub user_name: String,
    /// Export options used last.
    pub export: ExportPreferences,
}

impl AppSettings {
    /// The settings with surrounding blanks removed from the user name and a malformed locale
    /// dropped. A locale is a language tag of letters, digits and `-`, at most 35 characters.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.user_name = self.user_name.trim().to_owned();
        self.locale = self.locale.and_then(|locale| {
            let locale = locale.trim().to_owned();
            let ok = !locale.is_empty()
                && locale.len() <= 35
                && locale
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-');
            ok.then_some(locale)
        });
        self
    }

    /// Reads the settings file. A missing file gives the defaults; a broken one too, with a
    /// warning.
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!("cannot read {}: {error}", path.display());
                return Self::default();
            }
        };
        // Fields missing in the file keep their defaults: the file is laid over the defaults.
        let parsed = serde_json::from_str::<serde_json::Value>(&text).and_then(|file| {
            let mut merged = serde_json::to_value(Self::default())?;
            overlay(&mut merged, file);
            serde_json::from_value::<Self>(merged)
        });
        match parsed {
            Ok(settings) => settings.normalized(),
            Err(error) => {
                tracing::warn!("ignoring malformed {}: {error}", path.display());
                Self::default()
            }
        }
    }

    /// Writes the settings file: first a temporary file next to it, then a rename, so a crash
    /// never leaves half a file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        text.push('\n');
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, path)
    }
}

/// Lays `top` over `base`: objects are merged key by key, other values replace.
fn overlay(base: &mut serde_json::Value, top: serde_json::Value) {
    match (base, top) {
        (serde_json::Value::Object(base), serde_json::Value::Object(top)) => {
            for (key, value) in top {
                match base.get_mut(&key) {
                    Some(slot) => overlay(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, top) => *base = top,
    }
}

/// The settings and what the settings view shows next to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct SettingsView {
    /// The stored settings.
    pub settings: AppSettings,
    /// The operating system user name, used when `user_name` is empty (D-27).
    pub os_user_name: String,
}

/// The settings of the running app and where they are stored.
#[derive(Debug)]
pub struct SettingsState {
    path: Option<PathBuf>,
    current: Mutex<AppSettings>,
}

impl SettingsState {
    /// Loads the settings from `path`; without a path (no config directory) the defaults are
    /// used and changes are kept for this run only.
    pub fn load(path: Option<PathBuf>) -> Self {
        let current = path.as_deref().map(AppSettings::load).unwrap_or_default();
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    /// The current settings.
    pub fn get(&self) -> AppSettings {
        self.current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Stores new settings and writes the file. On a write error the settings still apply to
    /// this run and the error is returned.
    pub fn set(&self, settings: AppSettings) -> Result<AppSettings, CommandError> {
        let settings = settings.normalized();
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = settings.clone();
        if let Some(path) = &self.path {
            settings.save(path).map_err(|e| CommandError::Io {
                message: format!("{}: {e}", path.display()),
            })?;
        }
        Ok(settings)
    }

    fn view(&self) -> SettingsView {
        SettingsView {
            settings: self.get(),
            os_user_name: os_user_name(),
        }
    }
}

/// Applies the settings that Rust acts on: the audit user name of the session and the theme
/// of the native window. Call it off the main thread: it waits for the session.
pub fn apply(app: &AppHandle, settings: &AppSettings) {
    app.state::<SessionState>()
        .lock()
        .set_user_name(&settings.user_name);
    apply_theme(app.get_webview_window(MAIN_WINDOW).as_ref(), settings.theme);
}

/// Sets the theme of the native window (title bar and the webview's color scheme).
pub fn apply_theme(window: Option<&WebviewWindow>, theme: Theme) {
    if let Some(window) = window
        && let Err(error) = window.set_theme(theme.window_theme())
    {
        tracing::warn!("cannot set the window theme: {error}");
    }
}

/// The settings of the user and the operating system user name.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub fn app_settings(state: tauri::State<'_, SettingsState>) -> SettingsView {
    state.view()
}

/// Stores the settings, writes them to the settings file and applies the audit user name and
/// the window theme at once. Returns the stored settings (blanks trimmed).
#[tauri::command]
#[specta::specta]
pub async fn set_app_settings(
    app: AppHandle,
    settings: AppSettings,
) -> Result<SettingsView, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<SettingsState>();
        let stored = state.set(settings);
        apply(&app, &state.get());
        stored.map(|_| state.view())
    })
    .await
    .map_err(|e| CommandError::Io {
        message: e.to_string(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_broken_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE);
        assert_eq!(AppSettings::load(&path), AppSettings::default());
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(AppSettings::load(&path), AppSettings::default());
    }

    #[test]
    fn settings_round_trip_and_tolerate_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join(SETTINGS_FILE);
        let settings = AppSettings {
            theme: Theme::Dark,
            locale: Some("de".into()),
            user_name: "Anna Muster".into(),
            export: ExportPreferences {
                language: ReportLanguage::De,
                pdf_balloons: PdfBalloons::Annotations,
            },
        };
        settings.save(&path).unwrap();
        assert_eq!(AppSettings::load(&path), settings);
        assert!(!path.with_extension("json.tmp").exists());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"theme\": \"dark\""), "{text}");
        std::fs::write(&path, r#"{"theme": "light", "future": 1}"#).unwrap();
        let loaded = AppSettings::load(&path);
        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.locale, None);
        assert_eq!(loaded.export, ExportPreferences::default());
        std::fs::write(&path, r#"{"export": {"language": "de"}}"#).unwrap();
        let loaded = AppSettings::load(&path);
        assert_eq!(loaded.export.language, ReportLanguage::De);
        assert_eq!(loaded.export.pdf_balloons, PdfBalloons::PageContent);
        assert_eq!(loaded.theme, Theme::System);
    }

    #[test]
    fn normalizing_trims_names_and_drops_bad_locales() {
        let settings = AppSettings {
            locale: Some(" de-CH ".into()),
            user_name: "  Anna ".into(),
            ..AppSettings::default()
        }
        .normalized();
        assert_eq!(settings.locale.as_deref(), Some("de-CH"));
        assert_eq!(settings.user_name, "Anna");
        for bad in ["", "../x", "de_CH", &"x".repeat(36)] {
            let s = AppSettings {
                locale: Some(bad.to_owned()),
                ..AppSettings::default()
            };
            assert_eq!(s.normalized().locale, None, "{bad:?}");
        }
    }

    #[test]
    fn state_without_path_keeps_settings_in_memory() {
        let state = SettingsState::load(None);
        let stored = state
            .set(AppSettings {
                theme: Theme::Light,
                ..AppSettings::default()
            })
            .unwrap();
        assert_eq!(stored.theme, Theme::Light);
        assert_eq!(state.get().theme, Theme::Light);
        assert_eq!(Theme::System.window_theme(), None);
    }
}
