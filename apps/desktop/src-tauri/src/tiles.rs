//! Wiring of the `dimo://tile/...` protocol (T0.7). The logic lives in `dimo_pdf::tiles`.
//!
//! The webview reaches the scheme under a platform specific URL (see `tileUrl` in
//! `src/lib/viewport/tiles.ts`): `dimo://localhost/tile/...` on macOS and Linux,
//! `http://dimo.localhost/tile/...` on Windows. The handler is asynchronous: it hands the
//! request to the tile service and returns at once; a worker thread answers later. Neither the
//! main thread nor the tokio runtime ever waits for a render.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dimo_pdf::PdfEngine;
use dimo_pdf::library::{LIBRARY_FILE_NAME, PDFIUM_PATH_ENV};
use dimo_pdf::tiles::{TILE_CONTENT_TYPE, Tile, TileConfig, TileError, TileKey, TileService};
use tauri::http::{self, Method, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

/// Name of the custom URI scheme.
pub const SCHEME: &str = "dimo";

/// Tiles are addressed by content hash and never change within an app version.
const CACHE_CONTROL: &str = "private, max-age=31536000, immutable";

/// Folder below the resource directory that holds the bundled PDFium on Windows and Linux.
pub const PDFIUM_RESOURCE_DIR: &str = "pdfium";

/// The PDFium library shipped with an installed app (R.13, ADR 0005), `None` in a source
/// checkout. macOS keeps it in `Contents/Frameworks/` next to the executable's folder, where the
/// bundle signs it; Windows and Linux in [`PDFIUM_RESOURCE_DIR`] below the resource directory
/// (the folder of the executable on Windows, also for the portable zip). Only macOS looks next
/// to the executable's folder, so a library in a neighbouring folder is never loaded elsewhere.
pub fn bundled_pdfium(exe: Option<&Path>, resource_dir: Option<&Path>) -> Option<PathBuf> {
    let frameworks = exe
        .filter(|_| cfg!(target_os = "macos"))
        .and_then(Path::parent)
        .map(|dir| dir.join("../Frameworks").join(LIBRARY_FILE_NAME));
    let resources = resource_dir.map(|dir| dir.join(PDFIUM_RESOURCE_DIR).join(LIBRARY_FILE_NAME));
    [frameworks, resources]
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
}

/// The tile service of the app, or why it is missing (no PDFium library).
pub struct TileState {
    service: Result<Arc<TileService>, String>,
}

impl TileState {
    /// Loads PDFium and starts the tile service with its disk cache in `cache_dir`. A set
    /// `DIMO_PDFIUM_PATH` wins over the `bundled` library; without either the development default
    /// in `vendor/pdfium/` is used (see `dimo_pdf::library`).
    pub fn start(cache_dir: Option<PathBuf>, bundled: Option<PathBuf>) -> Self {
        let engine = match bundled {
            Some(path) if std::env::var_os(PDFIUM_PATH_ENV).is_none() => {
                PdfEngine::start_with_library(&path)
            }
            _ => PdfEngine::start(),
        };
        let service = engine
            .map(|engine| {
                let config = TileConfig {
                    disk_dir: cache_dir,
                    ..TileConfig::default()
                };
                Arc::new(TileService::new(engine, config))
            })
            .map_err(|e| {
                tracing::warn!("PDFium unavailable, drawings cannot be opened: {e}");
                e.to_string()
            });
        Self { service }
    }

    /// The tile service, or the reason PDFium could not be loaded.
    pub fn service(&self) -> Result<&Arc<TileService>, &str> {
        self.service.as_ref().map_err(String::as_str)
    }
}

/// The custom protocol handler registered for [`SCHEME`].
#[allow(
    clippy::needless_pass_by_value,
    reason = "signature required by register_asynchronous_uri_scheme_protocol"
)]
pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    if request.method() != Method::GET {
        responder.respond(text(StatusCode::METHOD_NOT_ALLOWED, "only GET"));
        return;
    }
    let uri = request.uri();
    let key = match TileKey::from_url_parts(uri.host(), uri.path()) {
        Ok(key) => key,
        Err(e) => {
            responder.respond(response(Err(e)));
            return;
        }
    };
    let Some(state) = ctx.app_handle().try_state::<TileState>() else {
        responder.respond(response(Err(TileError::Stopped)));
        return;
    };
    match state.service() {
        Ok(service) => service.request(key, move |result| responder.respond(response(result))),
        Err(reason) => responder.respond(text(StatusCode::SERVICE_UNAVAILABLE, reason)),
    }
}

