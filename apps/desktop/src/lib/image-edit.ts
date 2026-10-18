/**
 * View helpers of the image editor (R.20). The editor keeps the settings being edited as view
 * state; everything that depends on the pixels (sheet size, previews, corner proposals, balloon
 * positions) comes from Rust. These helpers only turn numbers typed or dragged on screen into
 * the decimal strings of `ImageEdit`, and draw handles with the maps Rust sends.
 */

import type {
  Homography,
  ImageEdit,
  ImagePoint,
  Orientation,
  Paper,
  PaperSize,
  Rotation,
  SheetCorners,
  Trim,
} from "$lib/ipc/bindings";
import type { Point, Size } from "$lib/viewport/view-math";

/** Settings that leave the image as it is. */
export const DEFAULT_EDIT: ImageEdit = {
  rotation: "deg0",
  fine_angle: "0",
  corners: null,
  trim: null,
  paper: null,
  clean_up: false,
};

/** Range of the straighten slider in degrees; typed angles may go to {@link MAX_ANGLE}. */
export const SLIDER_ANGLE = 10;

/** Largest fine angle Rust accepts, either way. */
export const MAX_ANGLE = 45;

/** Standard paper sizes in the order of the list (R01 decision 9). */
export const PAPER_SIZES: readonly PaperSize[] = [
  "a4",
  "a3",
  "a2",
  "a1",
  "a0",
  "letter",
  "tabloid",
];

/** Smallest trim, as a fraction of the page, so a rectangle never collapses. */
const MIN_TRIM = 0.02;

const ROTATIONS: readonly Rotation[] = ["deg0", "deg90", "deg180", "deg270"];

/** A number as a plain decimal string with at most `places` decimals, no trailing zeros. */
export function decimalText(value: number, places: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }
  const fixed = value.toFixed(places);
  const text = fixed.includes(".") ? fixed.replace(/\.?0+$/, "") : fixed;
  return text === "-0" || text === "" ? "0" : text;
}

/** The number of a decimal string of Rust, 0 if it is not one. */
export function decimalValue(text: string): number {
  const value = Number(text);
  return Number.isFinite(value) ? value : 0;
}

/** Clockwise quarter turns of a rotation, 0 to 3. */
export function turnsOf(rotation: Rotation): number {
  return ROTATIONS.indexOf(rotation);
}

/** `rotation` turned by `delta` clockwise quarter turns. */
export function turnedBy(rotation: Rotation, delta: number): Rotation {
  const turns = (((turnsOf(rotation) + delta) % 4) + 4) % 4;
  return ROTATIONS[turns] ?? "deg0";
}

/**
 * The same four points named for other quarter turns. Corners are named as the sheet reads
 * after the turns: one more clockwise turn makes the old bottom left the new top left.
 */
export function renameCorners(corners: SheetCorners, from: Rotation, to: Rotation): SheetCorners {
  const list = [corners.top_left, corners.top_right, corners.bottom_right, corners.bottom_left];
  const delta = (((turnsOf(to) - turnsOf(from)) % 4) + 4) % 4;
  const at = (i: number): ImagePoint => list[(i - delta + 4) % 4] ?? corners.top_left;
  return { top_left: at(0), top_right: at(1), bottom_right: at(2), bottom_left: at(3) };
}

/** The point a projective map of Rust sends `p` to, `null` if it goes to infinity. */
export function mapPoint(map: Homography, p: Point): Point | null {
  const m = map.map((v) => v ?? 0);
  const [a = 0, b = 0, c = 0, d = 0, e = 0, f = 0, g = 0, h = 0, i = 1] = m;
  const w = g * p.x + h * p.y + i;
  if (Math.abs(w) < 1e-12) {
    return null;
  }
  return { x: (a * p.x + b * p.y + c) / w, y: (d * p.x + e * p.y + f) / w };
}

/** A pixel position of the stored page as numbers. */
export function pixelPoint(p: ImagePoint): Point {
  return { x: decimalValue(p.x), y: decimalValue(p.y) };
}

/** A pixel position as decimals with two places, kept inside the page. */
export function imagePoint(p: Point, page: Size): ImagePoint {
  const clamp = (v: number, max: number) => Math.min(max, Math.max(0, v));
  return {
    x: decimalText(clamp(p.x, page.width), 2),
    y: decimalText(clamp(p.y, page.height), 2),
  };
}

/** Width and height of the stored page in pixels, from the corners of the whole page. */
export function pageSize(page: SheetCorners): Size {
  const points = [page.top_left, page.top_right, page.bottom_right, page.bottom_left].map(
    pixelPoint,
  );
  return {
    width: Math.max(...points.map((p) => p.x)),
    height: Math.max(...points.map((p) => p.y)),
  };
}

/** A rectangle on the trim view. */
export interface TrimRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** The trim rectangle in units of the trim view of `size`; the whole view without a trim. */
export function trimRect(trim: Trim | null, size: Size): TrimRect {
  const f = (v: string | undefined, whole: number) => (v === undefined ? whole : decimalValue(v));
  return {
    left: f(trim?.left, 0) * size.width,
    top: f(trim?.top, 0) * size.height,
    right: f(trim?.right, 1) * size.width,
    bottom: f(trim?.bottom, 1) * size.height,
  };
}

/**
 * The trim of a rectangle drawn on the trim view of `size`: fractions with four places, inside
 * 0 to 1, at least {@link MIN_TRIM} wide and high. `null` when it keeps the whole page.
 */
export function trimOf(rect: TrimRect, size: Size): Trim | null {
  if (size.width <= 0 || size.height <= 0) {
    return null;
  }
  const unit = (v: number) => Math.min(1, Math.max(0, v));
  let left = unit(Math.min(rect.left, rect.right) / size.width);
  let right = unit(Math.max(rect.left, rect.right) / size.width);
  let top = unit(Math.min(rect.top, rect.bottom) / size.height);
  let bottom = unit(Math.max(rect.top, rect.bottom) / size.height);
  if (right - left < MIN_TRIM) {
    right = Math.min(1, left + MIN_TRIM);
    left = right - MIN_TRIM;
  }
  if (bottom - top < MIN_TRIM) {
    bottom = Math.min(1, top + MIN_TRIM);
    top = bottom - MIN_TRIM;
  }
  const trim: Trim = {
    left: decimalText(left, 4),
    top: decimalText(top, 4),
    right: decimalText(right, 4),
    bottom: decimalText(bottom, 4),
  };
  const whole = trim.left === "0" && trim.top === "0" && trim.right === "1" && trim.bottom === "1";
  return whole ? null : trim;
}

/** A standard paper in the orientation that fits a page of `size` best. */
export function standardPaper(size: PaperSize, page: Size | null): Paper {
  const orientation: Orientation =
    page !== null && page.width > page.height ? "landscape" : "portrait";
  return { format: "standard", size, orientation };
}

/** True if the settings leave the image as it is. */
export function isDefaultEdit(edit: ImageEdit): boolean {
  return sameEdit(edit, DEFAULT_EDIT);
}

/** True if both settings are the same. */
export function sameEdit(a: ImageEdit, b: ImageEdit): boolean {
  return JSON.stringify(normalized(a)) === JSON.stringify(normalized(b));
}

function normalized(edit: ImageEdit): ImageEdit {
  return { ...edit, fine_angle: decimalText(decimalValue(edit.fine_angle), 2) };
}
