import type { Command, Patch, Project, Sheet, SheetView } from "$lib/ipc/bindings";
import { defaultGrid, isDrawn, rectBetween, viewsCommand, zoneGridCommand } from "$lib/zone-grid";
import type { Point } from "$lib/viewport/view-math";
import { documentStore } from "./document.svelte";
import { projectStore } from "./project.svelte";

/** The rectangle tools of the sheet properties: the zone frame and a new view (T2.7). */
export type RegionTool = "zone_frame" | "view";

/** What the store needs of the project and the shown sheet. */
export interface RegionProject {
  readonly project: Project | null;
  readonly sheets: readonly Sheet[];
  execute(command: Command): Promise<Patch | undefined>;
}

/**
 * Drawing a zone frame or a view rectangle on the shown sheet (M2 decision 1). View state only:
 * the finished rectangle becomes one undoable `set_zone_grid` or `set_views` command.
 */
export class SheetRegionsStore {
  /** The active tool, or `null`. */
  tool = $state<RegionTool | null>(null);
  /** Start and current point of the rectangle being drawn, in sheet space. */
  drag = $state.raw<{ from: Point; to: Point } | null>(null);

  readonly #project: RegionProject;
  readonly #sheetIndex: () => number;

  constructor(
    project: RegionProject = projectStore,
    sheetIndex: () => number = () => documentStore.sheet,
  ) {
    this.#project = project;
    this.#sheetIndex = sheetIndex;
  }

  /** The sheet on screen. */
  get sheet(): Sheet | undefined {
    return this.#project.sheets[this.#sheetIndex()];
  }

  /** Starts a tool; the same tool again stops it. */
  toggle(tool: RegionTool): void {
    this.tool = this.tool === tool ? null : tool;
    this.drag = null;
  }

  cancel(): void {
    this.tool = null;
    this.drag = null;
  }

  begin(at: Point): void {
    if (this.tool !== null) {
      this.drag = { from: at, to: at };
    }
  }

  moveTo(at: Point): void {
    if (this.drag !== null) {
      this.drag = { ...this.drag, to: at };
    }
  }

  /**
   * Ends the drag: a zone frame keeps the labels of the current grid (or gets the default ones),
   * a view is added after the others. Too small rectangles are ignored. The tool stops.
   */
  async finish(at: Point): Promise<Patch | undefined> {
    const drag = this.drag;
    const tool = this.tool;
    const sheet = this.sheet;
    this.drag = null;
    this.tool = null;
    if (drag === null || tool === null || sheet === undefined) {
      return undefined;
    }
    const rect = rectBetween(drag.from, at);
    if (!isDrawn(rect)) {
      return undefined;
    }
    if (tool === "zone_frame") {
      const grid =
        sheet.zone_grid ??
        defaultGrid({
          width: sheet.size.width ?? 0,
          height: sheet.size.height ?? 0,
        });
      return this.#project.execute(zoneGridCommand(sheet, { ...grid, frame: rect }));
    }
    const view: SheetView = { label: "", rect };
    return this.#project.execute(viewsCommand(sheet, [...sheet.views, view]));
  }
}

/** The rectangle tools of the app window. */
export const sheetRegions = new SheetRegionsStore();
