/**
 * Page layout of the report preview (T3.19): which tiles of a preview page to show and where.
 * The pages come from Rust as an ordinary document of the tile service (`report_preview`), so
 * they load through `dimo://tile/...` like a drawing. Screen geometry only.
 */

import { TILE_SIZE } from "$lib/ipc/bindings";
import { gridSize, tileZoomFor, tileUrl, zoomScale } from "$lib/viewport/tiles";
import type { Size } from "$lib/viewport/view-math";

/** One tile of a preview page, placed in CSS px from the page's top left corner. */
export interface PreviewTile {
  key: string;
  url: string;
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Tiles of page `page` (sheet units) of document `doc`, drawn `width` CSS px wide on a screen
 * with `devicePixelRatio`: the coarsest tile level that is not upscaled, as the viewport picks
 * it. `base` is the tile URL base (tests); inside Tauri the platform base is used.
 */
export function previewTiles(
  doc: string,
  index: number,
  page: Size,
  width: number,
  devicePixelRatio: number,
  base?: string,
): PreviewTile[] {
  if (!(width > 0) || !(page.width > 0) || !(page.height > 0)) {
    return [];
  }
  const scale = width / page.width; // CSS px per sheet unit
  const zoom = tileZoomFor(scale, devicePixelRatio);
  const grid = gridSize(page, zoom);
  const cssPerPx = scale / zoomScale(zoom);
  const tiles: PreviewTile[] = [];
  for (let y = 0; y < grid.rows; y++) {
    for (let x = 0; x < grid.columns; x++) {
      const w = Math.min(TILE_SIZE, grid.widthPx - x * TILE_SIZE);
      const h = Math.min(TILE_SIZE, grid.heightPx - y * TILE_SIZE);
      const address = { doc, sheet: index, zoom, x, y };
      tiles.push({
        key: `${String(zoom)}/${String(x)}/${String(y)}`,
        url: base === undefined ? tileUrl(address) : tileUrl(address, base),
        left: x * TILE_SIZE * cssPerPx,
        top: y * TILE_SIZE * cssPerPx,
        width: w * cssPerPx,
        height: h * cssPerPx,
      });
    }
  }
  return tiles;
}
