import { describe, expect, it } from "vitest";
import type { BalloonStyle } from "$lib/ipc/bindings";
import {
  UNITS_PER_MM,
  anchorMove,
  balloonGeometry,
  centerInRect,
  containsPoint,
  groupMoves,
  hitTest,
  leaderStart,
  placeBalloon,
  polygon,
  rectFrom,
  resolveStyle,
  type BalloonGeometry,
  type ResolvedStyle,
} from "./balloons";
import { NO_OVERRIDE } from "$lib/stores/balloon-tools.svelte";
import layoutFixture from "./balloon-layout.fixture.json";

const D24: BalloonStyle = {
  shape: "circle",
  size_mm: 7,
  outline_mm: 0.35,
  outline_color: "#0057B8",
  fill_color: "#FFFFFF",
  text_color: "#000000",
  leader: true,
};

const style = (shape: ResolvedStyle["shape"]): ResolvedStyle =>
  resolveStyle(D24, { ...NO_OVERRIDE, shape });

const box = (shape: BalloonGeometry["shape"]): BalloonGeometry => ({
  shape,
  center: { x: 100, y: 50 },
  width: 20,
  height: 10,
  stroke: 1,
  fontSize: 5,
});

describe("balloon style (D-24)", () => {
  it("applies overrides over the project default and fills missing sizes", () => {
    const s = resolveStyle(
      { ...D24, size_mm: null },
      { ...NO_OVERRIDE, outline_color: "#000000", leader: false },
    );
    expect(s.sizeMm).toBe(7);
    expect(s.outlineColor).toBe("#000000");
    expect(s.leader).toBe(false);
    expect(s.shape).toBe("circle");
    expect(resolveStyle(D24, { ...NO_OVERRIDE, size_mm: 10 }).sizeMm).toBe(10);
  });

  it("sizes balloons in sheet units: 7 mm on the printed sheet", () => {
    const g = balloonGeometry(style("circle"), "12", { x: 0, y: 0 });
    expect(g.width).toBeCloseTo(7 * UNITS_PER_MM, 9);
    expect(g.height).toBeCloseTo(19.843, 3);
    expect(g.stroke).toBeCloseTo(0.35 * UNITS_PER_MM, 9);
  });

  it("shrinks the font of long numbers in a circle and widens boxes instead", () => {
    const short = balloonGeometry(style("circle"), "7", { x: 0, y: 0 });
    const long = balloonGeometry(style("circle"), "1234", { x: 0, y: 0 });
    expect(long.width).toBe(short.width);
    expect(long.fontSize).toBeLessThan(short.fontSize);
    const rect = balloonGeometry(style("rectangle"), "1234", { x: 0, y: 0 });
    expect(rect.fontSize).toBe(short.fontSize);
    expect(rect.width).toBeGreaterThan(rect.height);
    const flag = balloonGeometry(style("flag"), "1", { x: 0, y: 0 });
    expect(flag.width).toBeGreaterThanOrEqual(flag.height * 1.5);
  });
});

interface LayoutCase {
  style: BalloonStyle;
  text: string;
  layout: { width: number; height: number; stroke: number; font_size: number };
}

describe("one balloon layout for viewport and ballooned PDF (D-24)", () => {
  // Written by the Rust test `balloon_layout` from `BalloonStyle::layout`, which the PDF
  // export uses.
  const cases = layoutFixture as LayoutCase[];

  it("has cases for every shape", () => {
    expect(new Set(cases.map((c) => c.style.shape))).toEqual(
      new Set(["circle", "rectangle", "flag"]),
    );
  });

  it.each(cases.map((c) => [c.style.shape, c.style.size_mm, c.text, c] as const))(
    "%s %s mm %s matches Rust",
    (_shape, _size, _text, c) => {
      const g = balloonGeometry(resolveStyle(c.style, NO_OVERRIDE), c.text, { x: 0, y: 0 });
      expect(g.width).toBeCloseTo(c.layout.width, 9);
      expect(g.height).toBeCloseTo(c.layout.height, 9);
      expect(g.stroke).toBeCloseTo(c.layout.stroke, 9);
      expect(g.fontSize).toBeCloseTo(c.layout.font_size, 9);
    },
  );
});

