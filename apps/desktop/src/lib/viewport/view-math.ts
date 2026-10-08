/**
 * Pure view math of the drawing viewport (T0.8, FR-DOC-04). View state only: no document data.
 *
 * Sheet space is PDF user units (1/72 inch), origin top left (AGENTS.md rule 4). Screen space is
 * CSS pixels relative to the top left corner of the viewport element. One transform maps sheet
 * to screen for the tile layer and the SVG overlay alike:
 *
 *   screen = sheet * scale + (tx, ty)
 */

import { MAX_TILE_ZOOM, MIN_TILE_ZOOM } from "$lib/ipc/bindings";

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Uniform scale (CSS px per sheet unit) and translation (CSS px). */
export interface ViewTransform {
  scale: number;
  tx: number;
  ty: number;
}

/** CSS pixels per sheet unit at 100 %: a CSS pixel is 1/96 inch, a sheet unit 1/72 inch. */
export const ACTUAL_SIZE_SCALE = 96 / 72;

/** Smallest view scale: a quarter of the coarsest tile level. */
export const MIN_SCALE = 2 ** MIN_TILE_ZOOM / 4;

/**
 * Largest view scale. At 16 CSS px per unit a Retina display (2 device px per CSS px) shows the
 * finest tile level 1:1, so the drawing stays crisp up to the zoom limit.
 */
export const MAX_SCALE = 2 ** MAX_TILE_ZOOM / 2;

/** Zoom factor of one keyboard or button step. */
export const ZOOM_STEP = Math.SQRT2;

/** Free space around the sheet when fitting, in CSS px. */
export const FIT_PADDING = 16;

/** Part of the sheet that panning keeps inside the viewport, in CSS px. */
export const PAN_KEEP_VISIBLE = 48;

export const IDENTITY: ViewTransform = { scale: 1, tx: 0, ty: 0 };

export function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

export function sheetToScreen(view: ViewTransform, p: Point): Point {
  return { x: p.x * view.scale + view.tx, y: p.y * view.scale + view.ty };
}

export function screenToSheet(view: ViewTransform, p: Point): Point {
  return { x: (p.x - view.tx) / view.scale, y: (p.y - view.ty) / view.scale };
}

/**
 * Zooms by `factor` around the screen point `at`: the sheet point under `at` stays under `at`.
 * The scale is clamped to `MIN_SCALE..MAX_SCALE`; at a limit the point still stays fixed.
 */
export function zoomAt(view: ViewTransform, factor: number, at: Point): ViewTransform {
  const scale = clampScale(view.scale * factor);
  const anchor = screenToSheet(view, at);
  return { scale, tx: at.x - anchor.x * scale, ty: at.y - anchor.y * scale };
}

export function panBy(view: ViewTransform, dx: number, dy: number): ViewTransform {
  return { scale: view.scale, tx: view.tx + dx, ty: view.ty + dy };
}

/** Largest view that shows the whole sheet with `padding` CSS px around it, centered. */
export function fitSheet(sheet: Size, viewport: Size, padding = FIT_PADDING): ViewTransform {
  if (sheet.width <= 0 || sheet.height <= 0 || viewport.width <= 0 || viewport.height <= 0) {
    return IDENTITY;
  }
  const available = {
    width: Math.max(1, viewport.width - 2 * padding),
    height: Math.max(1, viewport.height - 2 * padding),
  };
  const scale = clampScale(
    Math.min(available.width / sheet.width, available.height / sheet.height),
  );
  return {
    scale,
    tx: (viewport.width - sheet.width * scale) / 2,
    ty: (viewport.height - sheet.height * scale) / 2,
  };
}

/** View at `scale` (clamped) with the sheet point `center` in the middle of the viewport. */
export function centerOn(center: Point, scale: number, viewport: Size): ViewTransform {
  const s = clampScale(scale);
  return {
    scale: s,
    tx: viewport.width / 2 - center.x * s,
    ty: viewport.height / 2 - center.y * s,
  };
}

/**
 * Keeps at least `keep` CSS px of the sheet inside the viewport on both axes, so the drawing
 * cannot be panned out of sight.
 */
export function constrainPan(
  view: ViewTransform,
  sheet: Size,
  viewport: Size,
  keep = PAN_KEEP_VISIBLE,
): ViewTransform {
  const clampAxis = (t: number, sheetLength: number, viewLength: number): number => {
    const length = sheetLength * view.scale;
    const margin = Math.min(keep, length, viewLength);
    const min = margin - length;
    const max = viewLength - margin;
    return Math.min(max, Math.max(min, t));
  };
  return {
    scale: view.scale,
    tx: clampAxis(view.tx, sheet.width, viewport.width),
    ty: clampAxis(view.ty, sheet.height, viewport.height),
  };
}

/** The sheet area visible in a viewport of `viewport` CSS px. */
export function visibleSheetRect(view: ViewTransform, viewport: Size): Rect {
  const topLeft = screenToSheet(view, { x: 0, y: 0 });
  return {
    x: topLeft.x,
    y: topLeft.y,
    width: viewport.width / view.scale,
    height: viewport.height / view.scale,
  };
}

/**
 * Rounds the translation to whole device pixels. Tiles then start on device pixel boundaries,
 * which keeps one pixel lines sharp. Tile layer and overlay use the same snapped transform.
 */
export function snapToDevicePixels(view: ViewTransform, devicePixelRatio: number): ViewTransform {
  const dpr = devicePixelRatio > 0 ? devicePixelRatio : 1;
  return {
    scale: view.scale,
    tx: Math.round(view.tx * dpr) / dpr,
    ty: Math.round(view.ty * dpr) / dpr,
  };
}

/** Line height assumed for wheel events in line mode, in CSS px. */
const WHEEL_LINE_PX = 16;

/** Zoom per CSS pixel of wheel movement: 100 px (one mouse wheel notch in most webviews) is 1.22x. */
const WHEEL_ZOOM_PER_PX = 0.002;

/** Largest wheel movement of a single event, so a fast flick never jumps too far. */
const WHEEL_MAX_PX = 150;

/**
 * Zoom factor for a wheel event. Wheel up (negative `deltaY`) zooms in. `deltaMode` 1 is lines,
 * 2 is pages (one page is the viewport height).
 */
export function wheelZoomFactor(deltaY: number, deltaMode: number, pageHeight: number): number {
  const px =
    deltaMode === 1 ? deltaY * WHEEL_LINE_PX : deltaMode === 2 ? deltaY * pageHeight : deltaY;
  const clamped = Math.max(-WHEEL_MAX_PX, Math.min(WHEEL_MAX_PX, px));
  return Math.exp(-clamped * WHEEL_ZOOM_PER_PX);
}

/** Zoom in percent of the printed size, as shown in the toolbar. */
export function zoomPercent(scale: number): number {
  return Math.round((scale / ACTUAL_SIZE_SCALE) * 100);
}

/** CSS or SVG `matrix()` of a view transform. */
export function matrix(view: ViewTransform): string {
  return `matrix(${String(view.scale)},0,0,${String(view.scale)},${String(view.tx)},${String(view.ty)})`;
}
