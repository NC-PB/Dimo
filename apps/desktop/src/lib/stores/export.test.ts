import { describe, expect, it } from "vitest";
import type { ExportRequest, JobFinished, JobProgress } from "$lib/ipc/bindings";
import { ExportStore, type ExportApi } from "./export.svelte";

/** Rust's side of the export: records requests and lets the test send job events. */
function fakeApi(answer: number | null = 7) {
  const requests: ExportRequest[] = [];
  let progress: ((p: JobProgress) => void) | null = null;
  let finished: ((f: JobFinished) => void) | null = null;
  let beforeAnswer: (() => void) | null = null;
  const api: ExportApi = {
    exportProject: (request) => {
      requests.push(request);
      beforeAnswer?.();
      return Promise.resolve({ status: "ok", data: answer });
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
      { format: "csv", language: "en", pdf_balloons: "page_content", issued: true },
    ]);
    expect(store.job("csv")).toEqual({ state: "running", fraction: 0 });
    expect(store.busy("csv")).toBe(true);
    rust.progress({ id: 7, fraction: 0.8, message: "writing" });
    expect(store.job("csv")).toEqual({ state: "running", fraction: 0.8 });
    rust.finished({ id: 7, outcome: { state: "done", file_name: "part.csv" } });
    expect(store.job("csv")).toEqual({ state: "done", fileName: "part.csv" });
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
