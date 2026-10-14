/**
 * Exports of the open project (T1.9, FR-EXP-01, FR-EXP-09; reports T3.8, FR-EXP-04). Rust asks
 * for the file, runs the export as a background job and reports `job-progress` and
 * `job-finished`; this store keeps the state of the last job per format for the export view.
 *
 * Reports exported as issued first ask Rust for warnings (draft tables, open results, limits
 * changed since the last issued report, M3 decision 17); the view shows them and the user
 * confirms or goes back. They never block the export.
 */

import {
  commands,
  events,
  type CommandError,
  type ExportFormat,
  type ExportRequest,
  type IssuedRecord,
  type JobFinished,
  type JobProgress,
  type ReportFormat,
  type ReportKind,
  type ReportWarnings,
} from "$lib/ipc/bindings";
import { asCommandError } from "$lib/errors";
import { settingsStore } from "./settings.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** The drawing and list exports, one card each. */
export const EXPORT_FORMATS: readonly ExportFormat[] = ["ballooned_pdf", "csv", "xlsx"];

/** The reports, one card each with a format choice. */
export const REPORT_KINDS: readonly ReportKind[] = ["check_sheet", "inspection_report"];

/** Every report export, for the dev script report. */
export const REPORT_FORMATS: readonly ExportFormat[] = [
  "check_sheet_pdf",
  "check_sheet_xlsx",
  "inspection_report_pdf",
  "inspection_report_xlsx",
];

/** The export format of report `kind` in file `format`. */
export function reportExport(kind: ReportKind, format: ReportFormat): ExportFormat {
  return `${kind}_${format}`;
}

/** State of one export job. */
export type JobState =
  | { state: "running"; fraction: number }
  | {
      state: "done";
      fileName: string;
      issued: IssuedRecord | null;
      openError: CommandError | null;
    }
  | { state: "failed"; error: CommandError }
  | { state: "cancelled" };

/** Warnings waiting for the user before an issued report export. */
export interface PendingWarnings {
  kind: ReportKind;
  format: ExportFormat;
  warnings: ReportWarnings;
  /** Export as issued, as chosen when the warnings were asked for. */
  issued: boolean;
}

/** What the store needs from Rust, replaceable in tests. */
export interface ExportApi {
  exportProject: (request: ExportRequest) => Promise<Result<number | null>>;
  cancelJob: (id: number) => Promise<boolean>;
  reportExportWarnings: (kind: ReportKind) => Promise<Result<ReportWarnings>>;
  onProgress: (handler: (p: JobProgress) => void) => Promise<() => void>;
  onFinished: (handler: (f: JobFinished) => void) => Promise<() => void>;
}

const tauriApi: ExportApi = {
  exportProject: (request) => commands.exportProject(request),
  cancelJob: (id) => commands.cancelJob(id),
  reportExportWarnings: (kind) => commands.reportExportWarnings(kind),
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
  /** Export as issued: locks the numbering (D-23); reports are also recorded. */
  issued = $state(false);
  /** Warnings shown before an issued report export, until confirmed or dismissed. */
  pending = $state<PendingWarnings | null>(null);
  /** The report the Report ribbon acts on (T3.15). View state only. */
  document = $state<ReportKind>("inspection_report");
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
        this.#set(f.id, finishedState(f));
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

  /** The export format of report `kind` in the remembered file format. */
  reportFormat(kind: ReportKind): ExportFormat {
    const prefs = settingsStore.current.export;
    return reportExport(
      kind,
      kind === "check_sheet" ? prefs.check_sheet_format : prefs.inspection_report_format,
    );
  }

  /**
   * Asks for a file and starts the export with the stored options; `issued` defaults to the
   * check box.
   */
  async start(format: ExportFormat, issued: boolean = this.issued): Promise<void> {
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
        blank_columns: prefs.blank_columns,
        issued,
        open_after: prefs.open_after_export,
      });
      if (result.status === "error") {
        this.startError[format] = result.error;
      } else if (result.data !== null) {
        this.latest[format] = result.data;
      }
    } catch (e) {
      this.startError[format] = asCommandError(e);
    } finally {
      this.asking[format] = false;
    }
  }

  /**
   * Exports report `kind` in its remembered format. Exported as issued, Rust's warnings are
   * shown first (`pending`) when there are any.
   */
  async startReport(kind: ReportKind, issued: boolean = this.issued): Promise<void> {
    const format = this.reportFormat(kind);
    if (this.busy(format)) {
      return;
    }
    this.pending = null;
    if (issued) {
      this.startError[format] = undefined;
      const result = await this.#api.reportExportWarnings(kind);
      if (result.status === "error") {
        this.startError[format] = result.error;
        return;
      }
      const w = result.data;
      if (w.draft_limits || w.open_results || w.limits_changed) {
        this.pending = { kind, format, warnings: w, issued: true };
        return;
      }
    }
    await this.start(format, issued);
  }

  /** The user confirmed the warnings: export anyway. */
  async confirmPending(): Promise<void> {
    const pending = this.pending;
    this.pending = null;
    if (pending) {
      await this.start(pending.format, pending.issued);
    }
  }

  /** The user went back from the warnings. */
  dismissPending(): void {
    this.pending = null;
  }

  /** Asks Rust to stop the running job of `format` before its next step. */
  async cancel(format: ExportFormat): Promise<void> {
    const id = this.latest[format];
    if (id !== undefined && this.job(format)?.state === "running") {
      await this.#api.cancelJob(id);
    }
  }
}

function finishedState(f: JobFinished): JobState {
  switch (f.outcome.state) {
    case "done":
      return {
        state: "done",
        fileName: f.outcome.file_name,
        issued: f.outcome.issued,
        openError: f.outcome.open_error,
      };
    case "failed":
      return { state: "failed", error: f.outcome.error };
    case "cancelled":
      return { state: "cancelled" };
  }
}

export const exportStore = new ExportStore();
