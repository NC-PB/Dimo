import { describe, expect, it } from "vitest";
import {
  axisLabels,
  defaultGrid,
  gridLines,
  isDrawn,
  labelPositions,
  labelsText,
  letters,
  parseLabels,
  rectBetween,
  schemeOf,
  withAxis,
} from "./zone-grid";

describe("zone grid editor (T2.7, M2 decision 1)", () => {
  it("counts letters like spreadsheet columns", () => {
    expect([1, 2, 26, 27, 28, 52, 53].map(letters)).toEqual([
      "A",
      "B",
      "Z",
      "AA",
      "AB",
      "AZ",
      "BA",
    ]);
  });

  it("labels an axis as printed, also counted from the far end", () => {
    expect(axisLabels(4, { kind: "numbers", reversed: false })).toEqual(["1", "2", "3", "4"]);
    expect(axisLabels(4, { kind: "numbers", reversed: true })).toEqual(["4", "3", "2", "1"]);
    expect(axisLabels(3, { kind: "letters", reversed: false })).toEqual(["A", "B", "C"]);
    expect(axisLabels(3, { kind: "letters", reversed: true })).toEqual(["C", "B", "A"]);
    expect(axisLabels(0, { kind: "numbers", reversed: false })).toEqual(["1"]);
    expect(axisLabels(500, { kind: "numbers", reversed: false })).toHaveLength(100);
  });

  it("recognizes the scheme of labels, or none for labels typed by hand", () => {
    expect(schemeOf(["C", "B", "A"])).toEqual({ kind: "letters", reversed: true });
    expect(schemeOf(["1", "2"])).toEqual({ kind: "numbers", reversed: false });
    expect(schemeOf(["X", "Y"])).toBeNull();
    expect(schemeOf(["F", "E", "D"])).toBeNull();
  });

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

  it("starts with 8 by 6 zones on landscape sheets and 6 by 8 on portrait ones", () => {
    const landscape = defaultGrid({ width: 1190, height: 842 });
    expect(landscape.column_labels).toEqual(["1", "2", "3", "4", "5", "6", "7", "8"]);
    expect(landscape.row_labels).toEqual(["A", "B", "C", "D", "E", "F"]);
    expect(landscape.frame.origin.x).toBeCloseTo(28.35, 2);
    expect(landscape.frame.size.width).toBeCloseTo(1190 - 2 * 28.35, 1);
    const portrait = defaultGrid({ width: 595, height: 842 });
    expect(portrait.column_labels).toHaveLength(6);
    expect(portrait.row_labels).toHaveLength(8);
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
    expect(withAxis(grid, "rows", 3, { kind: "letters", reversed: true }).row_labels).toEqual([
      "C",
      "B",
      "A",
    ]);
  });
});
