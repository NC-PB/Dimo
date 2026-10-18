import { describe, expect, it } from "vitest";
import type { Homography, SheetCorners } from "$lib/ipc/bindings";
import {
  DEFAULT_EDIT,
  decimalText,
  imagePoint,
  isDefaultEdit,
  mapPoint,
  pageSize,
  renameCorners,
  standardPaper,
  trimOf,
  trimRect,
  turnedBy,
} from "./image-edit";

const pt = (x: string, y: string) => ({ x, y });

describe("image editor helpers (R.20)", () => {
  it("writes numbers as plain decimals", () => {
    expect(decimalText(1.5, 2)).toBe("1.5");
    expect(decimalText(-0.001, 2)).toBe("0");
    expect(decimalText(10, 2)).toBe("10");
    expect(decimalText(3, 0)).toBe("3");
    expect(decimalText(Number.NaN, 2)).toBe("0");
  });

  it("turns and renames corners with the quarter turns", () => {
    expect(turnedBy("deg0", -1)).toBe("deg270");
    expect(turnedBy("deg270", 1)).toBe("deg0");
    const corners: SheetCorners = {
      top_left: pt("0", "0"),
      top_right: pt("10", "0"),
      bottom_right: pt("10", "20"),
      bottom_left: pt("0", "20"),
    };
    const turned = renameCorners(corners, "deg0", "deg90");
    expect(turned.top_left).toEqual(corners.bottom_left);
    expect(turned.top_right).toEqual(corners.top_left);
    expect(renameCorners(turned, "deg90", "deg0")).toEqual(corners);
    expect(pageSize(corners)).toEqual({ width: 10, height: 20 });
  });

  it("maps points with the matrix Rust sends", () => {
    // A clockwise quarter turn of a page 20 high.
    const turn: Homography = [0, -1, 20, 1, 0, 0, 0, 0, 1];
    expect(mapPoint(turn, { x: 0, y: 0 })).toEqual({ x: 20, y: 0 });
    expect(imagePoint({ x: -3, y: 12.345 }, { width: 10, height: 20 })).toEqual(pt("0", "12.35"));
  });

  it("turns a rectangle into a trim and back", () => {
    const size = { width: 200, height: 100 };
    expect(trimOf({ left: 0, top: 0, right: 200, bottom: 100 }, size)).toBeNull();
    const trim = trimOf({ left: 150, top: 80, right: 20, bottom: 10 }, size);
    expect(trim).toEqual({ left: "0.1", top: "0.1", right: "0.75", bottom: "0.8" });
    expect(trimRect(trim, size)).toEqual({ left: 20, top: 10, right: 150, bottom: 80 });
    const thin = trimOf({ left: 50, top: 0, right: 50, bottom: 100 }, size);
    expect(thin?.right).toBe("0.27");
  });

  it("picks the paper orientation from the page", () => {
    expect(standardPaper("a4", { width: 300, height: 200 })).toEqual({
      format: "standard",
      size: "a4",
      orientation: "landscape",
    });
    expect(isDefaultEdit({ ...DEFAULT_EDIT, fine_angle: "0.00" })).toBe(true);
  });
});
