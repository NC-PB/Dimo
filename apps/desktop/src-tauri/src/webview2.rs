//! Start check for the WebView2 runtime on Windows (D-11, R.12).
//!
//! The MSI installs WebView2 through its bootstrapper, but the portable zip relies on the runtime
//! that Windows 11 and updated Windows 10 already have. Without it, creating the window fails and
//! the app would end without a word. [`ensure_runtime`] runs before the window is created and, if
//! no runtime is found, shows a native message box and tells the caller to quit.
//!
//! The message runs before the frontend and before the settings are read, so it cannot use the
//! frontend translations or the language option. It shows the English and the German text one
//! after the other instead. The message box comes from `rfd`, which the file dialogs use already,
//! so no `unsafe` code is needed here.

/// Microsoft's download page of the WebView2 runtime, the same as in the README of the portable
/// zip (`.github/workflows/release.yml`).
pub const DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

/// Title of the message box.
pub const MISSING_TITLE: &str = "DimoMark: WebView2 missing / WebView2 fehlt";

/// Text of the message box: English first, then German ("du", as in the app).
pub fn missing_message(error: &str) -> String {
    format!(
        "DimoMark cannot start: the Microsoft Edge WebView2 Runtime is not installed.\n\
         Install the Evergreen WebView2 Runtime from {DOWNLOAD_URL}\n\
         or use the DimoMark MSI installer, which installs it when needed.\n\
         Then start DimoMark again.\n\
         \n\
         DimoMark kann nicht starten: Die Microsoft Edge WebView2 Runtime ist nicht installiert.\n\
         Installiere die Evergreen WebView2 Runtime von {DOWNLOAD_URL}\n\
         oder verwende das MSI-Installationsprogramm von DimoMark, das sie bei Bedarf installiert.\n\
         Starte DimoMark danach erneut.\n\
         \n\
         Ctrl+C copies this text. / Ctrl+C kopiert diesen Text.\n\
         Details: {error}"
    )
}

/// True if the app can create its window. If no WebView2 runtime is found, shows
/// [`missing_message`] in a native message box and returns false; the caller then quits.
#[must_use]
pub fn ensure_runtime() -> bool {
    let Some(error) = missing_runtime() else {
        return true;
    };
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(MISSING_TITLE)
        .set_description(missing_message(&error))
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    false
}

/// Why no WebView2 runtime can be used, `None` if one is installed. On Windows this asks
/// WebView2 for the version of the installed runtime. Other systems always return `None`:
/// their webview is part of the system.
pub fn missing_runtime() -> Option<String> {
    #[cfg(windows)]
    if let Err(error) = tauri::webview_version() {
        return Some(error.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_names_the_runtime_the_page_and_the_msi_in_both_languages() {
        let text = missing_message("no runtime");
        assert_eq!(text.matches(DOWNLOAD_URL).count(), 2);
        assert!(text.contains("is not installed"));
        assert!(text.contains("ist nicht installiert"));
        assert!(text.contains("MSI installer"));
        assert!(text.contains("MSI-Installationsprogramm"));
        assert!(text.ends_with("Details: no runtime"));
        assert!(MISSING_TITLE.contains("WebView2"));
    }

    #[test]
    fn the_download_page_matches_the_readme_of_the_portable_zip() {
        let workflow = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../.github/workflows/release.yml"),
        )
        .expect("read .github/workflows/release.yml");
        assert!(workflow.contains(DOWNLOAD_URL));
    }

    /// The runtime is present on every machine that runs the tests: macOS and Linux always,
    /// and the windows-2025 CI image ships WebView2. On Windows this calls the real check.
    #[test]
    fn the_runtime_is_found_on_the_test_machine() {
        assert_eq!(missing_runtime(), None);
    }
}
