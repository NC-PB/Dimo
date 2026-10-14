/**
 * State of the Report tab (T3.19, ADR 0007): the live preview of the chosen report, the checks
 * before issuing and Print. Rust renders the preview through the same path as "Save copy"
 * (`report_preview`), counts the checks (`report_checks`) and prints (`print_report`); this
 * store only asks, debounces and drops late answers. View state only (rule 2).
 */

import {
  commands,
  type CommandError,
  type DocumentInfo,
  type ReportChecks,
  type ReportKind,
  type ReportView,
} from "$lib/ipc/bindings";
import { asCommandError } from "$lib/errors";
import { exportStore } from "$lib/stores/export.svelte";
import { settingsStore } from "$lib/stores/settings.svelte";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** What the store needs from Rust, replaceable in tests. */
export interface ReportViewApi {
  reportPreview: (view: ReportView) => Promise<Result<DocumentInfo>>;
  reportChecks: (kind: ReportKind) => Promise<Result<ReportChecks>>;
  printReport: (view: ReportView) => Promise<Result<string>>;
}

const tauriApi: ReportViewApi = {
  reportPreview: (view) => commands.reportPreview(view),
  reportChecks: (kind) => commands.reportChecks(kind),
  printReport: (view) => commands.printReport(view),
};

/** Wait after the last change before the preview is rendered again. */
export const PREVIEW_DELAY_MS = 300;

/** State of the last Print. */
export type PrintState =
  | { state: "idle" }
  | { state: "running" }
  | { state: "done"; fileName: string }
  | { state: "failed"; error: CommandError };

/** The report the Report tab shows: the chosen document with the remembered export options. */
export function currentView(): ReportView {
  const prefs = settingsStore.current.export;
  return {
    kind: exportStore.document,
    language: prefs.language,
    blank_columns: prefs.blank_columns,
  };
}

export class ReportViewStore {
  /** The rendered preview: content hash and page sizes, `null` before the first answer. */
  preview = $state.raw<DocumentInfo | null>(null);
  /** Why the last preview failed, `null` if it worked. */
  previewError = $state.raw<CommandError | null>(null);
  /** True while a preview is waiting or rendering. */
  loading = $state(false);
  /** The checks before issuing, for the shown document. */
  checks = $state.raw<ReportChecks | null>(null);
  /** Why the last check failed. */
  checksError = $state.raw<CommandError | null>(null);
  /** The last Print. */
  print = $state.raw<PrintState>({ state: "idle" });

  readonly #api: ReportViewApi;
  readonly #delay: number;
  #seq = 0;
  #timer: ReturnType<typeof setTimeout> | undefined;

  constructor(api: ReportViewApi = tauriApi, delay: number = PREVIEW_DELAY_MS) {
    this.#api = api;
    this.#delay = delay;
  }

  /**
   * Renders preview and checks of `view` again after a short pause, so typing in a field or a
   * quick series of changes renders once.
   */
  schedule(view: ReportView): void {
    clearTimeout(this.#timer);
    this.loading = true;
    this.#timer = setTimeout(() => void this.refresh(view), this.#delay);
  }

  /** Renders preview and checks of `view` now. A late answer of an older call is dropped. */
  async refresh(view: ReportView): Promise<void> {
    clearTimeout(this.#timer);
    const seq = ++this.#seq;
    this.loading = true;
    let preview: Result<DocumentInfo>;
    let checks: Result<ReportChecks>;
    try {
      [preview, checks] = await Promise.all([
        this.#api.reportPreview(view),
        this.#api.reportChecks(view.kind),
      ]);
    } catch (e) {
      // Outside Tauri (a plain browser) the calls throw instead of answering.
      const error: CommandError = asCommandError(e);
      preview = { status: "error", error };
      checks = { status: "error", error };
    }
    if (seq !== this.#seq) {
      return;
    }
    this.loading = false;
    if (preview.status === "ok") {
      this.preview = preview.data;
      this.previewError = null;
    } else {
      this.previewError = preview.error;
    }
    if (checks.status === "ok") {
      this.checks = checks.data;
      this.checksError = null;
    } else {
      this.checksError = checks.error;
    }
  }

  /** Forgets everything, for a closed project. */
  clear(): void {
    clearTimeout(this.#timer);
    this.#seq += 1;
    this.preview = null;
    this.previewError = null;
    this.checks = null;
    this.checksError = null;
    this.loading = false;
  }

  /** Opens the PDF of `view` with the system viewer for printing. */
  async printReport(view: ReportView): Promise<void> {
    if (this.print.state === "running") {
      return;
    }
    this.print = { state: "running" };
    try {
      const result = await this.#api.printReport(view);
      this.print =
        result.status === "ok"
          ? { state: "done", fileName: result.data }
          : { state: "failed", error: result.error };
    } catch (e) {
      this.print = { state: "failed", error: asCommandError(e) };
    }
  }
}

export const reportView = new ReportViewStore();
