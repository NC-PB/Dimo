/**
 * Exports of the open project (T1.9, FR-EXP-01, FR-EXP-09). Rust asks for the file, runs the
 * export as a background job and reports `job-progress` and `job-finished`; this store keeps
 * the state of the last job per format for the export view.
 */

import {
  commands,
  events,
  type CommandError,
  type ExportFormat,
  type ExportRequest,
  type JobFinished,
  type JobProgress,
} from "$lib/ipc/bindings";
import { settingsStore } from "./settings.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

export const EXPORT_FORMATS: readonly ExportFormat[] = ["ballooned_pdf", "csv", "xlsx"];

/** State of one export job. */
export type JobState =
  | { state: "running"; fraction: number }
  | { state: "done"; fileName: string }
  | { state: "failed"; error: CommandError };

/** What the store needs from Rust, replaceable in tests. */
export interface ExportApi {
  exportProject: (request: ExportRequest) => Promise<Result<number | null>>;
  onProgress: (handler: (p: JobProgress) => void) => Promise<() => void>;
  onFinished: (handler: (f: JobFinished) => void) => Promise<() => void>;
}

const tauriApi: ExportApi = {
  exportProject: (request) => commands.exportProject(request),
  onProgress: (handler) => events.jobProgress.listen((e) => handler(e.payload)),
  onFinished: (handler) => events.jobFinished.listen((e) => handler(e.payload)),
};

export class ExportStore {
  /** Job states by job ID. Events may arrive before the command returns the ID. */
  jobs = $state<Record<number, JobState>>({});
  /** The last job of each format. */
  latest = $state<Partial<Record<ExportFormat, number>>>({});
  /** Formats whose save dialog is open. */
  asking = $state<Partial<Record<ExportFormat, boolean>>>({});
  /** Errors from starting an export (before a job exists). */
  startError = $state<Partial<Record<ExportFormat, CommandError>>>({});
  /** Export as issued: locks the numbering first (D-23). */
  issued = $state(false);
  readonly #api: ExportApi;

  constructor(api: ExportApi = tauriApi) {
    this.#api = api;
  }

  /** Listens to the job events. Returns the function that stops listening. */
  async connect(): Promise<() => void> {
    const stops = await Promise.all([
      this.#api.onProgress((p) => {
        this.#set(p.id, { state: "running", fraction: p.fraction ?? 0 });
      }),
      this.#api.onFinished((f) => {
        this.#set(
          f.id,
          f.outcome.state === "done"
            ? { state: "done", fileName: f.outcome.file_name }
            : { state: "failed", error: f.outcome.error },
        );
      }),
    ]);
    return () => {
      for (const stop of stops) {
        stop();
      }
    };
  }

  #set(id: number, state: JobState): void {
    const current = this.jobs[id];
    // A late progress event never hides the result.
    if (current && current.state !== "running" && state.state === "running") {
      return;
    }
    this.jobs[id] = state;
  }

  /** State of the last job of `format`, `null` if there was none. */
  job(format: ExportFormat): JobState | null {
    const id = this.latest[format];
    if (id === undefined) {
      return null;
    }
    return this.jobs[id] ?? { state: "running", fraction: 0 };
  }

  /** True while a dialog or a job of `format` runs. */
  busy(format: ExportFormat): boolean {
    return this.asking[format] === true || this.job(format)?.state === "running";
  }

  /** Asks for a file and starts the export with the stored options. */
  async start(format: ExportFormat): Promise<void> {
    if (this.busy(format)) {
      return;
    }
    const prefs = settingsStore.current.export;
    this.asking[format] = true;
    this.startError[format] = undefined;
    try {
      const result = await this.#api.exportProject({
        format,
        language: prefs.language,
        pdf_balloons: prefs.pdf_balloons,
        issued: this.issued,
      });
      if (result.status === "error") {
        this.startError[format] = result.error;
      } else if (result.data !== null) {
        this.latest[format] = result.data;
      }
    } catch (e) {
      this.startError[format] = { kind: "io", message: String(e) };
    } finally {
      this.asking[format] = false;
    }
  }
}

export const exportStore = new ExportStore();
