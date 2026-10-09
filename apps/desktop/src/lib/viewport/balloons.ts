/**
 * Balloon geometry of the viewport overlay (T1.6, FR-BAL-03, D-24). Pure functions in sheet
 * space (rule 4): shapes, leader lines, hit testing, box selection, drag math and the offset
 * placement of a new balloon. Screen geometry only, floats are fine (rule 5).
 *
 * The shapes follow the ballooned PDF writer (`dimo-pdf` `overlay.rs`): a box of `width` x
 * `height` around the center; the circle is the inscribed ellipse, the rectangle is the box,
 * the flag is the box with its left end cut to a point (cut length `height / 2`, at most
 * `width / 2`). The leader runs from where the ray from the center to the anchor leaves the
 * shape, and is left out when the anchor lies inside.
 *
 * Balloon sizes follow the layout rule of `dimo_core::BalloonMetrics` with the numbers of the
 * generated `BALLOON_METRICS`, the same values the ballooned PDF uses. `balloons.test.ts`
 * checks `balloonGeometry` against sizes computed by Rust (`balloon-layout.fixture.json`).
 */

import {
  BALLOON_METRICS,
  type BalloonMove,
  type BalloonShape,
  type BalloonStyle,
  type BalloonStyleOverride,
} from "$lib/ipc/bindings";
import type { Point, Rect, Size } from "./view-math";

/** Sheet units per millimeter (1 unit = 1/72 inch), from Rust (`dimo_core::UNITS_PER_MM`). */
export const UNITS_PER_MM = BALLOON_METRICS.units_per_mm;

/** D-24 defaults, used when a style value is missing. */
export const DEFAULT_SIZE_MM = 7;
export const DEFAULT_OUTLINE_MM = 0.35;

/** A balloon style with every value present. */
export interface ResolvedStyle {
  shape: BalloonShape;
  sizeMm: number;
  outlineMm: number;
  outlineColor: string;
  fillColor: string;
  textColor: string;
  leader: boolean;
}

/** The project default with the per balloon overrides applied (as `BalloonStyle::with_override`). */
export function resolveStyle(base: BalloonStyle, o: BalloonStyleOverride): ResolvedStyle {
  return {
    shape: o.shape ?? base.shape,
    sizeMm: positive(o.size_mm) ?? positive(base.size_mm) ?? DEFAULT_SIZE_MM,
    outlineMm: positive(o.outline_mm) ?? positive(base.outline_mm) ?? DEFAULT_OUTLINE_MM,
    outlineColor: o.outline_color ?? base.outline_color,
    fillColor: o.fill_color ?? base.fill_color,
    textColor: o.text_color ?? base.text_color,
    leader: o.leader ?? base.leader,
  };
}

function positive(value: number | null): number | null {
  return value !== null && Number.isFinite(value) && value > 0 ? value : null;
}

/** Outline and text of one balloon in sheet units. */
export interface BalloonGeometry {
  shape: BalloonShape;
  center: Point;
  width: number;
  height: number;
  /** Outline width. */
  stroke: number;
  /** Font size (em) of the number. */
  fontSize: number;
}

/** Estimated width of `text` in em (`BalloonMetrics::text_width_em`). */
export function textWidthEm(text: string): number {
  let ems = 0;
  for (const c of text) {
    ems += c === "." || c === "," ? BALLOON_METRICS.narrow_em : BALLOON_METRICS.digit_em;
  }
  return ems;
}

/**
 * Geometry of a balloon showing `text` at `center` (`BalloonMetrics::layout` in Rust). The
 * height is the style size. A circle keeps its diameter and shrinks the font for long numbers;
 * a rectangle or flag grows wider instead, so the number keeps its size.
 */
export function balloonGeometry(
  style: ResolvedStyle,
  text: string,
  center: Point,
): BalloonGeometry {
  const k = BALLOON_METRICS;
  const height = style.sizeMm * k.units_per_mm;
  const stroke = style.outlineMm * k.units_per_mm;
  let fontSize = height * k.font_share;
  const textWidth = textWidthEm(text) * fontSize;
  const padded = textWidth + 2 * k.box_padding_share * height;
  let width: number;
  switch (style.shape) {
    case "circle": {
      const room = height * k.circle_text_share;
      if (textWidth > room) {
        fontSize *= room / textWidth;
      }
      width = height;
      break;
    }
    case "rectangle":
      width = Math.max(height, padded);
      break;
    case "flag":
      width = Math.max(height * k.flag_min_width_share, padded + height * k.flag_point_share);
      break;
  }
  return { shape: style.shape, center, width, height, stroke, fontSize };
}

