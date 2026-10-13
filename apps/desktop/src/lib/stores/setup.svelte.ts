/**
 * The setup pane after a new project (M3 decision 23, T3.17): general tolerance, unit and scale,
 * part number and customer. It opens by itself only for a project created from a drawing in
 * this window (`projectStore.createdSession`), never for an opened one, and the Drawing tab can
 * open it again. View state only: the answers go to Rust through the existing undoable commands
 * (`set_tolerance_settings` through the tolerance store, so the offer of decision 22 works as
 * usual, then `update_sheet` and `update_project_info` as one undo step).
 */

import type {
  Command,
  Patch,
  Project,
  ProjectInfo,
  Scale,
  Sheet,
  TableClass,
  ToleranceSettings,
  Unit,
} from "$lib/ipc/bindings";
import { sameScale, updateSheetCommand } from "$lib/sheet-properties";
import { balloonTools, type Tool } from "./balloon-tools.svelte";
import { projectStore } from "./project.svelte";
import { toleranceStore } from "./tolerance.svelte";
import { view } from "./view.svelte";

/** The answers of the pane. */
export interface SetupAnswers {
  general: TableClass | null;
  unit: Unit;
  scale: Scale;
  partNumber: string;
  customer: string;
}

/** What the store needs of the project. */
export interface SetupProject {
  readonly project: Project | null;
  readonly session: number;
  readonly createdSession: number;
  readonly sheets: readonly Sheet[];
  execute(command: Command): Promise<Patch | undefined>;
}

/** What the store needs of the tolerance settings. */
export interface SetupTolerance {
  readonly settings: ToleranceSettings | null;
  setSettings(change: Partial<ToleranceSettings>): Promise<boolean>;
}

export interface SetupDeps {
  project?: SetupProject;
  tolerance?: SetupTolerance;
  tools?: { setTool(tool: Tool): void };
  /** Switches to a ribbon tab, so the drawing and Place balloon are on screen. */
  showDrawing?: () => void;
}

/** True if two general tolerance choices name the same table version and class. */
export function sameGeneral(a: TableClass | null, b: TableClass | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  return a.table.id === b.table.id && a.table.version === b.table.version && a.class === b.class;
}

export class SetupStore {
  /** Session the user opened the pane for from the Drawing tab. */
  #openedFor = $state(-1);
  /** Session whose pane was closed with Skip or Start ballooning. */
  #closedFor = $state(-1);
  /** True while the answers are being sent. */
  busy = $state(false);

  readonly #project: SetupProject;
  readonly #tolerance: SetupTolerance;
  readonly #tools: { setTool(tool: Tool): void };
  readonly #showDrawing: () => void;

  constructor(deps: SetupDeps = {}) {
    this.#project = deps.project ?? projectStore;
    this.#tolerance = deps.tolerance ?? toleranceStore;
    this.#tools = deps.tools ?? balloonTools;
    this.#showDrawing =
      deps.showDrawing ??
      (() => {
        if (view.tab !== "home" && view.tab !== "drawing") {
          view.setTab("home");
        } else {
          view.set("drawing");
        }
      });
  }

  /** Whether the pane is shown: for a new project until it is closed, or when reopened. */
  get visible(): boolean {
    const p = this.#project;
    if (p.project === null) {
      return false;
    }
    const session = p.session;
    return (
      this.#openedFor === session || (p.createdSession === session && this.#closedFor !== session)
    );
  }

  /** Session the pane is shown for, `-1` while it is hidden; the form refills when it changes. */
  get shownFor(): number {
    return this.visible ? this.#project.session : -1;
  }

  /** Opens the pane for the open project (Drawing tab, "Set up drawing"). */
  show(): void {
    if (this.#project.project !== null) {
      this.#openedFor = this.#project.session;
      this.#showDrawing();
    }
  }

  /** Skip: closes the pane, nothing changes. */
  close(): void {
    this.#closedFor = this.#project.session;
    this.#openedFor = -1;
  }

  /** Opens or closes the pane. */
  toggle(): void {
    if (this.visible) {
      this.close();
    } else {
      this.show();
    }
  }

  /** The answers the pane starts with: what the project holds now. */
  current(): SetupAnswers | null {
    const project = this.#project.project;
    const sheet = this.#project.sheets[0];
    if (project === null || sheet === undefined) {
      return null;
    }
    return {
      general: this.#tolerance.settings?.general ?? null,
      unit: sheet.unit,
      scale: sheet.scale,
      partNumber: project.info.part_number,
      customer: project.info.customer,
    };
  }

  /**
   * Start ballooning: sends what changed, closes the pane and switches on Place balloon.
   * The general tolerance is its own undo step (and may offer to update existing
   * characteristics, decision 22); unit, scale (every sheet of the drawing) and part data are
   * one more. Resolves to false, and keeps the pane open, if Rust refused a change.
   */
  async apply(answers: SetupAnswers): Promise<boolean> {
    const project = this.#project.project;
    if (project === null || this.busy) {
      return false;
    }
    this.busy = true;
    try {
      const settings = this.#tolerance.settings;
      if (settings !== null && !sameGeneral(answers.general, settings.general)) {
        if (!(await this.#tolerance.setSettings({ general: answers.general }))) {
          return false;
        }
      }
      const commands = this.#changes(project.info, answers);
      if (commands.length > 0) {
        const command: Command =
          commands.length === 1 && commands[0] !== undefined
            ? commands[0]
            : { type: "batch", commands };
        if ((await this.#project.execute(command)) === undefined) {
          return false;
        }
      }
      this.close();
      this.#showDrawing();
      this.#tools.setTool("place");
      return true;
    } finally {
      this.busy = false;
    }
  }

  #changes(info: ProjectInfo, answers: SetupAnswers): Command[] {
    const commands: Command[] = [];
    for (const sheet of this.#project.sheets) {
      const unit = sheet.unit === answers.unit ? undefined : answers.unit;
      const scale = sameScale(sheet.scale, answers.scale) ? undefined : answers.scale;
      if (unit !== undefined || scale !== undefined) {
        commands.push(updateSheetCommand(sheet, { unit, scale }));
      }
    }
    if (info.part_number !== answers.partNumber || info.customer !== answers.customer) {
      commands.push({
        type: "update_project_info",
        info: { ...info, part_number: answers.partNumber, customer: answers.customer },
      });
    }
    return commands;
  }
}

/** The setup pane of the app window. */
export const setupStore = new SetupStore();
