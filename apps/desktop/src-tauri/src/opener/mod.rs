//! Opening files and the release page with the default application of the operating system.
//!
//! Only two things are ever opened: a file DimoMark just wrote, whose extension must be the one of
//! its format ([`open_written_file`]), and the DimoMark release page on GitHub for the update check
//! ([`open_url`], R.9). The path or URL is one argument of the launcher, never part of a shell
//! string: `open` on macOS, `xdg-open` on Linux and `ShellExecuteW` on Windows (the one module
//! with `unsafe`, see `windows.rs`).

#[cfg(windows)]
mod windows;

use std::ffi::OsStr;
use std::path::Path;

use crate::ipc::{CommandError, ExportReason};

/// The only web address [`open_url`] opens, and the pages below it. Written as GitHub spells the
/// repository in its answers; the comparison ignores case like GitHub does for owner and name.
pub const RELEASES_URL: &str = "https://github.com/NC-PB/Dimo/releases";

/// Opens the file `path` that DimoMark just wrote, through `opener`. Refused if its extension is not
/// `extension`: the user can type any file name in the save dialog, and a file of another type
/// (a script, say) must never be started from here.
pub(crate) fn open_written_file(
    path: &Path,
    extension: &str,
    opener: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), CommandError> {
    let matches = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension));
    if !matches {
        return Err(CommandError::export(
            ExportReason::WrongExtension,
            format_args!("not opened: {} is not a .{extension} file", path.display()),
        ));
    }
    opener(path).map_err(|e| {
        tracing::warn!("cannot open {}: {e}", path.display());
        CommandError::export(
            ExportReason::OpenFailed,
            format_args!("cannot open {}: {e}", path.display()),
        )
    })
}

/// Starts the default application for the file `path` and does not wait for it. The path is
/// made absolute, so it cannot be read as an option.
pub(crate) fn open_with_system(path: &Path) -> std::io::Result<()> {
    launch(std::path::absolute(path)?.as_os_str())
}

/// Opens `url` in the browser if it is the DimoMark release page or a page below it
/// ([`RELEASES_URL`]); every other address is refused with `InvalidInput` (R.9, ADR 0008).
pub fn open_url(url: &str) -> std::io::Result<()> {
    if !is_release_page(url) {
        tracing::warn!("refused to open the address {url:?}");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("only {RELEASES_URL} can be opened"),
        ));
    }
    launch(OsStr::new(url))
}

/// True for [`RELEASES_URL`] and the pages below it, written with letters, digits and
/// `- . _ ~ /` only: no query, fragment, escapes, spaces or `..`.
pub(crate) fn is_release_page(url: &str) -> bool {
    let Some((prefix, rest)) = url.split_at_checked(RELEASES_URL.len()) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(RELEASES_URL) {
        return false;
    }
    (rest.is_empty() || rest.starts_with('/'))
        && !rest.contains("..")
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~/".contains(&b))
}

/// Starts the default application for `target` (an absolute path or an allowed URL) without
/// waiting for it. Windows uses `ShellExecuteW` (no shell string); the others a launcher
/// process, reaped on a thread.
#[cfg(windows)]
fn launch(target: &OsStr) -> std::io::Result<()> {
    windows::shell_open(target)
}

/// Starts the launcher for `target` and reaps it on a thread, so the caller does not wait; the
/// opened application keeps running.
#[cfg(not(windows))]
fn launch(target: &OsStr) -> std::io::Result<()> {
    let mut child = open_command(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// The process that opens `target` with the default application: `open` on macOS, `xdg-open`
/// elsewhere.
#[cfg(not(windows))]
fn open_command(target: &OsStr) -> std::process::Command {
    let mut command = std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    });
    command.arg(target);
    command
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn written_files_open_only_with_their_own_extension() {
        let mut opened = Vec::new();
        let ok = open_written_file(Path::new("/tmp/part_check-sheet.PDF"), "pdf", |p| {
            opened.push(p.to_path_buf());
            Ok(())
        });
        assert!(ok.is_ok());
        assert_eq!(opened, [PathBuf::from("/tmp/part_check-sheet.PDF")]);
        for (name, extension) in [
            ("/tmp/run.bat", "pdf"),
            ("/tmp/noextension", "csv"),
            ("/tmp/list.xlsx", "csv"),
        ] {
            let refused = open_written_file(Path::new(name), extension, |_| {
                panic!("{name} must not be opened")
            });
            assert!(
                matches!(
                    refused,
                    Err(CommandError::Export {
                        reason: ExportReason::WrongExtension,
                        ..
                    })
                ),
                "{name}"
            );
        }
        let failed = open_written_file(Path::new("/tmp/a.csv"), "csv", |_| {
            Err(std::io::Error::other("no application"))
        });
        assert!(
            matches!(&failed, Err(CommandError::Export { reason: ExportReason::OpenFailed, message }) if message.contains("no application")),
            "{failed:?}"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn open_command_passes_the_target_as_one_argument() {
        let path = std::path::absolute("/tmp/a b; touch x & c.pdf").unwrap();
        let command = open_command(path.as_os_str());
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [path.as_os_str()]);
        let launcher = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        assert_eq!(command.get_program(), launcher);
    }

    /// NFR-PORT-02: an export on a network share is opened by its UNC path, unchanged: no
    /// `\\?\UNC\` form and no drive letter.
    #[cfg(windows)]
    #[test]
    fn unc_paths_reach_the_opener_unchanged() {
        let path = Path::new(r"\\server\share\Zeichnungen Müller\Prüfteil 1_check-sheet.pdf");
        assert_eq!(std::path::absolute(path).unwrap(), path);
        let mut opened = None;
        open_written_file(path, "pdf", |p| {
            opened = Some(p.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(opened.as_deref(), Some(path));
    }

    #[test]
    fn only_the_release_page_is_opened() {
        for ok in [
            RELEASES_URL,
            "https://github.com/NC-PB/dimo/releases/",
            "https://github.com/NC-PB/dimo/releases/latest",
            "https://github.com/NC-PB/dimo/releases/tag/v0.1.0",
            "https://github.com/nc-pb/DIMO/releases/tag/v0.1.0",
        ] {
            assert!(is_release_page(ok), "{ok}");
        }
        for refused in [
            "",
            "https://github.com/NC-PB/dimo",
            "https://github.com/NC-PB/dimo/releasesX",
            "https://github.com/NC-PB/dimo/releaseß",
            "HTTPS://github.com/NC-PB/dimo/releases/../x",
            "https://github.com/NC-PB/dimo/releases.evil.example",
            "http://github.com/NC-PB/dimo/releases",
            "https://github.com/NC-PB/dimo/releases/../../other",
            "https://github.com/NC-PB/dimo/releases/tag/v1?x=1",
            "https://github.com/NC-PB/dimo/releases/tag/v1#x",
            "https://github.com/NC-PB/dimo/releases/%2e%2e",
            "https://github.com/NC-PB/dimo/releases/a b",
            "https://github.com/NC-PB/dimo/releases/a\"&calc",
            "https://evil.example/https://github.com/NC-PB/dimo/releases",
            "file:///etc/passwd",
            "/Applications/Calculator.app",
        ] {
            assert!(!is_release_page(refused), "{refused}");
            let error = open_url(refused).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{refused}");
        }
    }
}
