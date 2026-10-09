import {
  commands,
  events,
  type Balloon,
  type Characteristic,
  type Command,
  type CommandError,
  type DocumentInfo,
  type OpenNotice,
  type Patch,
  type Project,
  type ProjectLoaded,
  type ProjectPatched,
  type ProjectStatus,
  type ProjectStatusChanged,
  type Sheet,
} from "$lib/ipc/bindings";
import { documentStore } from "./document.svelte";
import { PatchMismatch, applyChanges, balloonsBySheet, characteristicsById } from "./patch";
import { unsavedPrompt } from "./prompt.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The project commands the store calls. The generated `commands` in the app, fakes in tests. */
export interface ProjectCommands {
  projectState(): Promise<Result<ProjectLoaded>>;
  newProject(discard: boolean): Promise<Result<boolean>>;
  openProject(discard: boolean): Promise<Result<boolean>>;
  saveProject(): Promise<Result<boolean>>;
  saveProjectAs(): Promise<Result<boolean>>;
  confirmClose(discard: boolean): Promise<Result<null>>;
  execute(command: Command): Promise<Result<ProjectPatched>>;
  undo(): Promise<Result<ProjectPatched>>;
  redo(): Promise<Result<ProjectPatched>>;
}

type Unlisten = () => void;
interface Listenable<T> {
  listen(handler: (event: { payload: T }) => void): Promise<Unlisten>;
}

/** The project events the store listens to. The generated `events` in the app. */
export interface ProjectEvents {
  projectLoaded: Listenable<ProjectLoaded>;
  projectPatched: Listenable<ProjectPatched>;
  projectStatusChanged: Listenable<ProjectStatusChanged>;
  closeRequested: Listenable<null>;
}

/** Answer to "the project has unsaved changes". */
export type UnsavedChoice = "save" | "discard" | "cancel";

const NO_BALLOONS: readonly Balloon[] = [];

/**
 * The error of a failed command as a `CommandError`. Tauri reports arguments Rust could not read,
 * for example a malformed decimal string in `update_fields`, as plain text rather than as the
 * command's error type; such text becomes an `invalid_argument` error.
 */
export function asCommandError(error: unknown): CommandError {
  if (typeof error === "object" && error !== null && "kind" in error) {
    return error as CommandError;
  }
  return { kind: "invalid_argument", message: String(error) };
}

/**
 * The open project as the frontend sees it (T1.5). Rust owns the document (ADR 0001): this store
 * only mirrors it. It is the one place where Rust's snapshots and patches are applied
 * ({@link ProjectStore.load}, {@link ProjectStore.applyPatched}); components read from it and
 * change the project only through {@link ProjectStore.execute}, undo and redo.
 *
 * `project` is replaced, never mutated: a patch gives new objects only to the lists and items it
 * changed, so unchanged characteristics and balloons keep their identity.
 */
export class ProjectStore {
  /** Session of the last applied state; `-1` before the first one. */
  session = $state(-1);
  /** Revision of the last applied patch within the session. */
  revision = $state(0);
  project = $state.raw<Project | null>(null);
  /** The drawing of the current revision, for the viewport (tiles keyed by content hash). */
  drawing = $state.raw<DocumentInfo | null>(null);
  status = $state.raw<ProjectStatus | null>(null);
  /** Shown once after opening, for example after crash recovery. */
  notice = $state.raw<OpenNotice | null>(null);
  error = $state.raw<CommandError | null>(null);
  /** True while a file dialog or file operation runs. */
  busy = $state(false);

  /** Characteristics by ID. */
  readonly characteristicById = $derived(characteristicsById(this.project));

  /** Balloons by sheet ID, in placement order. */
  readonly balloonsBySheet = $derived(balloonsBySheet(this.project));

  readonly #api: ProjectCommands;
  readonly #ask: () => Promise<UnsavedChoice>;
  readonly #onDrawing: (drawing: DocumentInfo | null) => void;

  /**
   * @param api project commands
   * @param ask asks the user what to do with unsaved changes
   * @param onDrawing called when the shown drawing changes (new project, open, close)
   */
  constructor(
    api: ProjectCommands = commands,
    ask: () => Promise<UnsavedChoice> = () => Promise.resolve("cancel"),
    onDrawing: (drawing: DocumentInfo | null) => void = () => undefined,
  ) {
    this.#api = api;
    this.#ask = ask;
    this.#onDrawing = onDrawing;
  }

