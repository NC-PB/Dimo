//! "Copy diagnostics" in File > About (D-06, R.10): a plain text the user pastes into a bug
//! report. It holds the app version, the operating system, the settings without personal data and
//! the last lines of the log file ([`crate::logging`]).
//!
//! Never in the text: drawing or project content (the log does not hold it, see
//! [`crate::logging`]), the user and company name of the settings, and the home folder or the
//! login name, which are replaced by `~` and `<user>` wherever they appear. File names in log
//! lines stay, they are needed to follow an error. Nothing is sent anywhere: the frontend puts
//! the text on the clipboard and the user decides where it goes (rule 9).

use std::path::{Path, PathBuf};

use tauri::State;

use crate::env::{UNKNOWN_USER, os_user_name};
use crate::ipc::{AppInfo, CommandError};
use crate::logging::rotated;
use crate::settings::{AppSettings, SettingsState};
use crate::tiles::TileState;

/// Number of log lines in the text, the newest last.
pub const LOG_LINES: usize = 200;

/// Shortest name that is replaced in the log lines. Shorter names would replace parts of
/// ordinary words.
const MIN_REDACTED_LEN: usize = 3;

/// The log file of the running app, `None` if it could not be opened. Managed state.
#[derive(Debug, Clone, Default)]
pub struct LogFile(pub Option<PathBuf>);

/// Facts of the running app that the text reports.
#[derive(Debug, Clone)]
pub struct Facts {
    /// Version, build profile and PDF engine.
    pub info: AppInfo,
    /// Operating system and architecture, for example `macos aarch64`.
    pub platform: String,
    /// The log file, `None` if there is none.
    pub log_file: Option<PathBuf>,
    /// Home folder of the user, replaced by `~`.
    pub home: Option<PathBuf>,
    /// Login name of the user, replaced by `<user>`.
    pub login: String,
}

/// The diagnostics text for `facts` and `settings`, with `log` as the log text (older lines
/// first). Personal data is replaced as the module documentation says.
pub fn text(facts: &Facts, settings: &AppSettings, log: &str) -> String {
    let redact = Redactor::new(facts, settings);
    let info = &facts.info;
    let engine = if info.pdfium_available {
        "available"
    } else {
        "missing"
    };
    let log_file = facts.log_file.as_deref().map_or_else(
        || "none".to_owned(),
        |path| redact.apply(&path.display().to_string()),
    );
    let lines: Vec<&str> = log.lines().collect();
    let start = lines.len().saturating_sub(LOG_LINES);
    let mut out = format!(
        "Dimo diagnostics\nVersion: {} ({:?})\nPlatform: {}\nPDF engine: {engine}\n\
         Log file: {log_file}\nSettings: {}\n\nLast {} log lines:\n",
        info.version,
        info.build_profile,
        facts.platform,
        settings_line(settings),
        lines.len() - start
    );
    for line in &lines[start..] {
        out.push_str(&redact.apply(line));
        out.push('\n');
    }
    out
}

/// The settings as one JSON line, the user and company name replaced by whether they are set.
fn settings_line(settings: &AppSettings) -> String {
    let mut value = serde_json::to_value(settings).unwrap_or_default();
    if let Some(map) = value.as_object_mut() {
        for (key, name) in [
            ("user_name", &settings.user_name),
            ("company_name", &settings.company_name),
        ] {
            let state = if name.trim().is_empty() {
                "empty"
            } else {
                "set"
            };
            map.insert(key.to_owned(), state.into());
        }
    }
    value.to_string()
}

/// Replaces the home folder, the login name and the names of the settings in a text.
struct Redactor {
    /// Text to find and its replacement, longest first.
    pairs: Vec<(String, &'static str)>,
}

impl Redactor {
    fn new(facts: &Facts, settings: &AppSettings) -> Self {
        let mut pairs = Vec::new();
        if let Some(home) = &facts.home {
            let home = home.display().to_string();
            let home = home.trim_end_matches(['/', '\\']);
            if !home.is_empty() {
                pairs.push((home.to_owned(), "~"));
            }
        }
        for (name, replacement) in [
            (settings.user_name.trim(), "<user>"),
            (settings.company_name.trim(), "<company>"),
            (facts.login.trim(), "<user>"),
        ] {
            // The fallback of `os_user_name` is a word, not a name.
            if name.chars().count() >= MIN_REDACTED_LEN && name != UNKNOWN_USER {
                pairs.push((name.to_owned(), replacement));
            }
        }
        pairs.sort_by_key(|(find, _)| std::cmp::Reverse(find.len()));
        Self { pairs }
    }

