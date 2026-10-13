import { afterEach, describe, expect, it } from "vitest";
import type {
  ExportRequest,
  JobFinished,
  JobProgress,
  ReportKind,
  ReportWarnings,
} from "$lib/ipc/bindings";
import { DEFAULT_SETTINGS, settingsStore } from "./settings.svelte";
import { ExportStore, reportExport, type ExportApi } from "./export.svelte";

const NO_WARNINGS: ReportWarnings = {
  draft_limits: false,
  open_results: false,
  limits_changed: false,
};

/** Rust's side of the export: records requests and lets the test send job events. */
function fakeApi(answer: number | null = 7, warnings: ReportWarnings = NO_WARNINGS) {
  const requests: ExportRequest[] = [];
  const cancelled: number[] = [];
  const asked: ReportKind[] = [];
  let progress: ((p: JobProgress) => void) | null = null;
  let finished: ((f: JobFinished) => void) | null = null;
  let beforeAnswer: (() => void) | null = null;
  const api: ExportApi = {
    exportProject: (request) => {
      requests.push(request);
      beforeAnswer?.();
      return Promise.resolve({ status: "ok", data: answer });
    },
    cancelJob: (id) => {
      cancelled.push(id);
      return Promise.resolve(true);
    },
    reportExportWarnings: (kind) => {
      asked.push(kind);
      return Promise.resolve({ status: "ok", data: warnings });
    },
    onProgress: (handler) => {
      progress = handler;
      return Promise.resolve(() => (progress = null));
    },
    onFinished: (handler) => {
      finished = handler;
      return Promise.resolve(() => (finished = null));
    },
  };
  return {
    api,
    requests,
    cancelled,
    asked,
    progress: (p: JobProgress) => progress?.(p),
    finished: (f: JobFinished) => finished?.(f),
    /** Runs `action` while the command has not answered yet. */
    beforeAnswer: (action: () => void) => (beforeAnswer = action),
  };
}

describe("export store (T1.9)", () => {
  it("starts a job with the stored options and follows its events", async () => {
    const rust = fakeApi();
    const store = new ExportStore(rust.api);
    await store.connect();
    store.issued = true;
    await store.start("csv");
    expect(rust.requests).toEqual([
      {
        format: "csv",
        language: "en",
        pdf_balloons: "page_content",
        blank_columns: 3,
        issued: true,
        open_after: false,
      },
    ]);
    expect(store.job("csv")).toEqual({ state: "running", fraction: 0 });
    expect(store.busy("csv")).toBe(true);
    rust.progress({ id: 7, fraction: 0.8, message: "writing" });
    expect(store.job("csv")).toEqual({ state: "running", fraction: 0.8 });
    rust.finished({
      id: 7,
      outcome: {
        state: "done",
        file_name: "part.csv",
        issued: null,
        open_error: null,
      },
    });
    expect(store.job("csv")).toEqual({
      state: "done",
      fileName: "part.csv",
      issued: null,
      openError: null,
    });
    expect(store.busy("csv")).toBe(false);
    expect(store.job("xlsx")).toBeNull();
  });

  it("keeps events that arrive before the job ID, and ignores late progress", async () => {
    const rust = fakeApi(3);
    const store = new ExportStore(rust.api);
    await store.connect();
    rust.beforeAnswer(() => {
      rust.finished({
        id: 3,
        outcome: { state: "failed", error: { kind: "export", message: "disk full" } },
      });
      rust.progress({ id: 3, fraction: 0.8, message: "writing" });
    });
    await store.start("ballooned_pdf");
    expect(store.job("ballooned_pdf")).toEqual({
      state: "failed",
      error: { kind: "export", message: "disk full" },
    });
  });

  it("does nothing when the dialog is cancelled", async () => {
    const rust = fakeApi(null);
    const store = new ExportStore(rust.api);
    await store.start("xlsx");
    expect(store.job("xlsx")).toBeNull();
    expect(store.busy("xlsx")).toBe(false);
  });

  it("shows a refusal before a job exists", async () => {
    const store = new ExportStore({
      ...fakeApi().api,
      exportProject: () => Promise.resolve({ status: "error", error: { kind: "no_project" } }),
    });
    await store.start("csv");
    expect(store.startError.csv).toEqual({ kind: "no_project" });
    expect(store.job("csv")).toBeNull();
  });
});

