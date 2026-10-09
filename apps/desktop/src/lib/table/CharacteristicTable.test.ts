// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Command, CommandError, ProjectPatched } from "$lib/ipc/bindings";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";
import CharacteristicTable from "./CharacteristicTable.svelte";

/** Height of the scroll area the tests pretend to have, in CSS px. */
const VIEW_HEIGHT = 560;
const ROW_HEIGHT = 28;

type Reply = { status: "ok"; data: ProjectPatched } | { status: "error"; error: CommandError };

/** Records the commands the table sends; answers with an empty patch unless told otherwise. */
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

/** A project with `count` characteristics `c1`..`cN`, numbered in placement order. */
function projectWith(count: number, locked = false) {
  const project = emptyProject();
  project.characteristics = Array.from({ length: count }, (_, i) => ({
    ...characteristic(`c${String(i + 1)}`, i + 1),
    requirement_text: `R${String(i + 1)}`,
    nominal: "10.0",
  }));
  if (locked) {
    project.numbering = {
      lock: {
        reason: "manual",
        locked_at: "2026-10-09T10:00:00Z",
        locked_by: "peter",
        insert_policy: "next_free",
        highest_number: count,
      },
    };
  }
  return project;
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | null = null;

beforeEach(() => {
  // happy-dom does no layout: give the scroll area a size so the virtualizer has a window.
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.getAttribute("role") === "grid" ? VIEW_HEIGHT : 0;
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.getAttribute("role") === "grid" ? VIEW_HEIGHT : 0;
  });
  vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    // Header plus one row per characteristic, as laid out by the browser.
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

async function render(count: number, locked = false) {
  const rust = fakeRust();
  const project = new ProjectStore(rust.api);
  project.load(loaded(1, projectWith(count, locked)));
  const selected = new SelectionStore();
  component = mount(CharacteristicTable, { target, props: { project, selected } });
  flushSync();
  await tick();
  const grid = target.querySelector<HTMLElement>('[role="grid"]');
  if (!grid) {
    throw new Error("no grid");
  }
  return { rust, project, selected, grid };
}

function rows(): HTMLElement[] {
  return [...target.querySelectorAll<HTMLElement>('[role="row"][data-row]')];
}

function rowNumbers(): number[] {
  return rows().map((r) => Number(r.getAttribute("aria-rowindex")) - 1);
}

function key(element: HTMLElement, init: KeyboardEventInit) {
  element.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ...init }));
  flushSync();
}

