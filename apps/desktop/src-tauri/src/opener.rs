//! Opening files and the release page with the default application of the operating system.
//!
//! Only two things are ever opened: a file Dimo just wrote, whose extension must be the one of
//! its format ([`open_written_file`]), and the Dimo release page on GitHub for the update check
//! ([`open_url`], R.9). The path or URL is one argument of the launcher, never part of a shell
//! string: `open` on macOS, `xdg-open` on Linux and `cmd /C start` with a quoted argument on
//! Windows.

use std::ffi::OsStr;
use std::path::Path;

use crate::ipc::{CommandError, ExportReason};

/// The only web address [`open_url`] opens, and the pages below it.
pub const RELEASES_URL: &str = "https://github.com/NC-PB/dimo/releases";

/// Opens the file `path` that Dimo just wrote, through `opener`. Refused if its extension is not
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

/// Opens `url` in the browser if it is the Dimo release page or a page below it
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
fn is_release_page(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(RELEASES_URL) else {
        return false;
    };
    (rest.is_empty() || rest.starts_with('/'))
        && !rest.contains("..")
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~/".contains(&b))
}

/// Starts the launcher for `target` (an absolute path or an allowed URL) and reaps it on a
/// thread, so the caller does not wait; the opened application keeps running.
fn launch(target: &OsStr) -> std::io::Result<()> {
    let mut child = open_command(target)?
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// The process that opens `target` with the default application of the operating system.
fn open_command(target: &OsStr) -> std::io::Result<std::process::Command> {
    let mut command;
    if cfg!(target_os = "macos") {
        command = std::process::Command::new("open");
        command.arg(target);
    } else if cfg!(target_os = "windows") {
        // `start` is a command of cmd.exe. Rust quotes an argument only if it has a space, so the
        // target is quoted by hand and passed raw: inside double quotes `& | < > ^ ( )` are plain
        // characters to cmd. The system call (ShellExecuteW) would avoid cmd, but needs `unsafe`,
        // which the workspace forbids.
        let quoted = windows_start_argument(target)?;
        command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::raw_arg(&mut command, quoted);
        #[cfg(not(windows))]
        command.arg(quoted);
    } else {
        command = std::process::Command::new("xdg-open");
        command.arg(target);
    }
    Ok(command)
}

/// `target` as the quoted argument of `cmd /C start "" <target>`. Inside double quotes cmd treats
/// most operators as text, but `%` is still expanded and `"` would end the quote, so a target
/// with either is refused. Not Windows specific code, so every platform tests it.
fn windows_start_argument(target: &OsStr) -> std::io::Result<String> {
    let text = target.to_string_lossy();
    if text.contains(['%', '"', '\n', '\r', '\0']) {
        return Err(std::io::Error::other(
            "the file name contains %, a double quote or a line break",
        ));
    }
    Ok(format!("\"{text}\""))
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

    #[test]
    fn windows_start_argument_is_quoted_and_refuses_expansion() {
        let quoted = windows_start_argument(OsStr::new(r"C:\Temp\a&calc (1) ^b.pdf")).unwrap();
        assert_eq!(quoted, "\"C:\\Temp\\a&calc (1) ^b.pdf\"");
        for bad in ["a%PATH%.pdf", "a\"b.pdf", "a\nb.pdf"] {
            assert!(windows_start_argument(OsStr::new(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn open_command_passes_the_target_as_one_argument() {
        let path = std::path::absolute("/tmp/a b; touch x & c.pdf").unwrap();
        let command = open_command(path.as_os_str()).unwrap();
        let args: Vec<_> = command.get_args().collect();
        let last = args.last().unwrap();
        if cfg!(target_os = "windows") {
            assert_eq!(command.get_program(), "cmd");
            assert_eq!(args.len(), 4);
        } else {
            assert_eq!(Path::new(last), path);
            assert_eq!(args.len(), 1);
            let launcher = if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            assert_eq!(command.get_program(), launcher);
        }
    }

    #[test]
    fn only_the_release_page_is_opened() {
        for ok in [
            RELEASES_URL,
            "https://github.com/NC-PB/dimo/releases/",
            "https://github.com/NC-PB/dimo/releases/latest",
            "https://github.com/NC-PB/dimo/releases/tag/v0.1.0",
        ] {
            assert!(is_release_page(ok), "{ok}");
        }
        for refused in [
            "",
            "https://github.com/NC-PB/dimo",
            "https://github.com/NC-PB/dimo/releasesX",
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
