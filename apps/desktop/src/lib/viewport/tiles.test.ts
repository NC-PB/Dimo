import { describe, expect, it } from "vitest";
import { MAX_TILE_ZOOM, MIN_TILE_ZOOM, type TileAddress } from "$lib/ipc/bindings";
import {
  backdropZoomFor,
  fullTileRange,
  gridSize,
  placeTiles,
  rangesKey,
  seamOverlap,
  tileRangeFor,
  tileUrl,
  tileZoomFor,
} from "./tiles";
import { visibleSheetRect } from "./view-math";

const DOC = "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";
const ADDRESS: TileAddress = { doc: DOC, sheet: 0, zoom: -2, x: 3, y: 1 };
const BASE = "dimo://localhost/";

/** A4 landscape and A0 portrait in sheet units. */
const A4 = { width: 841.89, height: 595.276 };
const A0 = { width: 2383.94, height: 3370.39 };

describe("tile URLs (T0.7)", () => {
  it("uses the custom scheme form of macOS and Linux", () => {
    expect(tileUrl(ADDRESS, "dimo://localhost/")).toBe(`dimo://localhost/tile/${DOC}/0/-2/3/1`);
  });

  it("uses the localhost form of Windows", () => {
    expect(tileUrl(ADDRESS, "http://dimo.localhost/")).toBe(
      `http://dimo.localhost/tile/${DOC}/0/-2/3/1`,
    );
  });

  it("adds a missing slash after the base", () => {
    expect(tileUrl(ADDRESS, "dimo://localhost")).toBe(`dimo://localhost/tile/${DOC}/0/-2/3/1`);
  });
});

describe("tile zoom level for a view scale (T0.8)", () => {
  it("picks the coarsest level that is not upscaled", () => {
    // 1 device px per unit: level 0. 1.5: level 1 (2 px per unit, shown at 75 %).
    expect(tileZoomFor(1, 1)).toBe(0);
    expect(tileZoomFor(1.5, 1)).toBe(1);
    expect(tileZoomFor(0.3, 1)).toBe(-1);
    // Retina: twice the device pixels.
    expect(tileZoomFor(1, 2)).toBe(1);
    expect(tileZoomFor(0.75, 2)).toBe(1);
  });

  it("tolerates a tiny upscale instead of switching levels", () => {
    expect(tileZoomFor(1.01, 1)).toBe(0);
    expect(tileZoomFor(1.05, 1)).toBe(1);
  });

  it("stays within the rendered levels", () => {
    expect(tileZoomFor(1e-6, 1)).toBe(MIN_TILE_ZOOM);
    expect(tileZoomFor(1e6, 2)).toBe(MAX_TILE_ZOOM);
  });
});

describe("tile grid and visible range (T0.8)", () => {
  it("matches TileGrid in dimo-pdf", () => {
    // 841.89 * 2 = 1683.78 -> 1684 px, 4 columns; 595.276 * 2 = 1190.55 -> 1191 px, 3 rows.
    expect(gridSize(A4, 1)).toEqual({ widthPx: 1684, heightPx: 1191, columns: 4, rows: 3 });
    expect(gridSize(A4, -4)).toEqual({ widthPx: 53, heightPx: 38, columns: 1, rows: 1 });
  });

  it("selects the tiles a viewport shows", () => {
    // Zoom 1 (2 px per unit): one tile covers 256 units. View shows x 300..700, y 0..250.
    const view = { scale: 2, tx: -600, ty: 0 };
    const area = visibleSheetRect(view, { width: 800, height: 500 });
    expect(tileRangeFor(0, A4, 1, area)).toEqual({ sheet: 0, zoom: 1, x0: 1, y0: 0, x1: 3, y1: 1 });
  });

  it("adds a prefetch margin and clips it to the grid", () => {
    const area = { x: 300, y: 0, width: 400, height: 250 };
    expect(tileRangeFor(2, A4, 1, area, 1)).toEqual({
      sheet: 2,
      zoom: 1,
      x0: 0,
      y0: 0,
      x1: 4,
      y1: 2,
    });
  });

  it("returns nothing when the view is outside the sheet", () => {
    expect(tileRangeFor(0, A4, 1, { x: 900, y: 0, width: 100, height: 100 })).toBeNull();
    expect(tileRangeFor(0, A4, 1, { x: -500, y: -500, width: 100, height: 100 })).toBeNull();
  });

  it("covers the whole sheet when zoomed out", () => {
    const area = { x: -1000, y: -1000, width: 5000, height: 5000 };
    expect(tileRangeFor(0, A4, 1, area)).toEqual(fullTileRange(0, A4, 1));
  });

  it("uses a backdrop of at most 2 by 2 tiles", () => {
    for (const sheet of [A4, A0, { width: 100, height: 50 }]) {
      const range = fullTileRange(0, sheet, backdropZoomFor(sheet));
      expect(range.x1).toBeLessThanOrEqual(2);
      expect(range.y1).toBeLessThanOrEqual(2);
    }
    expect(backdropZoomFor(A4)).toBe(0);
    expect(backdropZoomFor(A0)).toBe(-2);
  });

  it("places tiles on the pixel grid of their level, edge tiles cut", () => {
    const tiles = placeTiles(DOC, A4, { sheet: 0, zoom: 1, x0: 2, y0: 2, x1: 4, y1: 3 }, BASE);
    expect(tiles).toEqual([
      {
        url: `${BASE}tile/${DOC}/0/1/2/2`,
        left: 1024,
        top: 1024,
        width: 512,
        height: 167,
        hasRight: true,
        hasBelow: false,
      },
      {
        url: `${BASE}tile/${DOC}/0/1/3/2`,
        left: 1536,
        top: 1024,
        width: 148,
        height: 167,
        hasRight: false,
        hasBelow: false,
      },
    ]);
  });

  it("overlaps tiles by one device pixel", () => {
    // Layer shown at 2/3 on a display with 1 device px per CSS px: 1.5 tile px = 1 device px.
    expect(seamOverlap(2 / 3, 1)).toBeCloseTo(1.5, 12);
    expect(seamOverlap(0.5, 2)).toBe(1);
  });

  it("keys ranges so equal interests compare equal", () => {
    const a = [fullTileRange(0, A4, 0)];
    expect(rangesKey(DOC, a)).toBe(rangesKey(DOC, [fullTileRange(0, A4, 0)]));
    expect(rangesKey(DOC, a)).not.toBe(rangesKey(DOC, [fullTileRange(1, A4, 0)]));
  });
});
