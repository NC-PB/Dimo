import { convertFileSrc } from "@tauri-apps/api/core";
import type { TileAddress } from "$lib/ipc/bindings";

/** Name of the custom URI scheme registered in `src-tauri/src/tiles.rs`. */
export const TILE_SCHEME = "dimo";

let cachedBase: string | null = null;

/**
 * Base URL of the `dimo` scheme on this platform, ending in `/`.
 *
 * Webviews expose custom schemes under different URLs: `dimo://localhost/` on macOS and Linux,
 * `http://dimo.localhost/` on Windows and Android. Tauri knows the platform, so the base is
 * taken from `convertFileSrc` with an empty path. Only works inside Tauri.
 */
export function tileBaseUrl(): string {
  cachedBase ??= convertFileSrc("", TILE_SCHEME);
  return cachedBase;
}

/**
 * URL of one tile: `<base>tile/{doc}/{sheet}/{zoom}/{x}/{y}`. The route matches
 * `TileKey::from_route` in `dimo-pdf`. Pass `base` in tests; inside Tauri the platform base is used.
 */
export function tileUrl(address: TileAddress, base: string = tileBaseUrl()): string {
  const root = base.endsWith("/") ? base : `${base}/`;
  const { doc, sheet, zoom, x, y } = address;
  return `${root}tile/${doc}/${String(sheet)}/${String(zoom)}/${String(x)}/${String(y)}`;
}