    fn apply(&self, text: &str) -> String {
        let mut text = text.to_owned();
        for (find, replacement) in &self.pairs {
            text = text.replace(find.as_str(), replacement);
        }
        text
    }
}

/// The log text: the rotated file `dimo.log.1` followed by `dimo.log`. Missing or unreadable
/// files count as empty; invalid UTF-8 is replaced.
pub fn read_log(path: &Path) -> String {
    let read = |path: &Path| {
        std::fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    };
    let mut log = read(&rotated(path));
    if !log.is_empty() && !log.ends_with('\n') {
        log.push('\n');
    }
    log.push_str(&read(path));
    log
}

/// The diagnostics text for "Copy diagnostics" (D-06, R.10). The frontend puts it on the
/// clipboard.
#[tauri::command]
#[specta::specta]
pub async fn diagnostics(
    app: tauri::AppHandle,
    tiles: State<'_, TileState>,
    settings: State<'_, SettingsState>,
    log_file: State<'_, LogFile>,
) -> Result<String, CommandError> {
    use tauri::Manager as _;
    let facts = Facts {
        info: AppInfo::current(tiles.service().is_ok()),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        log_file: log_file.0.clone(),
        home: app.path().home_dir().ok(),
        login: os_user_name(),
    };
    let settings = settings.get();
    crate::ipc::blocking(move || {
        let log = facts.log_file.as_deref().map(read_log).unwrap_or_default();
        Ok(text(&facts, &settings, &log))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::BuildProfile;

    fn facts(log_file: Option<PathBuf>) -> Facts {
        Facts {
            info: AppInfo {
                version: "0.1.0".to_owned(),
                build_profile: BuildProfile::Release,
                pdfium_available: true,
            },
            platform: "macos aarch64".to_owned(),
            log_file,
            home: Some(PathBuf::from("/Users/anna")),
            login: "anna".to_owned(),
        }
    }

    #[test]
    fn text_has_version_platform_and_settings_without_names() {
        let settings = AppSettings {
            user_name: "Anna Muster".to_owned(),
            company_name: "Muster Mechanik AG".to_owned(),
            locale: Some("de".to_owned()),
            ..AppSettings::default()
        };
        let text = text(
            &facts(Some(PathBuf::from("/Users/anna/Library/Logs/dimo.log"))),
            &settings,
            "",
        );
        assert!(
            text.starts_with("Dimo diagnostics\nVersion: 0.1.0 (Release)\n"),
            "{text}"
        );
        assert!(text.contains("Platform: macos aarch64\n"), "{text}");
        assert!(text.contains("PDF engine: available\n"), "{text}");
        assert!(
            text.contains("Log file: ~/Library/Logs/dimo.log\n"),
            "{text}"
        );
        assert!(text.contains(r#""user_name":"set""#), "{text}");
        assert!(text.contains(r#""company_name":"set""#), "{text}");
        assert!(text.contains(r#""locale":"de""#), "{text}");
        assert!(text.contains("Last 0 log lines:\n"), "{text}");
        for personal in ["Anna", "Muster", "/Users/anna"] {
            assert!(!text.contains(personal), "{personal} in {text}");
        }
    }

    #[test]
    fn log_lines_are_the_newest_and_redacted() {
        let settings = AppSettings {
            user_name: "Anna Muster".to_owned(),
            company_name: "Muster Mechanik AG".to_owned(),
            ..AppSettings::default()
        };
        let mut log: String = (0..LOG_LINES + 5)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        log.push('\n');
        log.push_str("WARN cannot write /Users/anna/Desktop/part.dimo: denied\n");
        log.push_str("INFO report by Anna Muster for Muster Mechanik AG, login anna\n");
        let text = text(&facts(None), &settings, &log);
        assert!(text.contains("Log file: none\n"), "{text}");
        assert!(
            text.contains(&format!("Last {LOG_LINES} log lines:\n")),
            "{text}"
        );
        assert!(!text.contains("line 6\n"), "older lines are left out");
        assert!(text.contains("line 7\n"));
        assert!(
            text.contains("cannot write ~/Desktop/part.dimo: denied\n"),
            "{text}"
        );
        assert!(
            text.contains("report by <user> for <company>, login <user>\n"),
            "{text}"
        );
    }

    #[test]
    fn short_names_are_not_replaced() {
        let settings = AppSettings {
            user_name: "Al".to_owned(),
            ..AppSettings::default()
        };
        let mut facts = facts(None);
        facts.login = "a".to_owned();
        let text = text(&facts, &settings, "Alignment of a sheet\n");
        assert!(text.contains("Alignment of a sheet\n"), "{text}");
        facts.login = UNKNOWN_USER.to_owned();
        let text = super::text(&facts, &settings, "unknown table\n");
        assert!(
            text.contains("unknown table\n"),
            "the login fallback is kept: {text}"
        );
    }

    /// A new settings field must be checked for personal data before it shows up in the text.
    #[test]
    fn settings_fields_are_known() {
        let value = serde_json::to_value(AppSettings::default()).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["company_name", "export", "locale", "theme", "user_name"],
            "check the new field for personal data in settings_line"
        );
        let mut export: Vec<&str> = value["export"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        export.sort_unstable();
        assert_eq!(
            export,
            [
                "blank_columns",
                "check_sheet_format",
                "inspection_report_format",
                "language",
                "open_after_export",
                "pdf_balloons"
            ],
            "check the new export field for personal data in settings_line"
        );
    }

    #[test]
    fn read_log_joins_the_rotated_file_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(crate::logging::LOG_FILE);
        assert_eq!(read_log(&path), "", "no files");
        std::fs::write(rotated(&path), "old").unwrap();
        std::fs::write(&path, "new\n").unwrap();
        assert_eq!(read_log(&path), "old\nnew\n");
    }
}
