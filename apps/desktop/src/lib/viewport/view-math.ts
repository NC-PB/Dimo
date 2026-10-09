/**
 * Pure view math of the drawing viewport (T0.8, FR-DOC-04). View state only: no document data.
 *
 * Sheet space is PDF user units (1/72 inch), origin top left (AGENTS.md rule 4). Screen space is
 * CSS pixels relative to the top left corner of the viewport element. One transform maps sheet
 * to screen for the tile layer and the SVG overlay alike:
 *
 *   screen = R(rotation) * sheet * scale + (tx, ty)
 *
 * `R` turns the sheet clockwise by 0, 90, 180 or 270 degrees around the sheet origin (T1.8,
 * FR-DOC-05). Only the view rotates: tiles and stored geometry stay in unrotated sheet space,
 * so tile cache entries stay valid and a rotation never rewrites a balloon.
 */

import { MAX_TILE_ZOOM, MIN_TILE_ZOOM, type Rotation } from "$lib/ipc/bindings";

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

/** View rotation in degrees, clockwise, in 90 degree steps. */
export type ViewRotation = 0 | 90 | 180 | 270;

/** Rotation of the view (degrees), uniform scale (CSS px per sheet unit), translation (CSS px). */
export interface ViewTransform {
  scale: number;
  tx: number;
  ty: number;
  rotation: ViewRotation;
}

const ROTATION_DEGREES: Record<Rotation, ViewRotation> = {
  deg0: 0,
  deg90: 90,
  deg180: 180,
  deg270: 270,
};

/** The view rotation of a sheet's stored `Rotation`. */
export function viewRotation(rotation: Rotation): ViewRotation {
  return ROTATION_DEGREES[rotation];
}

/** The stored `Rotation` of a view rotation. */
export function storedRotation(rotation: ViewRotation): Rotation {
  switch (rotation) {
    case 0:
      return "deg0";
    case 90:
      return "deg90";
    case 180:
      return "deg180";
    case 270:
      return "deg270";
  }
}

/** The rotation after `steps` clockwise quarter turns; negative steps turn counterclockwise. */
export function rotatedBy(rotation: ViewRotation, steps: number): ViewRotation {
  const quarter = (((rotation / 90 + steps) % 4) + 4) % 4;
  return (quarter * 90) as ViewRotation;
}

/** The size of a sheet as it appears on screen: width and height swap at 90 and 270 degrees. */
export function rotatedSize(size: Size, rotation: ViewRotation): Size {
  return rotation === 90 || rotation === 270 ? { width: size.height, height: size.width } : size;
}

