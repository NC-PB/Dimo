/**
 * Tolerance settings, explanations, re-interpretation and history (T2.8, FR-TOL-07, FR-TOL-08,
 * FR-CHR-10). Rust owns every rule (ADR 0001): it lists the tables and their classes, checks
 * imported tables, builds explanations in the UI language, re-interprets characteristics and
 * sets the shorter leg of an angle (T2.9a, `set_shorter_leg`). After the general tolerance
 * changed, Rust lists the characteristics it would update and updates them on request (T3.13,
 * M3 decision 22). This store only asks and sends commands; settings change with the undoable
 * `set_tolerance_settings` command.
 */

import {
  commands,
  type CharId,
  type Characteristic,
  type Command,
  type CommandError,
  type Language,
  type HistoryEntry,
  type LegOutcome,
  type LegSet,
  type Patch,
  type Project,
  type ProjectPatched,
  type Reinterpreted,
  type SheetId,
  type TableClass,
  type TableImport,
  type TableImportReason,
  type ToleranceSettings,
  type ToleranceTableInfo,
} from "$lib/ipc/bindings";
import { asCommandError, projectStore } from "./project.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The tolerance commands. The generated `commands` in the app, fakes in tests. */
export interface ToleranceApi {
  toleranceTables(): Promise<Result<ToleranceTableInfo[]>>;
  explainCharacteristic(id: CharId, language: Language): Promise<Result<string | null>>;
  characteristicHistory(id: CharId): Promise<Result<HistoryEntry[]>>;
  reinterpretCharacteristics(ids: CharId[]): Promise<Result<Reinterpreted>>;
  importToleranceTable(): Promise<Result<TableImport>>;
  setShorterLeg(id: CharId, leg: string | null): Promise<Result<LegSet>>;
  generalToleranceCandidates(): Promise<Result<CharId[]>>;
  applyGeneralTolerance(): Promise<Result<Reinterpreted>>;
}

/**
 * The sheet a characteristic sits on: the sheet of its first balloon, else of its first source
 * region (D-20: the unit of that sheet applies). `null` if it has neither. Used only to label
 * the leg field with the unit Rust parses a new leg in.
 */
export function sheetOf(project: Project, c: Characteristic): SheetId | null {
  return (
    project.balloons.find((b) => b.characteristic === c.id)?.sheet ?? c.sources[0]?.sheet ?? null
  );
}

/** What the store needs of the project: its state, the command path and patch results. */
export interface ToleranceProject {
  readonly project: Project | null;
  /** Revision of the open project; every applied patch raises it. */
  readonly revision: number;
  execute(command: Command): Promise<Patch | undefined>;
  executeWith(build: (project: Project) => Command | null): Promise<Patch | undefined>;
  applyPatched(event: ProjectPatched): void;
}

/** A refused table file: the reason, the line if known, and Rust's diagnostic message. */
export interface ImportRefusal {
  reason: TableImportReason;
  line: number | null;
  message: string;
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
  /** True if `imported` was in the project already, byte for byte: nothing changed. */
  importUnchanged = $state(false);
  /** Counts of the last re-interpretation, for a status line. */
  lastReinterpretation = $state.raw<Reinterpreted | null>(null);
  error = $state.raw<CommandError | null>(null);
  busy = $state(false);
  /** The characteristic whose typed shorter leg Rust refused last, and why (T2.9a). */
  legError = $state.raw<{ id: CharId; error: CommandError } | null>(null);
  /** What the last shorter leg did with the limits of that characteristic (T2.9a). */
  legOutcome = $state.raw<{ id: CharId; outcome: LegOutcome } | null>(null);
  /**
   * How many characteristics Rust would give the new general tolerance, offered after the
   * general tolerance changed (M3 decision 22), and whether the general tolerance was removed
   * (the update then takes the previous one away); `null` when there is no offer. The offer
   * holds only for the project revision it was counted on: undo, redo or any other change
   * withdraws it, so the number shown is always the number Update changes.
   */
  get generalOffer(): { count: number; removing: boolean } | null {
    const offer = this.#offer;
    return offer !== null && offer.revision === this.#project.revision ? offer : null;
  }
  #offer = $state.raw<{ count: number; removing: boolean; revision: number } | null>(null);
  /** How many characteristics the last accepted offer updated, for a confirmation line. */
  generalUpdated = $state<number | null>(null);

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

