//! Windows: opens a file or a web address with `ShellExecuteW`.
//!
//! The only module of the project with `unsafe` (owner decision 2026-10-15; the guard test
//! `crates/dimo-cli/tests/unsafe_guard.rs` fails on `unsafe` anywhere else). The target is
//! passed as one wide string, so no shell parses it.
#![allow(unsafe_code)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// `target` as a null terminated UTF-16 string; `InvalidInput` if it contains a NUL.
fn wide(target: &OsStr) -> std::io::Result<Vec<u16>> {
    let mut units: Vec<u16> = target.encode_wide().collect();
    if units.contains(&0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the target contains a NUL character",
        ));
    }
    units.push(0);
    Ok(units)
}

/// Opens `target` (an absolute path or an allowed URL) with the default application and does
/// not wait for it.
pub(super) fn shell_open(target: &OsStr) -> std::io::Result<()> {
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let file = wide(target)?;
    // SAFETY: `operation` and `file` are valid null terminated UTF-16 strings that outlive the
    // call; the optional parameters and directory are null, which the API allows; a null window
    // handle means no owner window. The function does not retain any pointer after it returns.
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    // Per the documentation, a value greater than 32 means success; smaller values are error
    // codes (the HINSTANCE is only a legacy way to carry them).
    let code = result as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "ShellExecuteW failed with code {code}"
        )))
    }
}