/** Corners of a rectangle or flag, clockwise from the top left (as `overlay.rs`). */
export function polygon(g: BalloonGeometry): Point[] {
  const hw = g.width / 2;
  const hh = g.height / 2;
  const left = g.center.x - hw;
  const right = g.center.x + hw;
  const top = g.center.y - hh;
  const bottom = g.center.y + hh;
  if (g.shape === "flag") {
    const cut = Math.min(hh, hw);
    return [
      { x: left + cut, y: top },
      { x: right, y: top },
      { x: right, y: bottom },
      { x: left + cut, y: bottom },
      { x: left, y: g.center.y },
    ];
  }
  return [
    { x: left, y: top },
    { x: right, y: top },
    { x: right, y: bottom },
    { x: left, y: bottom },
  ];
}

/** SVG `points` of a polygon. */
export function svgPoints(points: readonly Point[]): string {
  return points.map((p) => `${String(p.x)},${String(p.y)}`).join(" ");
}

/** The same shape grown by `by` on every side, for the selection halo. */
export function grown(g: BalloonGeometry, by: number): BalloonGeometry {
  return { ...g, width: g.width + 2 * by, height: g.height + 2 * by };
}

/** Smallest `t > 0` where `origin + t * dir` hits the segment `p`-`q`. */
function raySegment(origin: Point, dir: Point, p: Point, q: Point): number | null {
  const ex = q.x - p.x;
  const ey = q.y - p.y;
  const denom = dir.x * ey - dir.y * ex;
  if (Math.abs(denom) < 1e-12) {
    return null;
  }
  const wx = p.x - origin.x;
  const wy = p.y - origin.y;
  const t = (wx * ey - wy * ex) / denom;
  const s = (wx * dir.y - wy * dir.x) / denom;
  return t > 0 && s >= -1e-9 && s <= 1 + 1e-9 ? t : null;
}

/**
 * Where the leader starts: the point where the ray from the center to `anchor` leaves the
 * shape, or `null` if the anchor is inside the shape (no leader is drawn then).
 */
export function leaderStart(g: BalloonGeometry, anchor: Point): Point | null {
  const dx = anchor.x - g.center.x;
  const dy = anchor.y - g.center.y;
  if (dx === 0 && dy === 0) {
    return null;
  }
  let t: number;
  if (g.shape === "circle") {
    const a = g.width / 2;
    const b = g.height / 2;
    t = 1 / Math.sqrt((dx / a) ** 2 + (dy / b) ** 2);
  } else {
    const corners = polygon(g);
    t = Infinity;
    corners.forEach((p, i) => {
      const q = corners[(i + 1) % corners.length] ?? p;
      const hit = raySegment(g.center, { x: dx, y: dy }, p, q);
      if (hit !== null) {
        t = Math.min(t, hit);
      }
    });
  }
  return Number.isFinite(t) && t < 1 ? { x: g.center.x + t * dx, y: g.center.y + t * dy } : null;
}

/** True if `p` lies inside the shape grown by `tolerance` (sheet units). */
export function containsPoint(g: BalloonGeometry, p: Point, tolerance = 0): boolean {
  const dx = p.x - g.center.x;
  const dy = p.y - g.center.y;
  const hw = g.width / 2 + tolerance;
  const hh = g.height / 2 + tolerance;
  if (g.shape === "circle") {
    return (dx / hw) ** 2 + (dy / hh) ** 2 <= 1;
  }
  if (Math.abs(dx) > hw || Math.abs(dy) > hh) {
    return false;
  }
  if (g.shape === "flag") {
    // Left of the cut start the flag narrows to its point.
    const cut = Math.min(g.height / 2, g.width / 2);
    const fromLeft = dx + g.width / 2;
    if (fromLeft < cut) {
      return Math.abs(dy) <= fromLeft * (g.height / 2 / cut) + tolerance * Math.SQRT2;
    }
  }
  return true;
}

/**
 * Index of the topmost balloon at `p`: the last one in drawing order whose shape (grown by
 * `tolerance`) contains it. `-1` if none.
 */