  /**
   * Sets tolerance settings as one undo step. Limits of characteristics do not change. If the
   * general tolerance changed, asks Rust which characteristics it would update and offers that
   * (M3 decision 22); other edits withdraw an open offer.
   */
  async setSettings(change: Partial<ToleranceSettings>): Promise<boolean> {
    const settings = this.settings;
    if (settings === null) {
      return false;
    }
    const generalChanged =
      "general" in change && !sameChoice(change.general ?? null, settings.general);
    this.#offer = null;
    this.generalUpdated = null;
    // Built when the command runs, so a fast second edit keeps the first one (T3.18).
    const patch = await this.#project.executeWith((project) => ({
      type: "set_tolerance_settings",
      settings: { ...project.settings.tolerance, ...change },
    }));
    if (patch === undefined) {
      return false;
    }
    if (generalChanged) {
      await this.#offerGeneralUpdate((change.general ?? null) === null);
    }
    return true;
  }

  async #offerGeneralUpdate(removing: boolean): Promise<void> {
    const result = await call(() => this.#api.generalToleranceCandidates());
    if (result.status === "error") {
      this.error = result.error;
      return;
    }
    const count = result.data.length;
    this.#offer = count > 0 ? { count, removing, revision: this.#project.revision } : null;
  }

  /** Not now: the offer goes away, nothing changes. */
  declineGeneralUpdate(): void {
    this.#offer = null;
  }

  /**
   * Update: Rust gives the characteristics without tolerance or with a general tolerance the
   * current general tolerance, one undo step; limits entered by hand stay.
   */
  async applyGeneralUpdate(): Promise<Reinterpreted | null> {
    if (this.busy) {
      return null;
    }
    this.busy = true;
    try {
      const result = await call(() => this.#api.applyGeneralTolerance());
      if (result.status === "error") {
        this.error = result.error;
        return null;
      }
      this.error = null;
      this.#offer = null;
      this.generalUpdated = result.data.reinterpreted;
      this.#project.applyPatched(result.data.patched);
      return result.data;
    } finally {
      this.busy = false;
    }
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
        case "unchanged":
          this.importRefusal = null;
          this.imported = `${outcome.table.id} v${String(outcome.table.version)}`;
          this.importUnchanged = true;
          break;
        case "invalid":
          this.imported = null;
          this.importUnchanged = false;
          this.importRefusal = {
            reason: outcome.reason,
            line: outcome.line,
            message: outcome.message,
          };
          break;
        case "imported":
          this.importRefusal = null;
          this.imported = `${outcome.table.id} v${String(outcome.table.version)}`;
          this.importUnchanged = false;
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

  /**
   * Sets the shorter leg of an angle as typed in the sheet unit (T2.9a); empty text removes it.
   * Rust parses it, reads the requirement text again with it (limits entered by hand stay) and
   * stores both as one undo step. Returns false if Rust refused the leg.
   */
  async setShorterLeg(id: CharId, text: string): Promise<boolean> {
    const leg = text.trim() === "" ? null : text;
    const result = await call(() => this.#api.setShorterLeg(id, leg));
    if (result.status === "error") {
      this.legError = { id, error: result.error };
      this.legOutcome = null;
      return false;
    }
    this.legError = null;
    this.legOutcome = { id, outcome: result.data.outcome };
    this.#project.applyPatched(result.data.patched);
    return true;
  }

  /** The explanation of a characteristic's limits in `language`, `null` without a rule. */
  async explanation(id: CharId, language: Language): Promise<Result<string | null>> {
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
    this.importUnchanged = false;
    this.lastReinterpretation = null;
    this.legError = null;
    this.legOutcome = null;
    this.#offer = null;
    this.generalUpdated = null;
  }
}

/** True if two table and class choices name the same table version and class. */
function sameChoice(a: TableClass | null, b: TableClass | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  return a.table.id === b.table.id && a.table.version === b.table.version && a.class === b.class;
}

/** Tolerance state of the app window. */
export const toleranceStore = new ToleranceStore();
