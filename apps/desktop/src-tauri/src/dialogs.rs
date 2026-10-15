//! Native file dialogs (`rfd`), so the webview never names a path (NFR-SEC-01).
//!
//! Debug builds can skip a dialog through a `DIMO_DEV_*` variable (see [`crate::dev`]), so
//! scripted runs work without clicks. The override is resolved here, in one place: callers name
//! the [`DevOverride`] that applies to their dialog. Release builds always show the dialog.

use std::path::PathBuf;

use tauri::WebviewWindow;

/// A file type filter of a dialog: its name and the extensions without the dot. The first
/// extension is added to a saved file name that has none.
#[derive(Debug, Clone, Copy)]
pub struct Filter {
    /// Name shown in the dialog, for example `PDF`.
    pub name: &'static str,
    /// Extensions without the dot, at least one.
    pub extensions: &'static [&'static str],
}

/// The `DIMO_DEV_*` variable that can replace a dialog in debug builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevOverride {
    /// No override: always show the dialog.
    None,
    /// `DIMO_DEV_NEW_DRAWING`: the drawing of a new project.
    NewDrawing,
    /// `DIMO_DEV_IMPORT_TABLE`: the custom tolerance table to import.
    ImportTable,
    /// `DIMO_DEV_SAVE_AS`: the target of "save as".
    SaveAs,
    /// `DIMO_DEV_EXPORT_DIR`: the folder an export writes its suggested file name into.
    ExportDir,
}

impl DevOverride {
    /// The path that replaces the dialog, `None` to show it. Always `None` in release builds.
    fn path(self, suggested: Option<&str>) -> Option<PathBuf> {
        match self {
            Self::None => None,
            Self::NewDrawing => crate::dev::dev_new_drawing(),
            Self::ImportTable => crate::dev::dev_import_table(),
            Self::SaveAs => crate::dev::dev_save_as(),
            Self::ExportDir => crate::dev::dev_export_dir().map(|dir| {
                let _ = std::fs::create_dir_all(&dir);
                dir.join(suggested.unwrap_or_default())
            }),
        }
    }
}

/// Lets the user pick an existing file. `None` if the dialog was cancelled.
pub async fn pick_file(
    window: &WebviewWindow,
    filter: Filter,
    dev: DevOverride,
) -> Option<PathBuf> {
    if let Some(path) = dev.path(None) {
        return Some(path);
    }
    rfd::AsyncFileDialog::new()
        .add_filter(filter.name, filter.extensions)
        .set_parent(window)
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

/// Lets the user choose a file to write, starting with the name `suggested`. A chosen name
/// without an extension gets the first extension of `filter`. `None` if the dialog was
/// cancelled.
pub async fn save_file(
    window: &WebviewWindow,
    filter: Filter,
    suggested: &str,
    dev: DevOverride,
) -> Option<PathBuf> {
    let path = match dev.path(Some(suggested)) {
        Some(path) => path,
        None => rfd::AsyncFileDialog::new()
            .add_filter(filter.name, filter.extensions)
            .set_file_name(suggested)
            .set_parent(window)
            .save_file()
            .await?
            .path()
            .to_path_buf(),
    };
    Some(with_default_extension(path, filter))
}

/// `path` with the first extension of `filter` if it has none.
fn with_default_extension(mut path: PathBuf, filter: Filter) -> PathBuf {
    if path.extension().is_none()
        && let Some(extension) = filter.extensions.first()
    {
        path.set_extension(extension);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: Filter = Filter {
        name: "CSV",
        extensions: &["csv"],
    };

    #[test]
    fn saved_names_get_the_default_extension_only_without_one() {
        assert_eq!(
            with_default_extension(PathBuf::from("/tmp/x"), CSV),
            PathBuf::from("/tmp/x.csv")
        );
        assert_eq!(
            with_default_extension(PathBuf::from("/tmp/x.txt"), CSV),
            PathBuf::from("/tmp/x.txt")
        );
    }

    #[test]
    fn no_override_shows_the_dialog() {
        assert_eq!(DevOverride::None.path(Some("a.csv")), None);
    }
}