  get isOpen(): boolean {
    return this.project !== null;
  }

  get characteristics(): readonly Characteristic[] {
    return this.project?.characteristics ?? [];
  }

  /** Sheets of the current drawing revision, in page order. */
  get sheets(): readonly Sheet[] {
    const project = this.project;
    return project?.revisions.find((r) => r.id === project.current_revision)?.sheets ?? [];
  }

  /** Balloons on one sheet, in placement order. */
  balloonsOnSheet(sheet: string): readonly Balloon[] {
    return this.balloonsBySheet.get(sheet) ?? NO_BALLOONS;
  }

  get canUndo(): boolean {
    return this.status?.can_undo ?? false;
  }

  get canRedo(): boolean {
    return this.status?.can_redo ?? false;
  }

  get modified(): boolean {
    return this.status?.modified ?? false;
  }

  /**
   * Starts listening to the project events, then fetches the current state. Call once at
   * startup; returns a function that stops listening.
   */
  async connect(source: ProjectEvents = events): Promise<Unlisten> {
    const stops = await Promise.all([
      source.projectLoaded.listen((e) => {
        this.load(e.payload);
      }),
      source.projectPatched.listen((e) => {
        this.applyPatched(e.payload);
      }),
      source.projectStatusChanged.listen((e) => {
        this.applyStatus(e.payload);
      }),
      source.closeRequested.listen(() => {
        void this.requestClose();
      }),
    ]);
    await this.resync(false);
    return () => {
      for (const stop of stops) {
        stop();
      }
    };
  }

  /**
   * Applies a full state from Rust. States of older sessions are ignored, and within the same
   * session states that are not newer, unless `force` is set (after a patch did not fit).
   */
  load(loaded: ProjectLoaded, force = false): void {
    if (loaded.session < this.session) {
      return;
    }
    if (loaded.notice) {
      this.notice = loaded.notice;
    }
    const snapshot = loaded.snapshot;
    if (
      !force &&
      loaded.session === this.session &&
      (snapshot?.revision ?? 0) <= this.revision &&
      (snapshot === null) === (this.project === null)
    ) {
      return;
    }
    const showDrawing =
      loaded.session !== this.session || snapshot?.drawing.doc !== this.drawing?.doc;
    this.session = loaded.session;
    this.revision = snapshot?.revision ?? 0;
    this.project = snapshot?.project ?? null;
    this.drawing = snapshot?.drawing ?? null;
    this.status = snapshot?.status ?? null;
    if (showDrawing) {
      this.#onDrawing(this.drawing);
    }
  }

  /** Applies a patch event. A gap in revisions or a patch that does not fit triggers a resync. */
  applyPatched(event: ProjectPatched): void {
    if (event.session < this.session) {
      return;
    }
    if (event.session > this.session || event.revision > this.revision + 1) {
      void this.resync(true);
      return;
    }
    if (event.revision <= this.revision) {
      return;
    }
    if (this.project === null) {
      void this.resync(true);
      return;
    }
    try {
      this.project = applyChanges(this.project, event.patch.changes);
    } catch (e) {
      if (e instanceof PatchMismatch) {
        void this.resync(true);
        return;
      }
      throw e;
    }
    this.revision = event.revision;
    this.status = event.status;
  }

  /** Applies a status event (save, autosave). */
  applyStatus(event: ProjectStatusChanged): void {
    if (event.session === this.session && this.project !== null) {
      this.status = event.status;
    }
  }

  /** Fetches the full state from Rust. */
  async resync(force: boolean): Promise<void> {
    try {
      const result = await this.#api.projectState();
      if (result.status === "ok") {
        this.load(result.data, force);
      } else {
        this.error = result.error;
      }
    } catch {
      // Outside Tauri, for example in a plain browser: no project.
    }
  }