describe("shapes and leaders (as overlay.rs)", () => {
  it("cuts the flag to a point at the middle of its left edge", () => {
    expect(polygon(box("flag"))).toEqual([
      { x: 95, y: 45 },
      { x: 110, y: 45 },
      { x: 110, y: 55 },
      { x: 95, y: 55 },
      { x: 90, y: 50 },
    ]);
  });

  it("starts the leader where the ray to the anchor leaves the shape", () => {
    const circle = { ...box("circle"), width: 10 };
    expect(leaderStart(circle, { x: 120, y: 50 })).toEqual({ x: 105, y: 50 });
    expect(leaderStart(box("rectangle"), { x: 100, y: 80 })).toEqual({ x: 100, y: 55 });
    const flag = leaderStart(box("flag"), { x: 70, y: 50 });
    expect(flag?.x).toBeCloseTo(90, 9);
    expect(flag?.y).toBeCloseTo(50, 9);
  });

  it("draws no leader when the anchor is inside the shape", () => {
    expect(leaderStart(box("rectangle"), { x: 102, y: 51 })).toBeNull();
    expect(leaderStart(box("circle"), { x: 100, y: 50 })).toBeNull();
  });
});

describe("hit testing", () => {
  it("hits inside the shape and within the tolerance only", () => {
    const circle = { ...box("circle"), width: 10 };
    expect(containsPoint(circle, { x: 104, y: 50 })).toBe(true);
    expect(containsPoint(circle, { x: 106, y: 50 })).toBe(false);
    expect(containsPoint(circle, { x: 106, y: 50 }, 2)).toBe(true);
    // The corner of the box around a circle is outside the circle.
    expect(containsPoint(circle, { x: 104.5, y: 54.5 })).toBe(false);
  });

  it("follows the pointed end of a flag", () => {
    const flag = box("flag");
    expect(containsPoint(flag, { x: 91, y: 50 })).toBe(true);
    expect(containsPoint(flag, { x: 91, y: 46 })).toBe(false);
    expect(containsPoint(flag, { x: 108, y: 46 })).toBe(true);
  });

  it("returns the topmost balloon, the last one drawn", () => {
    const a = box("rectangle");
    const b = { ...box("rectangle"), center: { x: 105, y: 50 } };
    expect(hitTest([a, b], { x: 103, y: 50 })).toBe(1);
    expect(hitTest([a, b], { x: 92, y: 50 })).toBe(0);
    expect(hitTest([a, b], { x: 300, y: 50 })).toBe(-1);
  });

  it("box selects balloons by their center", () => {
    const rect = rectFrom({ x: 120, y: 60 }, { x: 95, y: 40 });
    expect(rect).toEqual({ x: 95, y: 40, width: 25, height: 20 });
    expect(centerInRect(box("circle"), rect)).toBe(true);
    expect(centerInRect(box("circle"), { x: 101, y: 0, width: 50, height: 100 })).toBe(false);
  });
});

describe("drag math", () => {
  const a = { id: "a", position: { x: 10, y: 20 }, anchor: { x: 1, y: 2 } };
  const b = { id: "b", position: { x: 30, y: 40 }, anchor: { x: 3, y: 4 } };
  const c = { id: "c", position: { x: 50, y: 60 }, anchor: { x: 5, y: 6 } };
  const balloons = [a, b, c];

  it("moves every selected balloon by the same delta and keeps the anchors", () => {
    expect(groupMoves(balloons, new Set(["a", "c"]), { x: 5, y: -2 })).toEqual([
      { id: "a", position: { x: 15, y: 18 }, anchor: null },
      { id: "c", position: { x: 55, y: 58 }, anchor: null },
    ]);
  });

  it("sends nothing for a zero move", () => {
    expect(groupMoves(balloons, new Set(["a"]), { x: 0, y: 0 })).toEqual([]);
    expect(anchorMove(a, { x: 1, y: 2 })).toEqual([]);
  });

  it("moves only the anchor when dragging the handle", () => {
    expect(anchorMove(b, { x: 7, y: 8 })).toEqual([
      { id: "b", position: { x: 30, y: 40 }, anchor: { x: 7, y: 8 } },
    ]);
  });
});

describe("placement of a new balloon (FR-BAL-01)", () => {
  const sheet = { width: 800, height: 600 };
  const size = 20;

  it("puts the balloon up and right of a clicked point", () => {
    expect(placeBalloon({ x: 100, y: 100 }, sheet, size)).toEqual({
      anchor: { x: 100, y: 100 },
      position: { x: 120, y: 80 },
    });
  });

  it("anchors a region at its corner on the balloon side", () => {
    const p = placeBalloon({ x: 100, y: 100, width: 40, height: 10 }, sheet, size);
    expect(p.anchor).toEqual({ x: 140, y: 100 });
    expect(p.position).toEqual({ x: 160, y: 80 });
  });

  it("flips to the left and below at the sheet edges and stays on the sheet", () => {
    expect(placeBalloon({ x: 790, y: 5 }, sheet, size)).toEqual({
      anchor: { x: 790, y: 5 },
      position: { x: 770, y: 25 },
    });
    const corner = placeBalloon({ x: 5, y: 5 }, { width: 15, height: 15 }, size);
    expect(corner.position.x).toBeGreaterThanOrEqual(0);
    expect(corner.position.y).toBeLessThanOrEqual(15);
  });
});
