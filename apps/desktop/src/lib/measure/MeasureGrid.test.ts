// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  Command,
  CommandError,
  MeasurementResult,
  MeasurementState,
  ProjectPatched,
  ResultType,
  RowState,
  Sample,
  Verdict,
} from "$lib/ipc/bindings";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { MeasurementStore } from "$lib/stores/measurement.svelte";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";
import MeasureGrid from "./MeasureGrid.svelte";

const VIEW_HEIGHT = 560;
const ROW_HEIGHT = 28;

type Reply = { status: "ok"; data: ProjectPatched } | { status: "error"; error: CommandError };

/** Records the commands the grid sends; answers with an empty patch unless told otherwise. */
function fakeRust() {
  const commands: Command[] = [];
  let reply: (command: Command) => Reply = () => ({
    status: "ok",
    data: { session: 1, revision: 0, patch: { changes: [] }, status: status() },
  });
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1) }),
    newProject: () => Promise.resolve({ status: "ok", data: true }),
    openProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProjectAs: () => Promise.resolve({ status: "ok", data: true }),
    confirmClose: () => Promise.resolve({ status: "ok", data: null }),
    execute: (command) => {
      commands.push(command);
      return Promise.resolve(reply(command));
    },
    undo: () => Promise.reject(new Error("not used")),
    redo: () => Promise.reject(new Error("not used")),
  };
  return {
    api,
    commands,
    answer(next: (command: Command) => Reply) {
      reply = next;
    },
  };
}

function sample(n: number): Sample {
  return {
    id: `s${String(n)}`,
    serial: String(n),
    date: "",
    inspector: "",
    gauges: "",
    comment: "",
  };
}

/**
 * Five characteristics: c2 and c4 are go/no go (attribute), the others have limits; three
 * samples. c1 of sample 1 holds 10.05 (pass), c3 of sample 1 holds 11 (fail), c2 of sample 2 is
 * OK, c5 of sample 1 has no limits in Rust's view (not evaluated).
 */
function setup() {
  const project = emptyProject();
  project.characteristics = [1, 2, 3, 4, 5].map((n) => ({
    ...characteristic(`c${String(n)}`, n),
    requirement_text: `R${String(n)}`,
  }));
  project.samples = [1, 2, 3].map(sample);
  const result = (s: string, c: string, value: MeasurementResult["value"]): MeasurementResult => ({
    sample: s,
    characteristic: c,
    value,
    source: { type: "manual" },
    comment: "",
  });
  project.results = [
    result("s1", "c1", { type: "variable", value: "10.05" }),
    result("s1", "c3", { type: "variable", value: "11" }),
    result("s2", "c2", { type: "attribute", value: "ok" }),
    result("s1", "c5", { type: "variable", value: "3.2" }),
  ];
  const types: ResultType[] = ["variable", "attribute", "variable", "attribute", "variable"];
  const cells: Verdict[][] = [
    ["pass", "missing", "missing"],
    ["missing", "pass", "missing"],
    ["fail", "missing", "missing"],
    ["missing", "missing", "missing"],
    ["not_evaluated", "missing", "missing"],
  ];
  const counts = { pass: 0, fail: 0, not_evaluated: 0, missing: 0 };
  const rows: RowState[] = project.characteristics.map((c, i) => ({
    characteristic: c.id,
    result_type: types[i] ?? "variable",
    draft: false,
    cells: cells[i] ?? [],
    counts,
    verdict: "open",
  }));
  const state: MeasurementState = {
    rows,
    samples: project.samples.map((s) => ({ sample: s.id, counts, verdict: "open" })),
    counts,
    verdict: "fail",
  };
  return { project, state };
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | null = null;

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.getAttribute("role") === "grid" ? VIEW_HEIGHT : 0;
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1400);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.getAttribute("role") === "grid" ? VIEW_HEIGHT : 0;
  });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1400);
  vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return Number(this.getAttribute("aria-rowcount") ?? 0) * ROW_HEIGHT;
  });
  target = document.createElement("div");
  document.body.appendChild(target);
});

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  vi.restoreAllMocks();
});

