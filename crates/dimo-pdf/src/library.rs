//! Locating the PDFium shared library.
//!
//! Search order, first match wins:
//!
//! 1. An explicit path passed by the caller ([`crate::PdfEngine::start_with_library`]).
//!    The installed app uses this with the path of the bundled library.
//! 2. The environment variable `DIMO_PDFIUM_PATH`: either the library file itself or a
//!    directory that contains it (`libpdfium.dylib`, `libpdfium.so` or `pdfium.dll`).
//! 3. The development default `vendor/pdfium/<target>/lib/` (`bin/` on Windows) inside the
//!    repository, filled by `scripts/fetch-pdfium.sh` or `scripts/fetch-pdfium.ps1`.
//!    `<target>` is the archive name used in `scripts/pdfium.toml`, for example `mac-arm64`.
//!
//! There is no fallback to a system wide PDFium: the version must match the bindings
//! (`scripts/pdfium.toml`), and rendering must be reproducible.

use std::path::{Path, PathBuf};

use crate::PdfError;

/// Environment variable that overrides the library location.
pub const PDFIUM_PATH_ENV: &str = "DIMO_PDFIUM_PATH";

/// File name of the PDFium library on this platform.
pub const LIBRARY_FILE_NAME: &str = if cfg!(target_os = "windows") {
    "pdfium.dll"
} else if cfg!(target_os = "macos") {
    "libpdfium.dylib"
} else {
    "libpdfium.so"
};

/// Target key of this build as used in `scripts/pdfium.toml` and `vendor/pdfium/`,
/// or `None` on a platform without a pinned PDFium build.
pub const fn host_target() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("mac-arm64")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("mac-x64")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("win-x64")
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some("win-arm64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x64")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("linux-arm64")
    } else {
        None
    }
}

/// The development default: `vendor/pdfium/<target>/{lib,bin}/<library>` in this repository.
///
/// Derived from the crate location at compile time, so it only exists in a source checkout.
pub fn vendor_library_path() -> Option<PathBuf> {
    let target = host_target()?;
    let sub = if cfg!(target_os = "windows") {
        "bin"
    } else {
        "lib"
    };
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    Some(
        repo_root
            .join("vendor/pdfium")
            .join(target)
            .join(sub)
            .join(LIBRARY_FILE_NAME),
    )
}

/// Resolves the library file according to the search order in the module docs.
///
/// An explicit path or the environment variable may name the file or its directory.
pub fn resolve_library_path(explicit: Option<&Path>) -> Result<PathBuf, PdfError> {
    let env = std::env::var_os(PDFIUM_PATH_ENV).map(PathBuf::from);
    resolve_from(explicit, env.as_deref(), vendor_library_path().as_deref())
}

fn resolve_from(
    explicit: Option<&Path>,
    env: Option<&Path>,
    vendor: Option<&Path>,
) -> Result<PathBuf, PdfError> {
    let mut searched = Vec::new();
    // An explicit path or the variable, when given, is binding: no silent fallback to vendor/.
    if let Some(given) = explicit.or(env) {
        let candidate = if given.is_dir() {
            given.join(LIBRARY_FILE_NAME)
        } else {
            given.to_path_buf()
        };
        if candidate.is_file() {
            return Ok(candidate);
        }
        searched.push(candidate);
        return Err(PdfError::LibraryNotFound { searched });
    }
    if let Some(vendor) = vendor {
        if vendor.is_file() {
            return Ok(vendor.to_path_buf());
        }
        searched.push(vendor.to_path_buf());
    }
    Err(PdfError::LibraryNotFound { searched })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target_matches_toml_keys() {
        let keys = [
            "mac-arm64",
            "mac-x64",
            "win-x64",
            "win-arm64",
            "linux-x64",
            "linux-arm64",
        ];
        if let Some(t) = host_target() {
            assert!(keys.contains(&t), "{t}");
        }
    }

    #[test]
    fn explicit_missing_path_does_not_fall_back() {
        let missing = Path::new("/definitely/not/here/libpdfium");
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let err = resolve_from(Some(missing), None, Some(&vendor)).unwrap_err();
        assert!(matches!(err, PdfError::LibraryNotFound { searched } if searched.len() == 1));
    }

    #[test]
    fn explicit_file_wins() {
        // Any existing file stands in for the library here; it is not loaded.
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert_eq!(resolve_from(Some(&file), None, None).unwrap(), file);
    }

    #[test]
    fn env_directory_gets_library_name() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let err = resolve_from(None, Some(dir), None).unwrap_err();
        let PdfError::LibraryNotFound { searched } = err else {
            panic!("unexpected error");
        };
        assert_eq!(searched, vec![dir.join(LIBRARY_FILE_NAME)]);
    }

    #[test]
    fn vendor_default_used_when_nothing_given() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert_eq!(resolve_from(None, None, Some(&file)).unwrap(), file);
        assert!(resolve_from(None, None, None).is_err());
    }
}
