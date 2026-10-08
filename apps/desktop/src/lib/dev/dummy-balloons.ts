/**
 * Dummy balloons for viewport performance and alignment checks (T0.8). Development only, not a
 * feature: real balloons, their numbering and style come from Rust (rule 2, D-24).
 */

import type { Point, Size } from "$lib/viewport/view-math";

/** Sheet units per millimetre (1 unit = 1/72 inch). */
export const UNITS_PER_MM = 72 / 25.4;

/**
 * Balloon look of D-24 in sheet units: circle of 7 mm, 0.35 mm blue outline, white fill, bold
 * black number, leader line. The dummies use the plain 7 mm size; scaling with the sheet size is
 * decided by the real balloon style in Rust.
 */
export const DUMMY_STYLE = {
  radius: 3.5 * UNITS_PER_MM,
  stroke: 0.35 * UNITS_PER_MM,
  color: "#0057B8",
  fontSize: 3.5 * UNITS_PER_MM,
} as const;

export interface DummyBalloon {
  /** Balloon number, starting at 1. */
  number: number;
  /** Balloon center in sheet units. */
  center: Point;
  /** End of the leader line on the drawing, in sheet units. */
  anchor: Point;
}

/** Small deterministic pseudo random generator (mulberry32), so every run shows the same set. */
function random(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * `count` dummy balloons on `sheet`. With `anchors`, one balloon per anchor (up to `count`),
 * placed up and to the right of it; otherwise anchors are spread over the sheet by a seeded
 * generator. Balloon centers stay inside the sheet.
 */
export function dummyBalloons(
  sheet: Size,
  count: number,
  seed: number,
  anchors: readonly Point[] = [],
): DummyBalloon[] {
  const r = DUMMY_STYLE.radius;
  const next = random(seed);
  const clamp = (v: number, max: number) => Math.min(max - r, Math.max(r, v));
  const n = anchors.length > 0 ? Math.min(count, anchors.length) : count;
  const balloons: DummyBalloon[] = [];
  for (let i = 0; i < n; i++) {
    const anchor = anchors[i] ?? { x: next() * sheet.width, y: next() * sheet.height };
    const angle = anchors.length > 0 ? -Math.PI / 4 : next() * 2 * Math.PI;
    const distance = 3 * r;
    balloons.push({
      number: i + 1,
      anchor,
      center: {
        x: clamp(anchor.x + Math.cos(angle) * distance, sheet.width),
        y: clamp(anchor.y + Math.sin(angle) * distance, sheet.height),
      },
    });
  }
  return balloons;
}

/**
 * Leader line from the anchor to the balloon outline (not into the circle), or `null` when the
 * anchor lies inside the balloon.
 */
export function leaderLine(balloon: DummyBalloon): { from: Point; to: Point } | null {
  const dx = balloon.anchor.x - balloon.center.x;
  const dy = balloon.anchor.y - balloon.center.y;
  const length = Math.hypot(dx, dy);
  if (length <= DUMMY_STYLE.radius) {
    return null;
  }
  const k = DUMMY_STYLE.radius / length;
  return {
    from: balloon.anchor,
    to: { x: balloon.center.x + dx * k, y: balloon.center.y + dy * k },
  };
}
