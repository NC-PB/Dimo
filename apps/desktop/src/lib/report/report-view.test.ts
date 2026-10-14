import { afterEach, describe, expect, it, vi } from "vitest";
import type { CommandError, DocumentInfo, ReportChecks, ReportView } from "$lib/ipc/bindings";
import { previewTiles } from "./preview";
import { ReportViewStore, type ReportViewApi } from "./report-view.svelte";

const VIEW: ReportView = { kind: "inspection_report", language: "en", blank_columns: 3 };

const CHECKS: ReportChecks = {
  warnings: { draft_limits: false, open_results: true, limits_changed: false },
  samples: 2,
  rows: 5,
  missing_values: 2,
  not_evaluated: 0,
  draft_rows: 0,
  changed_rows: 0,
  missing_fields: [],
};

function doc(name: string): DocumentInfo {
  return { doc: name, name: `${name}.pdf`, sheets: [{ width: 842, height: 595 }] };
}

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

function fake() {
  const asked: ReportView[] = [];
  const answers: ((value: Result<DocumentInfo>) => void)[] = [];
  const api: ReportViewApi = {
    reportPreview: (view) => {
      asked.push(view);
      return new Promise((resolve) => answers.push(resolve));
    },
    reportChecks: () => Promise.resolve({ status: "ok", data: CHECKS }),
    printReport: () => Promise.resolve({ status: "ok", data: "part_inspection-report.pdf" }),
  };
  return { api, asked, answers };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("report view store (T3.19)", () => {
  it("renders once after a quick series of changes", async () => {
    vi.useFakeTimers();
    const rust = fake();
    const store = new ReportViewStore(rust.api, 300);
    store.schedule(VIEW);
    store.schedule({ ...VIEW, language: "de" });
    store.schedule({ ...VIEW, kind: "check_sheet" });
    expect(store.loading).toBe(true);
    await vi.advanceTimersByTimeAsync(299);
    expect(rust.asked).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(rust.asked).toEqual([{ ...VIEW, kind: "check_sheet" }]);
    rust.answers[0]?.({ status: "ok", data: doc("a") });
    await vi.waitFor(() => {
      expect(store.preview?.doc).toBe("a");
    });
    expect(store.checks).toEqual(CHECKS);
    expect(store.loading).toBe(false);
  });

  it("drops a late answer of an older preview", async () => {
    const rust = fake();
    const store = new ReportViewStore(rust.api, 0);
    const first = store.refresh(VIEW);
    const second = store.refresh({ ...VIEW, language: "de" });
    rust.answers[1]?.({ status: "ok", data: doc("new") });
    await second;
    rust.answers[0]?.({ status: "ok", data: doc("old") });
    await first;
    expect(store.preview?.doc).toBe("new");
  });

  it("keeps the last preview and names the error when rendering fails", async () => {
    const rust = fake();
    const store = new ReportViewStore(rust.api, 0);
    const ok = store.refresh(VIEW);
    rust.answers[0]?.({ status: "ok", data: doc("a") });
    await ok;
    const failed = store.refresh(VIEW);
    rust.answers[1]?.({
      status: "error",
      error: { kind: "export", reason: "report_failed", message: "typst" },
    });
    await failed;
    expect(store.preview?.doc).toBe("a");
    expect(store.previewError).toEqual({
      kind: "export",
      reason: "report_failed",
      message: "typst",
    });
    store.clear();
    expect([store.preview, store.checks, store.previewError]).toEqual([null, null, null]);
  });

  it("prints through Rust and reports the file or the error", async () => {
    const rust = fake();
    const store = new ReportViewStore(rust.api, 0);
    await store.printReport(VIEW);
    expect(store.print).toEqual({ state: "done", fileName: "part_inspection-report.pdf" });
    rust.api.printReport = () =>
      Promise.resolve({
        status: "error",
        error: { kind: "export", reason: "open_failed", message: "no viewer" },
      });
    await store.printReport(VIEW);
    expect(store.print).toEqual({
      state: "failed",
      error: { kind: "export", reason: "open_failed", message: "no viewer" },
    });
  });
});

describe("preview tiles (T3.19)", () => {
  it("covers an A4 landscape page with tiles of one level, placed in CSS px", () => {
    const page = { width: 841.89, height: 595.276 };
    const tiles = previewTiles("abc", 1, page, 842, 2, "dimo://localhost/");
    // 842 CSS px for 841.89 units at 2 device px per CSS px: level 1 (2 px per unit).
    expect(tiles.length).toBe(4 * 3);
    expect(tiles[0]?.url).toBe("dimo://localhost/tile/abc/1/1/0/0");
    expect(tiles.at(-1)?.url).toBe("dimo://localhost/tile/abc/1/1/3/2");
    const right = Math.max(...tiles.map((t) => t.left + t.width));
    const bottom = Math.max(...tiles.map((t) => t.top + t.height));
    expect(right).toBeCloseTo((Math.ceil(841.89 * 2) / 2) * (842 / 841.89), 6);
    expect(bottom).toBeCloseTo((Math.ceil(595.276 * 2) / 2) * (842 / 841.89), 6);
    expect(previewTiles("abc", 0, page, 0, 1)).toEqual([]);
  });
});
