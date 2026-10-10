/**
 * Zone grids and view rectangles of a sheet, drawn by hand (T2.7, M2 decision 1, D-21). The
 * values live in the project and change only through the undoable `set_zone_grid` and
 * `set_views` commands; this module builds those commands. Default grids and axis labels come
 * from Rust (`zone_grid_form`, T2.7a), which also validates every grid and view.
 */

import type {
  Command,
  Rect,
  Sheet,
  SheetView,
  ZoneGrid,
  ZoneGridEdit,
  ZoneGridForm,
} from "$lib/ipc/bindings";
import type { Point } from "$lib/viewport/view-math";

/** Most columns or rows of a grid, as in Rust (`ZoneGrid::MAX_DIVISIONS`). */
export const MAX_DIVISIONS = 100;

/**
 * The zone grid query of Rust (T2.7a): default grids and axis labels come from `dimo-core`
 * (rule 2). The generated `commands` in the app.
 */
export interface ZoneGridApi {
  zoneGridForm(edit: ZoneGridEdit): Promise<ZoneGridForm>;
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
