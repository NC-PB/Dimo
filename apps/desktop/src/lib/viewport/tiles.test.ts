import { describe, expect, it } from "vitest";
import type { TileAddress } from "$lib/ipc/bindings";
import { tileUrl } from "./tiles";

const DOC = "635a89735fc1a305a99c3d42e394785d0ca00e2d82f2af1d5bdba8b66fca5c82";
const ADDRESS: TileAddress = { doc: DOC, sheet: 0, zoom: -2, x: 3, y: 1 };

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
