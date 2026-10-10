/**
 * Tolerance settings, explanations, re-interpretation and history (T2.8, FR-TOL-07, FR-TOL-08,
 * FR-CHR-10). Rust owns every rule (ADR 0001): it lists the tables and their classes, checks
 * imported tables, builds explanations in the UI language and re-interprets characteristics.
 * This store only asks and sends commands; settings change with the undoable
 * `set_tolerance_settings` command.
 */

import {
  commands,
  type CharId,
  type Command,
  type CommandError,
  type ExplainLanguage,
  type HistoryEntry,
  type Patch,
  type Project,
  type ProjectPatched,
  type Reinterpreted,
  type TableImport,
  type ToleranceSettings,
  type ToleranceTableInfo,
} from "$lib/ipc/bindings";
import { asCommandError, projectStore } from "./project.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The tolerance commands. The generated `commands` in the app, fakes in tests. */
export interface ToleranceApi {
  toleranceTables(): Promise<Result<ToleranceTableInfo[]>>;
  explainCharacteristic(id: CharId, language: ExplainLanguage): Promise<Result<string | null>>;
  characteristicHistory(id: CharId): Promise<Result<HistoryEntry[]>>;
  reinterpretCharacteristics(ids: CharId[]): Promise<Result<Reinterpreted>>;
  importToleranceTable(): Promise<Result<TableImport>>;
}

/** What the store needs of the project: its state, the command path and patch results. */
export interface ToleranceProject {
  readonly project: Project | null;
  execute(command: Command): Promise<Patch | undefined>;
  applyPatched(event: ProjectPatched): void;
}

/** A refused table file: Rust's message and the line, if known. */
export interface ImportRefusal {
  message: string;
  line: number | null;
}

async function call<T>(action: () => Promise<Result<T>>): Promise<Result<T>> {
  try {
    const result = await action();
    return result.status === "ok"
      ? result
      : { status: "error", error: asCommandError(result.error) };
  } catch (e) {
    return { status: "error", error: asCommandError(e) };
  }
}

export class ToleranceStore {
  /** Shipped and custom tables the settings can name, from Rust. */
  tables = $state.raw<readonly ToleranceTableInfo[]>([]);
  /** Why the last table import was refused, `null` after a good one. */
  importRefusal = $state.raw<ImportRefusal | null>(null);
  /** The table imported last, for a confirmation line. */
  imported = $state<string | null>(null);
  /** Counts of the last re-interpretation, for a status line. */
  lastReinterpretation = $state.raw<Reinterpreted | null>(null);
  error = $state.raw<CommandError | null>(null);
  busy = $state(false);

  readonly #api: ToleranceApi;
  readonly #project: ToleranceProject;
  #sequence = 0;

  constructor(api: ToleranceApi = commands, project: ToleranceProject = projectStore) {
    this.#api = api;
    this.#project = project;
  }

  /** The tolerance settings of the open project. */
  get settings(): ToleranceSettings | null {
    return this.#project.project?.settings.tolerance ?? null;
  }

  /** Tables usable as general tolerance: general and custom tables (M2 decision 2). */
  get generalTables(): ToleranceTableInfo[] {
    return this.tables.filter((t) => t.usage !== "fit");
  }

  /** Tables usable as drawing rule: custom tables of the project. */
  get customTables(): ToleranceTableInfo[] {
    return this.tables.filter((t) => t.usage === "custom");
  }

  /** Asks Rust for the tables again; older answers are dropped. */
  async refreshTables(): Promise<void> {
    const sequence = ++this.#sequence;
    if (this.#project.project === null) {
      this.tables = [];
      return;
    }
    const result = await call(() => this.#api.toleranceTables());
    if (sequence !== this.#sequence) {
      return;
    }
    if (result.status === "ok") {
      this.tables = result.data;
    } else {
      this.error = result.error;
    }
  }

  /** Sets tolerance settings as one undo step. Limits of characteristics do not change. */
  async setSettings(change: Partial<ToleranceSettings>): Promise<boolean> {
    const settings = this.settings;
    if (settings === null) {
      return false;
    }
    const patch = await this.#project.execute({
      type: "set_tolerance_settings",
      settings: { ...settings, ...change },
    });
    return patch !== undefined;
  }

  /** Lets the user pick a custom table file; Rust reads, checks and stores it. */
  async importTable(): Promise<void> {
    if (this.busy) {
      return;
    }
    this.busy = true;
    try {
      const result = await call(() => this.#api.importToleranceTable());
      if (result.status === "error") {
        this.error = result.error;
        return;
      }
      this.error = null;
      const outcome = result.data;
      switch (outcome.result) {
        case "cancelled":
          break;
        case "invalid":
          this.imported = null;
          this.importRefusal = { message: outcome.message, line: outcome.line };
          break;
        case "imported":
          this.importRefusal = null;
          this.imported = `${outcome.table.id} v${String(outcome.table.version)}`;
          this.#project.applyPatched(outcome.patched);
          await this.refreshTables();
          break;
      }
    } finally {
      this.busy = false;
    }
  }

  /** Reads `ids` again with the current settings, one undo step. Manual limits stay. */
  async reinterpret(ids: Iterable<CharId>): Promise<Reinterpreted | null> {
    const list = [...ids];
    if (list.length === 0 || this.busy) {
      return null;
    }
    this.busy = true;
    try {
      const result = await call(() => this.#api.reinterpretCharacteristics(list));
      if (result.status === "error") {
        this.error = result.error;
        return null;
      }
      this.error = null;
      this.#project.applyPatched(result.data.patched);
      this.lastReinterpretation = result.data;
      return result.data;
    } finally {
      this.busy = false;
    }
  }

  /** The explanation of a characteristic's limits in `language`, `null` without a rule. */
  async explanation(id: CharId, language: ExplainLanguage): Promise<Result<string | null>> {
    return call(() => this.#api.explainCharacteristic(id, language));
  }

  /** The change history of a characteristic, oldest first (FR-CHR-10). */
  async history(id: CharId): Promise<Result<HistoryEntry[]>> {
    return call(() => this.#api.characteristicHistory(id));
  }

  dismiss(): void {
    this.error = null;
    this.importRefusal = null;
    this.imported = null;
    this.lastReinterpretation = null;
  }
}

/** Tolerance state of the app window. */
export const toleranceStore = new ToleranceStore();
