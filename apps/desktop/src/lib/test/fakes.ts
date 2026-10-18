/**
 * Fakes of the Rust commands for frontend tests (MF F8): one place for the stand-ins that
 * component and store tests used to copy. Each fake records what it was asked and answers
 * with an empty change unless the test says otherwise; a command a test does not expect
 * rejects with "not used", so a forgotten call shows up. Used by tests only.
 */

import { vi } from "vitest";
import type {
  Command,
  MeasurementState,
  NumberingPreview,
  NumberingStrategy,
  ProjectLoaded,
  ProjectPatched,
} from "$lib/ipc/bindings";
import type { Result } from "$lib/ipc/result";
import { loaded, status } from "$lib/stores/fixtures";
import type { MeasurementCommands } from "$lib/stores/measurement.svelte";
import type { NumberingApi } from "$lib/stores/numbering.svelte";
import type { ProjectCommands } from "$lib/stores/project.svelte";
import type { ToleranceApi } from "$lib/stores/tolerance.svelte";

/** A successful answer of a command. */
export function ok<T>(data: T): Promise<Result<T>> {
  return Promise.resolve({ status: "ok", data });
}

/** The answer of a command the test did not expect. */
export function unused(): Promise<never> {
  return Promise.reject(new Error("not used"));
}

/** A patch event with no changes, as Rust sends it for a command that changed nothing. */
export function emptyPatch(session = 1, revision = 0): ProjectPatched {
  return { session, revision, patch: { changes: [] }, status: status(), drawing: null };
}

/** What Rust answers to one document command. */
export type Reply = Result<ProjectPatched>;

/**
 * The project commands: `commands` records every executed document command, `answer` sets
 * the reply (an empty patch by default). Opening and saving succeed; undo and redo are not used
 * unless `overrides` provides them.
 */
export function fakeProjectApi(
  options: { state?: ProjectLoaded; overrides?: Partial<ProjectCommands> } = {},
) {
  const commands: Command[] = [];
  let reply: (command: Command) => Reply = () => ({ status: "ok", data: emptyPatch() });
  const api: ProjectCommands = {
    projectState: () => ok(options.state ?? loaded(1)),
    newProject: () => ok(true),
    openProject: () => ok(true),
    saveProject: () => ok(true),
    saveProjectAs: () => ok(true),
    confirmClose: () => ok(null),
    closeProject: () => ok(null),
    openRecentProject: () => ok(true),
    execute: (command) => {
      commands.push(command);
      return Promise.resolve(reply(command));
    },
    undo: unused,
    redo: unused,
    ...options.overrides,
  };
  return {
    api,
    commands,
    /** Sets what Rust answers from now on. */
    answer(next: (command: Command) => Reply) {
      reply = next;
    },
  };
}

/** The tolerance commands; each is unused unless `overrides` provides it. */
export function fakeToleranceApi(overrides: Partial<ToleranceApi> = {}): ToleranceApi {
  return {
    toleranceTables: () => ok([]),
    explainCharacteristic: unused,
    characteristicHistory: unused,
    reinterpretCharacteristics: unused,
    importToleranceTable: unused,
    setShorterLeg: unused,
    shorterLegUnit: () => ok("mm"),
    generalToleranceCandidates: () => ok([]),
    applyGeneralTolerance: unused,
    ...overrides,
  };
}

/** The numbering preview: records the asked strategies and answers with `preview`. */
export function fakeNumberingApi(
  preview: (strategy: NumberingStrategy) => NumberingPreview = (strategy) => ({
    strategy,
    entries: [],
    changed: 0,
  }),
) {
  const asked: NumberingStrategy[] = [];
  const api: NumberingApi = {
    previewNumbering: (strategy) => {
      asked.push(strategy);
      return ok(preview(strategy));
    },
  };
  return { api, asked };
}

/** The measurement queries: answers with `state` and the changed characteristics. */
export function fakeMeasurementApi(
  state: MeasurementState | (() => MeasurementState),
  changed: string[] = [],
): MeasurementCommands {
  return {
    measurementState: () => ok(typeof state === "function" ? state() : state),
    changedSinceIssuedReport: () => ok(changed),
  };
}

/**
 * happy-dom does no layout. Gives grids (`role="grid"`) the scroll area of a browser:
 * `height` CSS px high, `width` wide, and a content height of one `rowHeight` per row of
 * `aria-rowcount`, so virtualizers have a window. Undo with `vi.restoreAllMocks()`.
 */
export function stubGridLayout(options: { height: number; width: number; rowHeight: number }) {
  const gridHeight = function (this: HTMLElement) {
    return this.getAttribute("role") === "grid" ? options.height : 0;
  };
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(gridHeight);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(gridHeight);
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(options.width);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(options.width);
  vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return Number(this.getAttribute("aria-rowcount") ?? 0) * options.rowHeight;
  });
}