/// Maps a tile result to an HTTP response.
///
/// Cancelled requests get `204 No Content`: the image element fires `error` and the viewport
/// requests the tile again if it becomes visible.
pub fn response(result: Result<Tile, TileError>) -> http::Response<Vec<u8>> {
    let error = match result {
        Ok(tile) => {
            return http::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, TILE_CONTENT_TYPE)
                .header(header::CACHE_CONTROL, CACHE_CONTROL)
                // Lets the viewport draw tiles into a canvas without tainting it.
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(tile.as_bytes().to_vec())
                .unwrap_or_default();
        }
        Err(error) => error,
    };
    let status = match &error {
        TileError::InvalidAddress(_) => StatusCode::BAD_REQUEST,
        TileError::UnknownDocument(_) | TileError::OutOfRange(_) => StatusCode::NOT_FOUND,
        TileError::Cancelled => StatusCode::NO_CONTENT,
        TileError::Stopped => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    if status == StatusCode::NO_CONTENT {
        return http::Response::builder()
            .status(status)
            .body(Vec::new())
            .unwrap_or_default();
    }
    text(status, &error.to_string())
}

fn text(status: StatusCode, message: &str) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.as_bytes().to_vec())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_pdfium_is_found_in_frameworks_or_resources() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("MacOS").join("dimo");
        let resources = dir.path().join("Resources");
        assert_eq!(
            bundled_pdfium(Some(&exe), Some(&resources)),
            None,
            "source checkout"
        );

        let in_resources = resources.join(PDFIUM_RESOURCE_DIR).join(LIBRARY_FILE_NAME);
        std::fs::create_dir_all(in_resources.parent().unwrap()).unwrap();
        std::fs::write(&in_resources, b"").unwrap();
        assert_eq!(
            bundled_pdfium(Some(&exe), Some(&resources)),
            Some(in_resources),
            "Windows and Linux"
        );

        let frameworks = dir.path().join("Frameworks");
        std::fs::create_dir_all(&frameworks).unwrap();
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(frameworks.join(LIBRARY_FILE_NAME), b"").unwrap();
        let found = bundled_pdfium(Some(&exe), Some(&resources)).unwrap();
        assert!(found.is_file());
        assert_eq!(
            found.ends_with(Path::new("Frameworks").join(LIBRARY_FILE_NAME)),
            cfg!(target_os = "macos"),
            "macOS looks in Frameworks first, other platforms never: {}",
            found.display()
        );
        assert_eq!(bundled_pdfium(None, None), None);
    }

    #[test]
    fn tile_response_is_cacheable_png() {
        let r = response(Ok(Tile::new(vec![1, 2, 3])));
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(r.headers()[header::CACHE_CONTROL], CACHE_CONTROL);
        assert_eq!(r.body(), &vec![1, 2, 3]);
    }

    #[test]
    fn error_statuses() {
        let status = |e| response(Err(e)).status();
        assert_eq!(
            status(TileError::InvalidAddress(String::new())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(TileError::UnknownDocument(String::new())),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status(TileError::OutOfRange(String::new())),
            StatusCode::NOT_FOUND
        );
        assert_eq!(status(TileError::Cancelled), StatusCode::NO_CONTENT);
        assert_eq!(status(TileError::Stopped), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            status(TileError::Render(String::new())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