async function render() {
  const rust = fakeRust();
  const { project: data, state } = setup();
  const project = new ProjectStore(rust.api);
  project.load(loaded(1, data));
  const measurement = new MeasurementStore({
    measurementState: () => Promise.resolve({ status: "ok", data: state }),
    changedSinceIssuedReport: () => Promise.resolve({ status: "ok", data: ["c3"] }),
  });
  await measurement.refresh(true);
  const selected = new SelectionStore();
  component = mount(MeasureGrid, { target, props: { project, measurement, selected } });
  flushSync();
  await tick();
  const grid = target.querySelector<HTMLElement>('[role="grid"]');
  if (!grid) {
    throw new Error("no grid");
  }
  grid.focus();
  return { rust, project, selected, grid };
}

function key(element: HTMLElement, init: KeyboardEventInit) {
  element.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
  flushSync();
}

/** The active cell as "c<n>/s<n>" or "c<n>/type", from `aria-activedescendant`. */
function activeCell(grid: HTMLElement): string {
  const id = grid.getAttribute("aria-activedescendant") ?? "";
  const value = /^mg-(c\d+)-(\d+)-cell$/.exec(id);
  if (value) {
    return `${value[1] ?? ""}/s${String(Number(value[2]) + 1)}`;
  }
  const kind = /^mg-type-(c\d+)-cell$/.exec(id);
  return kind ? `${kind[1] ?? ""}/type` : id;
}

function editor(): HTMLInputElement {
  const input = target.querySelector<HTMLInputElement>("input.editor");
  if (!input) {
    throw new Error("no editor");
  }
  return input;
}