  async #call<T>(action: () => Promise<Result<T>>): Promise<T | undefined> {
    try {
      const result = await action();
      if (result.status === "error") {
        this.error = asCommandError(result.error);
        return undefined;
      }
      this.error = null;
      return result.data;
    } catch (e) {
      this.error = { kind: "io", message: String(e) };
      return undefined;
    }
  }

  async #busy<T>(work: () => Promise<T>): Promise<T | undefined> {
    if (this.busy) {
      return undefined;
    }
    this.busy = true;
    try {
      return await work();
    } finally {
      this.busy = false;
    }
  }

  /**
   * Asks what to do with unsaved changes. Returns whether to go on and whether the changes
   * may be dropped. "Save" saves first and goes on only if the save happened.
   */
  async #settleUnsaved(): Promise<{ proceed: boolean; discard: boolean }> {
    if (!this.modified) {
      return { proceed: true, discard: false };
    }
    const choice = await this.#ask();
    if (choice === "discard") {
      return { proceed: true, discard: true };
    }
    if (choice === "save") {
      const saved = (await this.#call(() => this.#api.saveProject())) === true;
      return { proceed: saved, discard: false };
    }
    return { proceed: false, discard: false };
  }

  /** New project from a drawing chosen in the file dialog (Rust). */
  async newProject(): Promise<void> {
    await this.#busy(async () => {
      const { proceed, discard } = await this.#settleUnsaved();
      if (proceed) {
        await this.#call(() => this.#api.newProject(discard));
      }
    });
  }

  /** Opens a project chosen in the file dialog (Rust). */
  async open(): Promise<void> {
    await this.#busy(async () => {
      const { proceed, discard } = await this.#settleUnsaved();
      if (proceed) {
        await this.#call(() => this.#api.openProject(discard));
      }
    });
  }

  /** Saves; a project without a file asks for one. Resolves to whether it was saved. */
  async save(): Promise<boolean> {
    if (!this.isOpen) {
      return false;
    }
    return (await this.#busy(() => this.#call(() => this.#api.saveProject()))) === true;
  }

  /** Saves under a file chosen in the dialog. Resolves to whether it was saved. */
  async saveAs(): Promise<boolean> {
    if (!this.isOpen) {
      return false;
    }
    return (await this.#busy(() => this.#call(() => this.#api.saveProjectAs()))) === true;
  }

  /** The window is closing with unsaved changes: ask, then let Rust close it. */
  async requestClose(): Promise<void> {
    if (this.busy) {
      return;
    }
    const { proceed, discard } = await this.#settleUnsaved();
    if (proceed) {
      await this.#call(() => this.#api.confirmClose(discard));
    }
  }

  /**
   * Runs document commands one after another in call order, so fast input (typing, undo then
   * redo) reaches Rust in the order the user gave it. Each result is applied like the matching
   * `project-patched` event; whichever comes first wins, the other is ignored by revision.
   */
  #edit(send: () => Promise<Result<ProjectPatched>>): Promise<Patch | undefined> {
    const run = this.#edits.then(async () => {
      const patched = await this.#call(send);
      if (patched === undefined) {
        return undefined;
      }
      if (patched.patch.changes.length > 0) {
        this.applyPatched(patched);
      }
      return patched.patch;
    });
    this.#edits = run.catch(() => undefined);
    return run;
  }

  #edits: Promise<unknown> = Promise.resolve();

  /**
   * Executes a document command. When the promise resolves, the store already shows the change;
   * the returned patch tells the caller what was created (for example the ID of a new
   * characteristic to select). Refused commands set `error` and return `undefined`.
   */
  execute(command: Command): Promise<Patch | undefined> {
    return this.#edit(() => this.#api.execute(command));
  }

  /** Undoes the last command, if there is one when its turn comes. */
  async undo(): Promise<void> {
    await this.#edit(() =>
      this.canUndo ? this.#api.undo() : Promise.resolve({ status: "ok", data: this.#unchanged() }),
    );
  }

  /** Redoes the last undone command, if there is one when its turn comes. */
  async redo(): Promise<void> {
    await this.#edit(() =>
      this.canRedo ? this.#api.redo() : Promise.resolve({ status: "ok", data: this.#unchanged() }),
    );
  }

  /** An empty patch result, for undo or redo without a step. */
  #unchanged(): ProjectPatched {
    return {
      session: this.session,
      revision: this.revision,
      patch: { changes: [] },
      status: this.status ?? {
        file_name: null,
        modified: false,
        can_undo: false,
        can_redo: false,
        autosave: { state: "clean" },
      },
    };
  }

  dismissNotice(): void {
    this.notice = null;
  }

  dismissError(): void {
    this.error = null;
  }
}

/** The project of the app window. */
export const projectStore = new ProjectStore(
  commands,
  () => unsavedPrompt.ask(),
  (drawing) => {
    documentStore.show(drawing);
  },
);