export function hitTest(shapes: readonly BalloonGeometry[], p: Point, tolerance = 0): number {
  for (let i = shapes.length - 1; i >= 0; i--) {
    const shape = shapes[i];
    if (shape && containsPoint(shape, p, tolerance)) {
      return i;
    }
  }
  return -1;
}

/** True if `p` is within `tolerance` of `target`. */
export function nearPoint(p: Point, target: Point, tolerance: number): boolean {
  return Math.hypot(p.x - target.x, p.y - target.y) <= tolerance;
}

/** The rectangle spanned by two corners, with positive width and height. */
export function rectFrom(a: Point, b: Point): Rect {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    width: Math.abs(b.x - a.x),
    height: Math.abs(b.y - a.y),
  };
}

/** True if the balloon center lies inside `rect` (box selection). */
export function centerInRect(g: BalloonGeometry, rect: Rect): boolean {
  return (
    g.center.x >= rect.x &&
    g.center.x <= rect.x + rect.width &&
    g.center.y >= rect.y &&
    g.center.y <= rect.y + rect.height
  );
}

/** A balloon as the move math needs it. */
export interface Placed {
  id: string;
  position: Point;
  anchor: Point;
}

/**
 * One `MoveBalloons` entry per balloon in `ids`, moved by `delta`. Leader anchors stay on the
 * drawing (`anchor: null`), so the leader follows the balloon. Empty for a zero move.
 */
export function groupMoves(
  balloons: readonly Placed[],
  ids: ReadonlySet<string>,
  delta: Point,
): BalloonMove[] {
  if (delta.x === 0 && delta.y === 0) {
    return [];
  }
  return balloons
    .filter((b) => ids.has(b.id))
    .map((b) => ({
      id: b.id,
      position: { x: b.position.x + delta.x, y: b.position.y + delta.y },
      anchor: null,
    }));
}

/** The `MoveBalloons` entry that puts the leader anchor of one balloon at `anchor`. */
export function anchorMove(balloon: Placed, anchor: Point): BalloonMove[] {
  if (anchor.x === balloon.anchor.x && anchor.y === balloon.anchor.y) {
    return [];
  }
  return [{ id: balloon.id, position: balloon.position, anchor }];
}

/** Where a new balloon goes: its center and the leader anchor on the drawing. */
export interface Placement {
  position: Point;
  anchor: Point;
}

/** Signs of a diagonal direction in sheet space, each 1 or -1. */
export interface Diagonal {
  x: 1 | -1;
  y: 1 | -1;
}

/** Up and to the right on an unrotated sheet. */
export const UP_RIGHT: Diagonal = { x: 1, y: -1 };

/**
 * Simple offset placement of a new balloon (FR-BAL-01; overlap avoidance is M4, FR-BAL-08).
 *
 * The balloon goes diagonally away from the target in direction `towards` (by default up and
 * to the right; the viewport passes the sheet direction that is up and right on screen for a
 * rotated sheet), its center one balloon height away from the anchor in x and y. For a region
 * the anchor is the region corner on that side, so the leader does not cross the callout
 * text. Where the balloon would leave the sheet the side flips. The center always stays on
 * the sheet.
 */
export function placeBalloon(
  target: Point | Rect,
  sheet: Size,
  size: number,
  towards: Diagonal = UP_RIGHT,
): Placement {
  const region = "width" in target ? target : null;
  const point = region
    ? { x: region.x + region.width / 2, y: region.y + region.height / 2 }
    : target;
  const halfW = region ? region.width / 2 : 0;
  const halfH = region ? region.height / 2 : 0;
  const offset = size;
  const r = size / 2;
  const fits = (center: number, half: number, sign: number, length: number) => {
    const edge = center + sign * (half + offset + r);
    return edge >= 0 && edge <= length;
  };
  const sx = fits(point.x, halfW, towards.x, sheet.width) ? towards.x : -towards.x;
  const sy = fits(point.y, halfH, towards.y, sheet.height) ? towards.y : -towards.y;
  const anchor = { x: point.x + sx * halfW, y: point.y + sy * halfH };
  const clamp = (v: number, max: number) => Math.min(Math.max(v, r), Math.max(r, max - r));
  return {
    anchor,
    position: {
      x: clamp(anchor.x + sx * offset, sheet.width),
      y: clamp(anchor.y + sy * offset, sheet.height),
    },
  };
}
