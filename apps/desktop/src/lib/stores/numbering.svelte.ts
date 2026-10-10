import {
  commands,
  type CharId,
  type Command,
  type CommandError,
  type InsertPolicy,
  type MultiInstance,
  type NumberingPreview,
  type NumberingSettings,
  type NumberingStrategy,
  type Patch,
  type Project,
} from "$lib/ipc/bindings";
import { asCommandError, projectStore } from "./project.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The preview command. The generated `commands` in the app, a fake in tests. */
export interface NumberingApi {
  previewNumbering(strategy: NumberingStrategy): Promise<Result<NumberingPreview>>;
}

/** What the store needs of the project: its state and the command path. */
export interface NumberingProject {
  readonly project: Project | null;
  readonly revision: number;
  execute(command: Command): Promise<Patch | undefined>;
}

/** Strategies in the order the picker lists them (FR-BAL-04). */
export const STRATEGIES: readonly NumberingStrategy[] = [
  "sheet_zone",
  "view",
  "view_clockwise",
  "kind",
  "manual",
];

/** Multi-instance numbering choices (D-22). */
export const MULTI_INSTANCE: readonly MultiInstance[] = ["quantity", "sub_number"];

/** Locked insert policies (FR-BAL-11, D-23). */
export const INSERT_POLICIES: readonly InsertPolicy[] = [
  "next_free",
  "sub_number",
  "letter_suffix",
];

/** New numbers by characteristic ID. */
export type Ghosts = Readonly<Partial<Record<CharId, string>>>;

const NO_GHOSTS: Ghosts = Object.freeze({});

/**
 * The numbering panel (T2.7, FR-BAL-04, FR-BAL-05): the picked strategy and its preview. Rust
 * computes the new order and numbers; the store only shows them as ghost numbers and sends the
 * undoable `apply_numbering` command (ADR 0001).
 */
export class NumberingStore {
  /** Strategy picked in the panel; `null` shows the strategy of the project. */
  picked = $state<NumberingStrategy | null>(null);
  /** True while ghost numbers are shown on the drawing and in the table. */
  previewing = $state(false);
  /** The last preview from Rust, for the picked strategy and the shown revision. */
  preview = $state.raw<NumberingPreview | null>(null);
  error = $state.raw<CommandError | null>(null);

  readonly #api: NumberingApi;
  readonly #project: NumberingProject;
  #sequence = 0;

  constructor(api: NumberingApi = commands, project: NumberingProject = projectStore) {
    this.#api = api;
    this.#project = project;
  }

  /** The numbering settings of the open project. */
  get settings(): NumberingSettings | null {
    return this.#project.project?.settings.numbering ?? null;
  }

  /** The strategy the panel shows: the picked one, else the project's. */
  get strategy(): NumberingStrategy {
    return this.picked ?? this.settings?.strategy ?? "sheet_zone";
  }

  /** True while numbering is locked: no preview, no apply (D-23). */
  get locked(): boolean {
    return (this.#project.project?.numbering.lock ?? null) !== null;
  }

  /** New numbers by characteristic while previewing; empty otherwise (FR-BAL-05). */
  readonly ghosts: Ghosts = $derived.by(() => {
    const preview = this.preview;
    if (!this.previewing || preview === null) {
      return NO_GHOSTS;
    }
    return Object.fromEntries(preview.entries.map((e) => [e.id, e.number]));
  });

  pick(strategy: NumberingStrategy): void {
    this.picked = strategy;
  }

  /** Turns the preview on or off. */
  setPreviewing(on: boolean): void {
    this.previewing = on;
    if (!on) {
      this.preview = null;
    }
  }

  /**
   * Asks Rust for the preview of the shown strategy. Answers of older requests are dropped, so
   * the ghosts always match the latest project state and strategy.
   */
  async refresh(): Promise<void> {
    const sequence = ++this.#sequence;
    if (!this.previewing || this.#project.project === null || this.locked) {
      this.preview = null;
      return;
    }
    try {
      const result = await this.#api.previewNumbering(this.strategy);
      if (sequence !== this.#sequence) {
        return;
      }
      if (result.status === "ok") {
        this.preview = result.data;
        this.error = null;
      } else {
        this.preview = null;
        this.error = asCommandError(result.error);
      }
    } catch (e) {
      if (sequence === this.#sequence) {
        this.preview = null;
        this.error = asCommandError(e);
      }
    }
  }

  /** Reorders and renumbers by the shown strategy as one undo step; ends the preview. */
  async apply(): Promise<boolean> {
    const patch = await this.#project.execute({ type: "apply_numbering", strategy: this.strategy });
    if (patch === undefined) {
      return false;
    }
    this.picked = null;
    this.setPreviewing(false);
    return true;
  }

  /** Changes multi-instance numbering or the locked insert policy (D-22, D-23). */
  async setSettings(change: Partial<Omit<NumberingSettings, "strategy">>): Promise<void> {
    const settings = this.settings;
    if (settings === null) {
      return;
    }
    const next = { ...settings, ...change };
    if (
      next.multi_instance !== settings.multi_instance ||
      next.insert_when_locked !== settings.insert_when_locked
    ) {
      await this.#project.execute({ type: "set_numbering_settings", settings: next });
    }
  }
}

/** The numbering panel of the app window. */
export const numberingStore = new NumberingStore();
