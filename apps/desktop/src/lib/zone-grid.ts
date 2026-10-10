/**
 * Zone grids and view rectangles of a sheet, drawn by hand (T2.7, M2 decision 1, D-21). The
 * values live in the project and change only through the undoable `set_zone_grid` and
 * `set_views` commands; this module builds those commands and the labels the editor offers.
 * Rust validates every grid and view.
 */

import type { Command, Rect, Sheet, SheetView, ZoneGrid } from "$lib/ipc/bindings";
import type { Point, Size } from "$lib/viewport/view-math";

/** How the labels of one axis are written: letters `A, B, ...` or numbers `1, 2, ...`. */
export type LabelKind = "letters" | "numbers";

/**
 * Label scheme of one axis: the kind and whether it counts from the far end (numbers right to
 * left, letters bottom to top). Labels are stored as printed, left to right and top to bottom.
 */
export interface AxisScheme {
  kind: LabelKind;
  reversed: boolean;
}

/** Most columns or rows of a grid, as in Rust (`ZoneGrid::MAX_DIVISIONS`). */
export const MAX_DIVISIONS = 100;

/** Default: rows lettered top to bottom, columns numbered left to right. */
export const DEFAULT_ROWS: AxisScheme = { kind: "letters", reversed: false };
export const DEFAULT_COLUMNS: AxisScheme = { kind: "numbers", reversed: false };

/** Letters of a count: 1 is `A`, 26 is `Z`, 27 is `AA`. */
export function letters(n: number): string {
  let out = "";
  let value = n;
  while (value > 0) {
    const rest = (value - 1) % 26;
    out = String.fromCharCode(65 + rest) + out;
    value = Math.floor((value - 1) / 26);
  }
  return out;
}

/** The labels of an axis with `count` divisions, as printed (left to right, top to bottom). */
export function axisLabels(count: number, scheme: AxisScheme): string[] {
  const n = Math.max(1, Math.min(MAX_DIVISIONS, Math.floor(count)));
  const labels = Array.from({ length: n }, (_, i) =>
    scheme.kind === "letters" ? letters(i + 1) : String(i + 1),
  );
  return scheme.reversed ? labels.reverse() : labels;
}

/** The scheme that produces `labels`, or `null` for labels typed by hand. */
export function schemeOf(labels: readonly string[]): AxisScheme | null {
  for (const kind of ["letters", "numbers"] as const) {
    for (const reversed of [false, true]) {
      const scheme = { kind, reversed };
      const expected = axisLabels(labels.length, scheme);
      if (expected.length === labels.length && expected.every((l, i) => l === labels[i])) {
        return scheme;
      }
    }
  }
  return null;
}

/** Labels typed as a list separated by commas or spaces. Empty parts are dropped. */
export function parseLabels(text: string): string[] {
  return text
    .split(/[\s,;]+/)
    .map((l) => l.trim())
    .filter((l) => l !== "");
}

/** Labels as the editor shows them. */
export function labelsText(labels: readonly string[]): string {
  return labels.join(", ");
}

/** The rectangle spanned by two points, in sheet space. */
export function rectBetween(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return {
    origin: { x, y },
    size: { width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) },
  };
}

/** Smallest rectangle side that counts as drawn, in sheet units (about 3.5 mm). */
export const MIN_RECT_SIDE = 10;

export function isDrawn(rect: Rect): boolean {
  return (rect.size.width ?? 0) >= MIN_RECT_SIDE && (rect.size.height ?? 0) >= MIN_RECT_SIDE;
}

/** Margin of the default frame inside the sheet edge: 10 mm in sheet units. */
const FRAME_MARGIN = (10 * 72) / 25.4;

/**
 * A first grid for a sheet: a frame 10 mm inside the sheet edge, 8 by 6 zones on landscape
 * sheets and 6 by 8 on portrait ones, rows lettered and columns numbered.
 */
export function defaultGrid(size: Size): ZoneGrid {
  const landscape = size.width >= size.height;
  const margin = Math.min(FRAME_MARGIN, size.width / 4, size.height / 4);
  return {
    frame: {
      origin: { x: margin, y: margin },
      size: { width: size.width - 2 * margin, height: size.height - 2 * margin },
    },
    column_labels: axisLabels(landscape ? 8 : 6, DEFAULT_COLUMNS),
    row_labels: axisLabels(landscape ? 6 : 8, DEFAULT_ROWS),
  };
}

/** A rectangle of the project with missing (non finite) values as 0. */
export function frameOf(rect: Rect): { x: number; y: number; width: number; height: number } {
  return {
    x: rect.origin.x ?? 0,
    y: rect.origin.y ?? 0,
    width: rect.size.width ?? 0,
    height: rect.size.height ?? 0,
  };
}

/** Positions of the inner grid lines: x of each column border, y of each row border. */
export function gridLines(grid: ZoneGrid): { xs: number[]; ys: number[] } {
  const f = frameOf(grid.frame);
  const columns = grid.column_labels.length;
  const rows = grid.row_labels.length;
  return {
    xs: Array.from({ length: columns - 1 }, (_, i) => f.x + ((i + 1) * f.width) / columns),
    ys: Array.from({ length: rows - 1 }, (_, i) => f.y + ((i + 1) * f.height) / rows),
  };
}

/** Where each label is drawn: column labels above the frame, row labels left of it. */
export function labelPositions(
  grid: ZoneGrid,
): { text: string; at: Point; axis: "columns" | "rows" }[] {
  const f = frameOf(grid.frame);
  const columns = grid.column_labels.length;
  const rows = grid.row_labels.length;
  return [
    ...grid.column_labels.map((text, i) => ({
      text,
      at: { x: f.x + ((i + 0.5) * f.width) / columns, y: f.y },
      axis: "columns" as const,
    })),
    ...grid.row_labels.map((text, i) => ({
      text,
      at: { x: f.x, y: f.y + ((i + 0.5) * f.height) / rows },
      axis: "rows" as const,
    })),
  ];
}

/** The `set_zone_grid` command; `null` removes the grid. */
export function zoneGridCommand(sheet: Sheet, grid: ZoneGrid | null): Command {
  return { type: "set_zone_grid", sheet: sheet.id, grid };
}

/** The `set_views` command with all views of the sheet. */
export function viewsCommand(sheet: Sheet, views: SheetView[]): Command {
  return { type: "set_views", sheet: sheet.id, views };
}

/** The grid with `count` columns (or rows) labelled by `scheme`. */
export function withAxis(
  grid: ZoneGrid,
  axis: "columns" | "rows",
  count: number,
  scheme: AxisScheme,
): ZoneGrid {
  const labels = axisLabels(count, scheme);
  return axis === "columns" ? { ...grid, column_labels: labels } : { ...grid, row_labels: labels };
}