/** Rows of the rotation matrix `R`: `x' = r00 x + r01 y`, `y' = r10 x + r11 y`. */
function rotationMatrix(rotation: ViewRotation): readonly [number, number, number, number] {
  switch (rotation) {
    case 0:
      return [1, 0, 0, 1];
    case 90:
      return [0, -1, 1, 0];
    case 180:
      return [-1, 0, 0, -1];
    case 270:
      return [0, 1, -1, 0];
  }
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

export const IDENTITY: ViewTransform = { scale: 1, tx: 0, ty: 0, rotation: 0 };

export function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

export function sheetToScreen(view: ViewTransform, p: Point): Point {
  const [r00, r01, r10, r11] = rotationMatrix(view.rotation);
  return {
    x: (r00 * p.x + r01 * p.y) * view.scale + view.tx,
    y: (r10 * p.x + r11 * p.y) * view.scale + view.ty,
  };
}

export function screenToSheet(view: ViewTransform, p: Point): Point {
  // The inverse of a rotation is its transpose.
  const [r00, r01, r10, r11] = rotationMatrix(view.rotation);
  const x = (p.x - view.tx) / view.scale;
  const y = (p.y - view.ty) / view.scale;
  return { x: r00 * x + r10 * y, y: r01 * x + r11 * y };
}

/** The view with the sheet point `anchor` at the screen point `at`. */
function anchored(view: ViewTransform, anchor: Point, at: Point): ViewTransform {
  const origin = sheetToScreen({ ...view, tx: 0, ty: 0 }, anchor);
  return { ...view, tx: at.x - origin.x, ty: at.y - origin.y };
}

/** Smallest axis aligned rectangle that contains `points`. */
function boundingRect(points: readonly Point[]): Rect {
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

/** The corners of a rectangle of `size` at the origin. */
function corners(size: Size): Point[] {
  return [
    { x: 0, y: 0 },
    { x: size.width, y: 0 },
    { x: 0, y: size.height },
    { x: size.width, y: size.height },
  ];
}

/** The sheet on screen: its bounding rectangle in CSS px (exact, as turns are quarters). */
export function sheetScreenRect(view: ViewTransform, sheet: Size): Rect {
  return boundingRect(corners(sheet).map((p) => sheetToScreen(view, p)));
}

/**
 * Turns the view to `rotation` and keeps the sheet point at the center of the viewport where it
 * is, so rotating does not move what the user looks at.
 */
export function rotateView(
  view: ViewTransform,
  rotation: ViewRotation,
  viewport: Size,
): ViewTransform {
  const center = { x: viewport.width / 2, y: viewport.height / 2 };
  return anchored({ ...view, rotation }, screenToSheet(view, center), center);
}

/**
 * Zooms by `factor` around the screen point `at`: the sheet point under `at` stays under `at`.
 * The scale is clamped to `MIN_SCALE..MAX_SCALE`; at a limit the point still stays fixed.
 */
export function zoomAt(view: ViewTransform, factor: number, at: Point): ViewTransform {
  const scaled = { ...view, scale: clampScale(view.scale * factor) };
  return anchored(scaled, screenToSheet(view, at), at);
}

export function panBy(view: ViewTransform, dx: number, dy: number): ViewTransform {
  return { ...view, tx: view.tx + dx, ty: view.ty + dy };
}

/**
 * Largest view that shows the whole sheet, as turned by `rotation`, with `padding` CSS px
 * around it, centered.
 */
export function fitSheet(
  sheet: Size,
  viewport: Size,
  rotation: ViewRotation = 0,
  padding = FIT_PADDING,
): ViewTransform {
  const turned = rotatedSize(sheet, rotation);
  if (turned.width <= 0 || turned.height <= 0 || viewport.width <= 0 || viewport.height <= 0) {
    return { ...IDENTITY, rotation };
  }
  const available = {
    width: Math.max(1, viewport.width - 2 * padding),
    height: Math.max(1, viewport.height - 2 * padding),
  };
  const scale = clampScale(
    Math.min(available.width / turned.width, available.height / turned.height),
  );
  // Put the bounding rectangle of the turned sheet in the middle of the viewport.
  const rect = sheetScreenRect({ scale, tx: 0, ty: 0, rotation }, sheet);
  return {
    scale,
    rotation,
    tx: (viewport.width - rect.width) / 2 - rect.x,
    ty: (viewport.height - rect.height) / 2 - rect.y,
  };
}

/** View at `scale` (clamped) with the sheet point `center` in the middle of the viewport. */
export function centerOn(
  center: Point,
  scale: number,
  viewport: Size,
  rotation: ViewRotation = 0,
): ViewTransform {
  const view = { scale: clampScale(scale), tx: 0, ty: 0, rotation };
  return anchored(view, center, {
    x: viewport.width / 2,
    y: viewport.height / 2,
  });
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
  const rect = sheetScreenRect(view, sheet);
  // Shift that moves the rectangle edge `start` (length `length`) into its allowed range.
  const shiftAxis = (start: number, length: number, viewLength: number): number => {
    const margin = Math.min(keep, length, viewLength);
    const min = margin - length;
    const max = viewLength - margin;
    return Math.min(max, Math.max(min, start)) - start;
  };
  return {
    ...view,
    tx: view.tx + shiftAxis(rect.x, rect.width, viewport.width),
    ty: view.ty + shiftAxis(rect.y, rect.height, viewport.height),
  };
}

/**
 * The sheet area visible in a viewport of `viewport` CSS px: the bounding rectangle, in sheet
 * units, of the viewport (exact, as turns are quarters).
 */
export function visibleSheetRect(view: ViewTransform, viewport: Size): Rect {
  return boundingRect(corners(viewport).map((p) => screenToSheet(view, p)));
}

/**
 * Rounds the translation to whole device pixels. Tiles then start on device pixel boundaries,
 * which keeps one pixel lines sharp. Tile layer and overlay use the same snapped transform.
 */
export function snapToDevicePixels(view: ViewTransform, devicePixelRatio: number): ViewTransform {
  const dpr = devicePixelRatio > 0 ? devicePixelRatio : 1;
  return {
    ...view,
    tx: Math.round(view.tx * dpr) / dpr,
    ty: Math.round(view.ty * dpr) / dpr,
  };
}
/**
 * Wheel and pinch zoom tuning. All constants live here (T1.0).
 *
 * Mouse wheel: discrete events, 100 px per notch in most webviews. Pinch on a trackpad arrives
 * as wheel events with `ctrlKey` set (Chromium, WebKit in Tauri) and small continuous deltas,
 * 1 to 10 px per event. Safari style gesture events carry an absolute `scale` instead.
 */
export const ZOOM_TUNING = {
  /** Line height assumed for wheel events in line mode, in CSS px. */
  wheelLinePx: 16,
  /** Zoom per CSS pixel of wheel movement: 100 px (one notch) is 1.22x. */
  wheelZoomPerPx: 0.002,
  /** Largest wheel movement of a single event, so a fast flick never jumps too far. */
  wheelMaxPx: 150,
  /** Zoom per CSS pixel of a pinch wheel event: 10 px is 1.105x, a slow pinch stays smooth. */
  pinchZoomPerPx: 0.01,
  /** Largest pinch movement of a single event. */
  pinchMaxPx: 40,
} as const;

/** The parts of a `WheelEvent` the zoom needs. */
export interface WheelInput {
  deltaY: number;
  /** 0 pixels, 1 lines, 2 pages. */
  deltaMode: number;
  /** Set by the webview for a trackpad pinch. */
  ctrlKey: boolean;
}

/**
 * Zoom factor for a wheel event. Wheel up (negative `deltaY`) zooms in; a pinch out (also
 * negative) zooms in. `deltaMode` 1 is lines, 2 is pages (one page is `pageHeight`).
 */
export function wheelZoomFactor(event: WheelInput, pageHeight: number): number {
  const { deltaY, deltaMode, ctrlKey } = event;
  const px =
    deltaMode === 1
      ? deltaY * ZOOM_TUNING.wheelLinePx
      : deltaMode === 2
        ? deltaY * pageHeight
        : deltaY;
  const max = ctrlKey ? ZOOM_TUNING.pinchMaxPx : ZOOM_TUNING.wheelMaxPx;
  const perPx = ctrlKey ? ZOOM_TUNING.pinchZoomPerPx : ZOOM_TUNING.wheelZoomPerPx;
  const clamped = Math.max(-max, Math.min(max, px));
  return Math.exp(-clamped * perPx);
}

/** Zoom factor between two `scale` values of a Safari gesture event (1 at gesture start). */
export function gestureZoomFactor(previousScale: number, scale: number): number {
  return previousScale > 0 && scale > 0 ? scale / previousScale : 1;
}

/** Zoom in percent of the printed size, as shown in the toolbar. */
export function zoomPercent(scale: number): number {
  return Math.round((scale / ACTUAL_SIZE_SCALE) * 100);
}

/** CSS or SVG `matrix()` of a view transform, rotation included. */
export function matrix(view: ViewTransform): string {
  const [r00, r01, r10, r11] = rotationMatrix(view.rotation);
  // `matrix(a, b, c, d, e, f)` is x' = a x + c y + e, y' = b x + d y + f. Adding 0 turns -0 into 0.
  const linear = [r00, r10, r01, r11].map((v) => String(v * view.scale + 0));
  return `matrix(${linear.join(",")},${String(view.tx)},${String(view.ty)})`;
}