function type(text: string) {
  const input = editor();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

describe("measurement grid (T3.5)", () => {
  it("shows marks with shape per verdict and a bold border on fail", async () => {
    await render();
    const cell = (c: string, s: number) =>
      target.querySelector<HTMLElement>(`#mg-${c}-${String(s - 1)}-cell`);
    expect(cell("c1", 1)?.querySelector("[data-mark]")?.getAttribute("data-mark")).toBe("pass");
    expect(cell("c3", 1)?.querySelector("[data-mark]")?.getAttribute("data-mark")).toBe("fail");
    expect(cell("c3", 1)?.classList.contains("fail")).toBe(true);
    expect(cell("c5", 1)?.querySelector("[data-mark]")?.getAttribute("data-mark")).toBe(
      "not_evaluated",
    );
    expect(cell("c4", 1)?.querySelector("[data-mark]")).toBeNull();
    expect(cell("c2", 2)?.textContent).toContain("OK");
    // Decision 16: the hint marker from Rust's list.
    const hinted = [...target.querySelectorAll('[data-testid="changed-hint"]')].map((h) =>
      h.closest("[data-row]")?.getAttribute("data-row"),
    );
    expect(hinted).toEqual(["2"]);
  });

  it("moves with arrows, Enter, Shift+Enter, Tab and Shift+Tab and selects the row", async () => {
    const { grid, selected } = await render();
    expect(activeCell(grid)).toBe("c1/s1");
    key(grid, { key: "ArrowDown" });
    key(grid, { key: "ArrowRight" });
    expect(activeCell(grid)).toBe("c2/s2");
    expect([...selected.ids]).toEqual(["c2"]);
    key(grid, { key: "Enter" });
    expect(activeCell(grid)).toBe("c3/s2");
    key(grid, { key: "Enter", shiftKey: true });
    expect(activeCell(grid)).toBe("c2/s2");
    key(grid, { key: "Tab" });
    expect(activeCell(grid)).toBe("c2/s3");
    key(grid, { key: "Tab" }); // last sample: stays
    expect(activeCell(grid)).toBe("c2/s3");
    key(grid, { key: "Tab", shiftKey: true });
    expect(activeCell(grid)).toBe("c2/s2");
    key(grid, { key: "End", metaKey: true });
    key(grid, { key: "End", ctrlKey: true });
    expect(activeCell(grid)).toMatch(/^c5\//);
    key(grid, { key: "Home" });
    expect(activeCell(grid)).toBe("c5/s1");
    key(grid, { key: "ArrowLeft" }); // left of the first sample: the result type
    expect(activeCell(grid)).toBe("c5/type");
  });

  it("typing starts editing, Enter commits through Rust and moves down", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "1" });
    expect(editor().value).toBe("1");
    type("10,02");
    editor().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    flushSync();
    expect(activeCell(grid)).toBe("c2/s1"); // moves on at once, Rust answers later
    await vi.waitFor(() => {
      expect(rust.commands).toEqual([
        {
          type: "set_results",
          results: [{ sample: "s1", characteristic: "c1", text: "10,02", comment: null }],
        },
      ]);
    });
  });

  it("Escape cancels typing without a command", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "5" });
    editor().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flushSync();
    expect(target.querySelector("input.editor")).toBeNull();
    expect(activeCell(grid)).toBe("c1/s1");
    await tick();
    expect(rust.commands).toEqual([]);
  });

  it("opens a refused value again with Rust's reason in words", async () => {
    const { rust, grid } = await render();
    rust.answer(() => ({
      status: "error",
      error: { kind: "rejected", reason: "invalid_result", message: "invalid result" },
    }));
    key(grid, { key: "a" });
    editor().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await vi.waitFor(() => {
      expect(target.querySelector('[data-testid="grid-notice"]')?.textContent).toContain(
        "Characteristic 1, sample 1: Not a number",
      );
    });
    expect(editor().value).toBe("a");
    expect(editor().getAttribute("aria-invalid")).toBe("true");
    expect(activeCell(grid)).toBe("c1/s1");
  });

  it("attribute keys set OK or not OK and move down; Space toggles", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "ArrowDown" }); // c2, attribute
    key(grid, { key: "o" });
    expect(activeCell(grid)).toBe("c3/s1");
    key(grid, { key: "ArrowDown" }); // c4, attribute
    key(grid, { key: "x" });
    key(grid, { key: "ArrowUp" });
    key(grid, { key: "ArrowUp" });
    key(grid, { key: "ArrowUp" });
    key(grid, { key: "ArrowRight" }); // c2 of sample 2 is OK
    key(grid, { key: " " });
    await vi.waitFor(() => {
      expect(rust.commands.length).toBe(3);
    });
    const sent = rust.commands.map((c) =>
      c.type === "set_results"
        ? c.results.map((r) => `${r.characteristic}/${r.sample}=${r.text}`).join()
        : c.type,
    );
    expect(sent).toEqual(["c2/s1=ok", "c4/s1=not ok", "c2/s2=not ok"]);
    expect(target.querySelector("input.editor")).toBeNull();
  });

  it("attribute keys on a value cell are typed characters", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "n" });
    expect(editor().value).toBe("n");
    expect(rust.commands).toEqual([]);
  });

  it("Delete clears a cell with a result and does nothing on an empty one", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "Delete" });
    key(grid, { key: "ArrowRight" });
    key(grid, { key: "Delete" });
    await vi.waitFor(() => {
      expect(rust.commands).toEqual([
        { type: "clear_results", cells: [{ sample: "s1", characteristic: "c1" }] },
      ]);
    });
  });

  it("pastes a block from the active cell as one set_results", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "ArrowDown" }); // c2/s1
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", {
      value: { getData: () => "ok\tnot ok\n10.1\t10.2\n" },
    });
    grid.dispatchEvent(event);
    await vi.waitFor(() => {
      expect(rust.commands.length).toBe(1);
    });
    const command = rust.commands[0];
    expect(command?.type).toBe("set_results");
    if (command?.type === "set_results") {
      expect(command.results.map((r) => `${r.characteristic}/${r.sample}=${r.text}`)).toEqual([
        "c2/s1=ok",
        "c2/s2=not ok",
        "c3/s1=10.1",
        "c3/s2=10.2",
      ]);
    }
  });

  it("refuses a pasted block that does not fit", async () => {
    const { rust, grid } = await render();
    key(grid, { key: "ArrowRight" });
    key(grid, { key: "ArrowRight" }); // sample 3: no room for two columns
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", { value: { getData: () => "1\t2" } });
    grid.dispatchEvent(event);
    flushSync();
    expect(target.querySelector('[data-testid="grid-notice"]')?.textContent).toContain(
      "1 rows by 2 columns",
    );
    expect(rust.commands).toEqual([]);
  });

  it("changes the result type and shows Rust's refusal", async () => {
    const { rust } = await render();
    rust.answer(() => ({
      status: "error",
      error: { kind: "rejected", reason: "result_type_conflict", message: "conflict" },
    }));
    const select = target.querySelector<HTMLSelectElement>("#mg-type-c1");
    if (!select) {
      throw new Error("no select");
    }
    select.value = "attribute";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    await vi.waitFor(() => {
      expect(target.querySelector('[data-testid="grid-notice"]')?.textContent).toContain(
        "results of the other type",
      );
    });
    expect(rust.commands).toEqual([
      { type: "set_result_type", ids: ["c1"], result_type: "attribute" },
    ]);
    expect(select.value).toBe("auto");
  });
});
