/**
 * Bringing a selected balloon into view (T1.7, FR-BAL-12): a characteristic selected in the
 * table shows its sheet and its balloon. Pure functions, so the viewport side of the selection
 * sync is testable without the component.
 */

import type { Balloon, CharId, SheetId } from "$lib/ipc/bindings";
import {
  centerOn,
  fitSheet,
  type Point,
  type Rect,
  type Size,
  type ViewRotation,
  type ViewTransform,
} from "./view-math";

/** What the viewport does for a new selection. */
export type RevealPlan =
  | { kind: "none" }
  /** Show sheet `sheet` and then the balloon at `center` (sheet space). */
  | { kind: "switch"; sheet: number; center: Point }
  /** The balloon at `center` is on the shown sheet but out of view: center it. */
  | { kind: "center"; center: Point };

/** Whether sheet point `p` lies inside `rect`, edges included. */
export function isInside(p: Point, rect: Rect): boolean {
  return (
    p.x >= rect.x && p.y >= rect.y && p.x <= rect.x + rect.width && p.y <= rect.y + rect.height
  );
}

/**
 * The reveal for a selection. Only when the selection has exactly one balloon, so group
 * selections (box select, select all) never move the view.
 *
 * @param selected selected characteristic IDs
 * @param balloons all balloons of the project
 * @param sheets the sheets of the project, in drawing order (index = viewport sheet index)
 * @param shownSheet index of the sheet in the viewport
 * @param visible the visible part of the shown sheet, in sheet space
 */
export function revealPlan(
  selected: ReadonlySet<CharId>,
  balloons: readonly Balloon[],
  sheets: readonly { id: SheetId }[],
  shownSheet: number,
  visible: Rect,
): RevealPlan {
  if (selected.size === 0) {
    return { kind: "none" };
  }
  const chosen = balloons.filter((b) => selected.has(b.characteristic));
  const only = chosen.length === 1 ? chosen[0] : undefined;
  if (only === undefined) {
    return { kind: "none" };
  }
  const center = { x: only.position.x ?? 0, y: only.position.y ?? 0 };
  const index = sheets.findIndex((s) => s.id === only.sheet);
  if (index < 0) {
    return { kind: "none" };
  }
  if (index !== shownSheet) {
    return { kind: "switch", sheet: index, center };
  }
  return isInside(center, visible) ? { kind: "none" } : { kind: "center", center };
}

/**
 * The view of a sheet that was just switched to for a reveal: centered on the balloon at the
 * zoom the user had, or fitted when that zoom is not larger than the fit (the whole sheet,
 * balloon included, is then visible anyway).
 */
export function viewAfterSwitch(
  center: Point,
  previousScale: number,
  sheet: Size,
  viewport: Size,
  rotation: ViewRotation,
): ViewTransform {
  const fitted = fitSheet(sheet, viewport, rotation);
  if (previousScale <= fitted.scale) {
    return fitted;
  }
  return centerOn(center, previousScale, viewport, rotation);
}