describe("characteristic table (T1.7)", () => {
  it("renders only the visible rows of 1000 characteristics", async () => {
    const { grid } = await render(1000);
    expect(grid.getAttribute("aria-rowcount")).toBe("1001");
    const shown = rows().length;
    expect(shown).toBeGreaterThan(VIEW_HEIGHT / ROW_HEIGHT - 2);
    expect(shown).toBeLessThan(VIEW_HEIGHT / ROW_HEIGHT + 2 * 8 + 2);
    expect(rowNumbers()[0]).toBe(1);

    // Scrolled to the middle: rows around 500 are rendered, still only a window of them.
    grid.scrollTop = 500 * ROW_HEIGHT;
    grid.dispatchEvent(new Event("scroll"));
    flushSync();
    const numbers = rowNumbers();
    expect(numbers.length).toBeLessThan(VIEW_HEIGHT / ROW_HEIGHT + 2 * 8 + 2);
    expect(Math.min(...numbers)).toBeGreaterThan(480);
    expect(Math.max(...numbers)).toBeLessThan(540);
  });

  it("sends a typed decimal as text in update_fields", async () => {
    const { rust, grid, selected } = await render(5);
    key(grid, { key: "ArrowDown" }); // first row
    key(grid, { key: "ArrowDown" }); // second row
    expect([...selected.ids]).toEqual(["c2"]);
    for (let i = 0; i < 3; i++) {
      key(grid, { key: "ArrowRight" });
    }
    key(grid, { key: "Enter" });
    const input = target.querySelector<HTMLInputElement>("input.editor");
    if (!input) {
      throw new Error("no editor");
    }
    expect(input.value).toBe("10.0");
    input.value = "12.50";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await vi.waitFor(() => {
      expect(rust.commands).toEqual([
        { type: "update_fields", ids: ["c2"], values: [{ field: "nominal", value: "12.50" }] },
      ]);
    });
    await vi.waitFor(() => {
      expect(target.querySelector("input.editor")).toBeNull();
    });
  });

  it("keeps the editor open and shows Rust's reason when a value is refused", async () => {
    const { rust, grid, project } = await render(3);
    rust.answer(() => ({
      status: "error",
      error:
        'invalid args `command` for command `execute`: invalid decimal string "1,5", expected digits' as unknown as CommandError,
    }));
    key(grid, { key: "ArrowDown" });
    for (let i = 0; i < 4; i++) {
      key(grid, { key: "ArrowRight" });
    }
    key(grid, { key: "1" }); // typing starts the edit with that character
    const input = target.querySelector<HTMLInputElement>("input.editor");
    if (!input) {
      throw new Error("no editor");
    }
    expect(input.value).toBe("1");
    input.value = "1,5";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await vi.waitFor(() => {
      expect(target.querySelector('[role="alert"]')?.textContent).toContain(
        'invalid decimal string "1,5"',
      );
    });
    expect(rust.commands[0]).toEqual({
      type: "update_fields",
      ids: ["c1"],
      values: [{ field: "upper_dev", value: "1,5" }],
    });
    expect(target.querySelector("input.editor")?.getAttribute("aria-invalid")).toBe("true");
    // The table shows the reason; the window wide error line does not repeat it.
    expect(project.error).toBeNull();
  });

  it("moves the selected rows with Alt+Arrow as one move_characteristics", async () => {
    const { rust, grid, selected } = await render(5);
    key(grid, { key: "ArrowDown" });
    key(grid, { key: "ArrowDown" });
    key(grid, { key: "ArrowDown", shiftKey: true });
    expect([...selected.ids]).toEqual(["c2", "c3"]);
    key(grid, { key: "ArrowDown", altKey: true });
    await vi.waitFor(() => {
      expect(rust.commands).toEqual([
        { type: "move_characteristics", ids: ["c2", "c3"], before: "c5" },
      ]);
    });
  });

  it("drops dragged rows into the gap under the pointer", async () => {
    const { rust, grid } = await render(5);
    const handle = target.querySelector<HTMLElement>('[data-row="3"] [data-handle]');
    const body = target.querySelectorAll<HTMLElement>('[role="rowgroup"]')[1];
    if (!handle || !body) {
      throw new Error("no handle");
    }
    vi.spyOn(body, "getBoundingClientRect").mockReturnValue(new DOMRect(0, 100, 1200, 140));
    grid.setPointerCapture = () => undefined;
    handle.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, button: 0, pointerId: 1, clientY: 205 }),
    );
    flushSync();
    // Release at the gap between row 1 and row 2 (y = 100 + 1 * 28).
    grid.dispatchEvent(
      new PointerEvent("pointerup", { bubbles: true, button: 0, pointerId: 1, clientY: 129 }),
    );
    await vi.waitFor(() => {
      expect(rust.commands).toEqual([{ type: "move_characteristics", ids: ["c4"], before: "c2" }]);
    });
  });

  it("explains why rows cannot move while numbering is locked", async () => {
    const { rust, grid } = await render(3, true);
    key(grid, { key: "ArrowDown" });
    key(grid, { key: "ArrowDown", altKey: true });
    await tick();
    expect(rust.commands).toEqual([]);
    expect(target.querySelector('[role="alert"]')?.textContent).toContain("locked by peter");
  });

  it("follows the shared selection and opens the requirement for a focus request", async () => {
    const { grid, selected } = await render(1000);
    selected.select(["c700"]);
    flushSync();
    expect(grid.scrollTop).toBeGreaterThan(600 * ROW_HEIGHT);
    grid.dispatchEvent(new Event("scroll"));
    flushSync();
    const row = target.querySelector('[role="row"][aria-rowindex="701"]');
    expect(row?.getAttribute("aria-selected")).toBe("true");

    selected.focus("c3");
    flushSync();
    grid.scrollTop = 0;
    grid.dispatchEvent(new Event("scroll"));
    flushSync();
    expect([...selected.ids]).toEqual(["c3"]);
    const input = target.querySelector<HTMLInputElement>("input.editor");
    expect(input?.value).toBe("R3");
    expect(input?.getAttribute("aria-label")).toBe("Requirement");
  });
});
