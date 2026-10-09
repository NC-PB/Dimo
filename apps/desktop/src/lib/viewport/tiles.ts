import { convertFileSrc } from "@tauri-apps/api/core";
import {
  MAX_TILE_ZOOM,
  MIN_TILE_ZOOM,
  TILE_SIZE,
  type TileAddress,
  type TileRange,
} from "$lib/ipc/bindings";
import type { Rect, Size } from "./view-math";

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

/** Pixels per sheet unit at a tile zoom level, `2^zoom` (`zoom_scale` in `dimo-pdf`). */
export function zoomScale(zoom: number): number {
  return 2 ** zoom;
}

/**
 * Tolerance in zoom levels before a finer level is used. Showing a level up to 2 % larger than
 * its native size is not visible and avoids switching levels on tiny zoom changes.
 */
const LEVEL_TOLERANCE = 0.03;

/**
 * Tile zoom level for a view scale (CSS px per sheet unit) on a display with
 * `devicePixelRatio`: the coarsest level with at least one tile pixel per device pixel, so the
 * drawing is never noticeably upscaled, clamped to the levels the tile service renders.
 */
export function tileZoomFor(scale: number, devicePixelRatio: number): number {
  const devicePxPerUnit = scale * (devicePixelRatio > 0 ? devicePixelRatio : 1);
  // `+ 0` turns -0 into 0.
  const level = Math.ceil(Math.log2(devicePxPerUnit) - LEVEL_TOLERANCE) + 0;
  return Math.min(MAX_TILE_ZOOM, Math.max(MIN_TILE_ZOOM, level));
}

/** Pixel size and tile count of a sheet at a zoom level, as `TileGrid` in `dimo-pdf`. */
export interface GridSize {
  widthPx: number;
  heightPx: number;
  columns: number;
  rows: number;
}

export function gridSize(sheet: Size, zoom: number): GridSize {
  const s = zoomScale(zoom);
  const widthPx = Math.max(1, Math.ceil(sheet.width * s));
  const heightPx = Math.max(1, Math.ceil(sheet.height * s));
  return {
    widthPx,
    heightPx,
    columns: Math.ceil(widthPx / TILE_SIZE),
    rows: Math.ceil(heightPx / TILE_SIZE),
  };
}

/**
 * Tiles of `sheet` at `zoom` that intersect `area` (sheet units), grown by `margin` tiles on
 * every side for prefetching and clipped to the grid. `null` when the area misses the sheet.
 */
export function tileRangeFor(
  sheetIndex: number,
  sheet: Size,
  zoom: number,
  area: Rect,
  margin = 0,
): TileRange | null {
  // The part of the area on the sheet.
  const left = Math.max(0, area.x);
  const top = Math.max(0, area.y);
  const right = Math.min(sheet.width, area.x + area.width);
  const bottom = Math.min(sheet.height, area.y + area.height);
  if (left >= right || top >= bottom) {
    return null;
  }
  const grid = gridSize(sheet, zoom);
  const unitsPerTile = TILE_SIZE / zoomScale(zoom);
  const x0 = Math.max(0, Math.floor(left / unitsPerTile) - margin);
  const y0 = Math.max(0, Math.floor(top / unitsPerTile) - margin);
  const x1 = Math.min(grid.columns, Math.ceil(right / unitsPerTile) + margin);
  const y1 = Math.min(grid.rows, Math.ceil(bottom / unitsPerTile) + margin);
  return { sheet: sheetIndex, zoom, x0, y0, x1, y1 };
}

/** All tiles of a sheet at a zoom level. */
export function fullTileRange(sheetIndex: number, sheet: Size, zoom: number): TileRange {
  const grid = gridSize(sheet, zoom);
  return { sheet: sheetIndex, zoom, x0: 0, y0: 0, x1: grid.columns, y1: grid.rows };
}

/**
 * Zoom level of the backdrop layer: the finest level at which the whole sheet takes at most
 * 2 by 2 tiles. It is always loaded, so panning and zooming never show an empty sheet.
 */
export function backdropZoomFor(sheet: Size): number {
  const longest = Math.max(sheet.width, sheet.height);
  if (longest <= 0) {
    return MIN_TILE_ZOOM;
  }
  const level = Math.floor(Math.log2((2 * TILE_SIZE) / longest));
  return Math.min(MAX_TILE_ZOOM, Math.max(MIN_TILE_ZOOM, level));
}

/** A tile placed in its layer: position and size in pixels of its zoom level. */
export interface PlacedTile {
  url: string;
  left: number;
  top: number;
  width: number;
  height: number;
  /** Whether another tile follows to the right, or below. */
  hasRight: boolean;
  hasBelow: boolean;
}

/**
 * Overlap in layer pixels so neighbouring tiles share one device pixel at their edge. Without it
 * the anti-aliased edges of two scaled images both let the background through and leave a faint
 * seam. One device pixel of overlap stretches a tile by at most 2 of its 512 pixels, which moves
 * the drawing by at most one device pixel at the far tile edge.
 */
export function seamOverlap(layerScale: number, devicePixelRatio: number): number {
  const devicePxPerTilePx = layerScale * (devicePixelRatio > 0 ? devicePixelRatio : 1);
  return devicePxPerTilePx > 0 ? 1 / devicePxPerTilePx : 0;
}

/** The tiles of `range` with their position in the layer of `range.zoom`. */
export function placeTiles(
  doc: string,
  sheet: Size,
  range: TileRange,
  base: string = tileBaseUrl(),
): PlacedTile[] {
  const grid = gridSize(sheet, range.zoom);
  const tiles: PlacedTile[] = [];
  for (let y = range.y0; y < range.y1; y++) {
    for (let x = range.x0; x < range.x1; x++) {
      const left = x * TILE_SIZE;
      const top = y * TILE_SIZE;
      tiles.push({
        url: tileUrl({ doc, sheet: range.sheet, zoom: range.zoom, x, y }, base),
        left,
        top,
        width: Math.min(TILE_SIZE, grid.widthPx - left),
        height: Math.min(TILE_SIZE, grid.heightPx - top),
        hasRight: x + 1 < grid.columns,
        hasBelow: y + 1 < grid.rows,
      });
    }
  }
  return tiles;
}

/** Stable text key of a document and its tile ranges, to send the interest only on change. */
export function rangesKey(doc: string, ranges: readonly TileRange[]): string {
  const parts = ranges.map(
    (r) =>
      `${String(r.sheet)}/${String(r.zoom)}/${String(r.x0)}-${String(r.x1)}/${String(r.y0)}-${String(r.y1)}`,
  );
  return `${doc}|${parts.join(",")}`;
}
