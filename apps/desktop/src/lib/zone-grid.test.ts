import { describe, expect, it } from "vitest";
import {
  gridLines,
  isDrawn,
  labelPositions,
  labelsText,
  parseLabels,
  rectBetween,
} from "./zone-grid";

// Default grids, axis labels and label schemes are Rust's (`dimo_core::zones`, T2.7a) and
// tested there.
describe("zone grid editor (T2.7, M2 decision 1)", () => {
  it("reads typed labels separated by commas or spaces", () => {
    expect(parseLabels(" A, B ,C  D;E,, ")).toEqual(["A", "B", "C", "D", "E"]);
    expect(parseLabels(" , ")).toEqual([]);
    expect(labelsText(["1", "2"])).toBe("1, 2");
  });

  it("spans a rectangle between two points in any direction", () => {
    expect(rectBetween({ x: 50, y: 10 }, { x: 20, y: 40 })).toEqual({
      origin: { x: 20, y: 10 },
      size: { width: 30, height: 30 },
    });
    expect(isDrawn(rectBetween({ x: 0, y: 0 }, { x: 5, y: 40 }))).toBe(false);
    expect(isDrawn(rectBetween({ x: 0, y: 0 }, { x: 15, y: 40 }))).toBe(true);
  });

  it("places inner lines and labels evenly in the frame", () => {
    const grid = {
      frame: { origin: { x: 10, y: 20 }, size: { width: 300, height: 100 } },
      column_labels: ["1", "2", "3"],
      row_labels: ["A", "B"],
    };
    expect(gridLines(grid)).toEqual({ xs: [110, 210], ys: [70] });
    expect(labelPositions(grid)).toEqual([
      { text: "1", at: { x: 60, y: 20 }, axis: "columns" },
      { text: "2", at: { x: 160, y: 20 }, axis: "columns" },
      { text: "3", at: { x: 260, y: 20 }, axis: "columns" },
      { text: "A", at: { x: 10, y: 45 }, axis: "rows" },
      { text: "B", at: { x: 10, y: 95 }, axis: "rows" },
    ]);
  });
});
