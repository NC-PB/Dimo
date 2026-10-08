import { describe, expect, it } from "vitest";
import { DUMMY_STYLE, dummyBalloons, leaderLine } from "./dummy-balloons";
import { frameStats } from "./pan-check";

const A4 = { width: 841.89, height: 595.276 };

describe("dummy balloons (T0.8, dev only)", () => {
  it("are deterministic, numbered and inside the sheet", () => {
    const a = dummyBalloons(A4, 500, 1);
    expect(a).toEqual(dummyBalloons(A4, 500, 1));
    expect(a).toHaveLength(500);
    expect(a.map((b) => b.number)).toEqual(Array.from({ length: 500 }, (_, i) => i + 1));
    for (const b of a) {
      expect(b.center.x).toBeGreaterThanOrEqual(DUMMY_STYLE.radius);
      expect(b.center.x).toBeLessThanOrEqual(A4.width - DUMMY_STYLE.radius);
      expect(b.center.y).toBeGreaterThanOrEqual(DUMMY_STYLE.radius);
      expect(b.center.y).toBeLessThanOrEqual(A4.height - DUMMY_STYLE.radius);
    }
  });

  it("use given anchors, one balloon each", () => {
    const anchors = [
      { x: 237.51, y: 166.1 },
      { x: 413.62, y: 158.96 },
    ];
    const balloons = dummyBalloons(A4, 500, 1, anchors);
    expect(balloons.map((b) => b.anchor)).toEqual(anchors);
    const [first] = balloons;
    if (!first) {
      throw new Error("no balloon");
    }
    const leader = leaderLine(first);
    expect(leader?.from).toEqual(anchors[0]);
    // The leader ends on the balloon outline.
    const to = leader?.to ?? first.center;
    const c = first.center;
    expect(Math.hypot(to.x - c.x, to.y - c.y)).toBeCloseTo(DUMMY_STYLE.radius, 9);
  });

  it("have the 7 mm D-24 size in sheet units", () => {
    expect(DUMMY_STYLE.radius * 2).toBeCloseTo((7 / 25.4) * 72, 9);
  });
});

describe("frame statistics (NFR-PERF-02)", () => {
  it("computes mean, nearest rank p95 and max", () => {
    const times = Array.from({ length: 100 }, (_, i) => i + 1);
    expect(frameStats(times)).toEqual({ frames: 100, meanMs: 50.5, p95Ms: 95, maxMs: 100 });
  });

  it("handles no frames", () => {
    expect(frameStats([])).toEqual({ frames: 0, meanMs: 0, p95Ms: 0, maxMs: 0 });
  });
});