describe("report exports (T3.8)", () => {
  it("maps report kind and file format to the export format", () => {
    expect(reportExport("check_sheet", "pdf")).toBe("check_sheet_pdf");
    expect(reportExport("inspection_report", "xlsx")).toBe("inspection_report_xlsx");
  });

  it("exports a report in the remembered format without asking for warnings", async () => {
    const rust = fakeApi(9);
    const store = new ExportStore(rust.api);
    await store.connect();
    await store.startReport("inspection_report");
    expect(rust.asked).toEqual([]);
    expect(rust.requests.map((r) => r.format)).toEqual(["inspection_report_pdf"]);
    expect(store.busy("inspection_report_pdf")).toBe(true);
    await store.cancel("inspection_report_pdf");
    expect(rust.cancelled).toEqual([9]);
    rust.finished({ id: 9, outcome: { state: "cancelled" } });
    expect(store.job("inspection_report_pdf")).toEqual({ state: "cancelled" });
    expect(store.busy("inspection_report_pdf")).toBe(false);
    await store.cancel("inspection_report_pdf");
    expect(rust.cancelled).toEqual([9]);
  });

  it("shows warnings before an issued report and exports only when confirmed", async () => {
    const rust = fakeApi(5, { ...NO_WARNINGS, open_results: true });
    const store = new ExportStore(rust.api);
    await store.connect();
    store.issued = true;
    await store.startReport("check_sheet");
    expect(rust.asked).toEqual(["check_sheet"]);
    expect(store.pending).toEqual({
      kind: "check_sheet",
      format: "check_sheet_pdf",
      warnings: { ...NO_WARNINGS, open_results: true },
      issued: true,
    });
    expect(rust.requests).toEqual([]);
    store.dismissPending();
    expect(store.pending).toBeNull();
    expect(rust.requests).toEqual([]);

    await store.startReport("check_sheet");
    // Unticking the box while the warnings are open does not change what was confirmed.
    store.issued = false;
    await store.confirmPending();
    expect(store.pending).toBeNull();
    expect(rust.requests.map((r) => [r.format, r.issued])).toEqual([["check_sheet_pdf", true]]);
    rust.finished({
      id: 5,
      outcome: {
        state: "done",
        file_name: "p.pdf",
        issued: { state: "recorded" },
        open_error: null,
      },
    });
    expect(store.job("check_sheet_pdf")).toEqual({
      state: "done",
      fileName: "p.pdf",
      issued: { state: "recorded" },
      openError: null,
    });
  });

  it("exports an issued report at once when there is nothing to warn about", async () => {
    const rust = fakeApi(6);
    const store = new ExportStore(rust.api);
    store.issued = true;
    await store.startReport("inspection_report");
    expect(rust.asked).toEqual(["inspection_report"]);
    expect(store.pending).toBeNull();
    expect(rust.requests.map((r) => r.format)).toEqual(["inspection_report_pdf"]);
  });
});

describe("open after export", () => {
  afterEach(() => {
    settingsStore.current = DEFAULT_SETTINGS;
  });

  it("sends the remembered preference and keeps an open error", async () => {
    const rust = fakeApi(9);
    const store = new ExportStore(rust.api);
    await store.connect();
    settingsStore.current = {
      ...DEFAULT_SETTINGS,
      export: { ...DEFAULT_SETTINGS.export, open_after_export: true },
    };
    await store.start("xlsx");
    expect(rust.requests.map((r) => r.open_after)).toEqual([true]);
    rust.finished({
      id: 9,
      outcome: {
        state: "done",
        file_name: "p.xlsx",
        issued: null,
        open_error: { kind: "export", message: "no application" },
      },
    });
    expect(store.job("xlsx")).toEqual({
      state: "done",
      fileName: "p.xlsx",
      issued: null,
      openError: { kind: "export", message: "no application" },
    });
  });
});
