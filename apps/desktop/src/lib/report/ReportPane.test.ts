// @vitest-environment happy-dom
/**
 * "Before you issue the report" (T3.19): the checks Rust counted, each with a way to fix it,
 * the warnings before an issued export, and Save copy and Issue report.
 */
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReportChecks } from "$lib/ipc/bindings";
import { measureUi } from "$lib/measure/measure-ui.svelte";
import { ExportStore, type ExportApi } from "$lib/stores/export.svelte";
import { emptyProject, loaded } from "$lib/stores/fixtures";
import { projectStore } from "$lib/stores/project.svelte";
import { view } from "$lib/stores/view.svelte";
import ReportPane from "./ReportPane.svelte";
import { ReportViewStore } from "./report-view.svelte";

const CHECKS: ReportChecks = {
  warnings: { draft_limits: true, open_results: true, limits_changed: false },
  samples: 3,
  rows: 6,
  missing_values: 2,
  not_evaluated: 0,
  draft_rows: 9,
  changed_rows: 0,
  missing_fields: ["report_number", "inspected_by"],
};

let target: HTMLElement | null = null;
let component: ReturnType<typeof mount> | null = null;
let session = 9000;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target?.remove();
  projectStore.load(loaded(++session, null));
  measureUi.onlyMissing = false;
  view.setTab("home");
});

function exportApi(started: string[]): ExportApi {
  return {
    exportProject: (request) => {
      started.push(`${request.format}${request.issued ? " issued" : ""}`);
      return Promise.resolve({ status: "ok", data: 1 });
    },
    cancelJob: () => Promise.resolve(true),
    reportExportWarnings: () => Promise.resolve({ status: "ok", data: CHECKS.warnings }),
    onProgress: () => Promise.resolve(() => undefined),
    onFinished: () => Promise.resolve(() => undefined),
  };
}

function render(checks: ReportChecks, kind: "check_sheet" | "inspection_report") {
  projectStore.load(loaded(++session, emptyProject()));
  const store = new ReportViewStore(
    {
      reportPreview: () => Promise.reject(new Error("not used")),
      reportChecks: () => Promise.reject(new Error("not used")),
      printReport: () => Promise.reject(new Error("not used")),
    },
    0,
  );
  store.checks = checks;
  const started: string[] = [];
  const exports = new ExportStore(exportApi(started));
  exports.document = kind;
  target = document.createElement("div");
  document.body.append(target);
  component = mount(ReportPane, { target, props: { store, exports } });
  flushSync();
  const root = target;
  const items = () =>
    [...root.querySelectorAll<HTMLElement>("[data-check]")].map((li) => ({
      id: li.dataset.check,
      ok: li.dataset.ok === "true",
      text: li.textContent.replace(/\s+/g, " ").trim(),
    }));
  return { root, items, exports, started };
}

describe("Report pane (T3.19)", () => {
  it("lists Rust's checks with counts and the fields still empty", () => {
    const { items } = render(CHECKS, "inspection_report");
    expect(items()).toEqual([
      { id: "values", ok: false, text: "Missing values: 2. Go to Measure" },
      { id: "draft", ok: false, text: "Rows with limits from a draft tolerance table: 9." },
      {
        id: "fields",
        ok: false,
        text: "Still empty in the report header: Report number, Inspected by. Open File > Info",
      },
    ]);
  });

  it("says what is fine, and a check sheet needs no values", () => {
    const done: ReportChecks = {
      ...CHECKS,
      missing_values: 0,
      draft_rows: 0,
      missing_fields: [],
    };
    const report = render(done, "inspection_report");
    expect(report.items().map((i) => [i.id, i.ok])).toEqual([
      ["values", true],
      ["fields", true],
    ]);
    if (component) {
      void unmount(component);
    }
    target?.remove();
    const sheet = render({ ...CHECKS, changed_rows: 1, draft_rows: 0 }, "check_sheet");
    expect(sheet.items().map((i) => [i.id, i.ok])).toEqual([
      ["blank", true],
      ["changed", false],
      ["fields", false],
    ]);
  });

  it("goes to Measure with only missing values shown", () => {
    const { root } = render(CHECKS, "inspection_report");
    const go = [...root.querySelectorAll("button")].find(
      (b) => b.textContent.trim() === "Go to Measure",
    );
    go?.click();
    flushSync();
    expect(view.current).toBe("measure");
    expect(measureUi.onlyMissing).toBe(true);
  });

  it("confirms Rust's warnings before an issued export", async () => {
    const { root, exports, started } = render(CHECKS, "inspection_report");
    await exports.startReport("inspection_report", true);
    flushSync();
    const dialog = root.querySelector("[role=alertdialog]");
    expect(dialog?.textContent).toContain("Some limits come from tolerance tables");
    expect(started).toEqual([]);
    const confirm = [...root.querySelectorAll("button")].find(
      (b) => b.textContent.trim() === "Export anyway",
    );
    confirm?.click();
    await Promise.resolve();
    expect(started).toEqual(["inspection_report_pdf issued"]);
    // While the job runs the pane offers Cancel instead of the two buttons.
    await vi.waitFor(() => {
      flushSync();
      const labels = [...root.querySelectorAll("button")].map((b) => b.textContent.trim());
      expect(labels).toContain("Cancel export");
      expect(labels).not.toContain("Save copy (not issued)");
    });
  });

  it("saves a copy without asking", async () => {
    const { exports, started } = render(CHECKS, "check_sheet");
    await exports.startReport("check_sheet", false);
    expect(started).toEqual(["check_sheet_pdf"]);
  });
});
