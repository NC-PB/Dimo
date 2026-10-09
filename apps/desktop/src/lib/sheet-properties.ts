/**
 * Sheet rotation, unit and scale (T1.8, FR-DOC-05, D-20). The values live in the project and
 * change only through the undoable `update_sheet` command; this module builds that command and
 * holds the choices of the properties panel. Rust validates every value.
 */

import type { Command, Patch, Rotation, Scale, Sheet, Unit } from "$lib/ipc/bindings";
import { documentStore } from "$lib/stores/document.svelte";
import { projectStore } from "$lib/stores/project.svelte";
import { rotatedBy, storedRotation, viewRotation } from "$lib/viewport/view-math";

/** Length units a sheet can use. Never inferred from the sheet size (D-20). */
export const SHEET_UNITS: readonly Unit[] = ["mm", "in"];

/** Common scales of the select, as `drawing:actual`: 2:1 enlarges, 1:2 reduces. */
export const SCALE_PRESETS: readonly Scale[] = [
  { drawing: 50, actual: 1 },
  { drawing: 20, actual: 1 },
  { drawing: 10, actual: 1 },
  { drawing: 5, actual: 1 },
  { drawing: 2, actual: 1 },
  { drawing: 1, actual: 1 },
  { drawing: 1, actual: 2 },
  { drawing: 1, actual: 5 },
  { drawing: 1, actual: 10 },
  { drawing: 1, actual: 20 },
  { drawing: 1, actual: 50 },
  { drawing: 1, actual: 100 },
];

export function sameScale(a: Scale, b: Scale): boolean {
  return a.drawing === b.drawing && a.actual === b.actual;
}

/** Text of a scale such as `1:2`. */
export function scaleText(scale: Scale): string {
  return `${String(scale.drawing)}:${String(scale.actual)}`;
}

/** The preset equal to `scale`, or `undefined` for a custom scale. */
export function presetFor(scale: Scale): Scale | undefined {
  return SCALE_PRESETS.find((p) => sameScale(p, scale));
}

/** Largest scale part, far beyond any drawing and well inside `u32`. */
export const MAX_SCALE_PART = 1_000_000;

/** A scale part typed by the user: whole number from 1 to {@link MAX_SCALE_PART}, else `null`. */
export function parseScalePart(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d{1,7}$/.test(trimmed)) {
    return null;
  }
  const value = Number(trimmed);
  return value >= 1 && value <= MAX_SCALE_PART ? value : null;
}

/** What to change on a sheet; omitted values stay as they are. */
export interface SheetChange {
  rotation?: Rotation;
  unit?: Unit;
  scale?: Scale;
}

/** The `update_sheet` command for `change`. */
export function updateSheetCommand(sheet: Sheet, change: SheetChange): Command {
  return {
    type: "update_sheet",
    sheet: sheet.id,
    rotation: change.rotation ?? null,
    unit: change.unit ?? null,
    scale: change.scale ?? null,
  };
}

/** The sheet of the project that is on screen, or `undefined`. */
export function shownSheet(): Sheet | undefined {
  return projectStore.sheets[documentStore.sheet];
}

/** Changes the shown sheet. Resolves when the project shows the change. */
export function updateShownSheet(change: SheetChange): Promise<Patch | undefined> {
  const sheet = shownSheet();
  return sheet
    ? projectStore.execute(updateSheetCommand(sheet, change))
    : Promise.resolve(undefined);
}

let rotating: Promise<unknown> = Promise.resolve();

/**
 * Turns the shown sheet by `steps` quarter turns, clockwise when positive. Calls queue up, and
 * each one reads the rotation when its turn comes, so quick key presses add up.
 */
export function rotateShownSheet(steps: number): Promise<void> {
  const turn = async (): Promise<void> => {
    const sheet = shownSheet();
    if (sheet) {
      const rotation = storedRotation(rotatedBy(viewRotation(sheet.rotation), steps));
      await projectStore.execute(updateSheetCommand(sheet, { rotation }));
    }
  };
  const next = rotating.then(turn);
  rotating = next.catch(() => undefined);
  return next;
}
