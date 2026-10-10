// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { Command, NumberingStrategy, Project } from "$lib/ipc/bindings";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { NumberingStore, type NumberingApi } from "$lib/stores/numbering.svelte";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import NumberingPanel from "./NumberingPanel.svelte";

function projectOf(locked: boolean): Project {
  const project = emptyProject();
  project.characteristics = [characteristic("c1", 1), characteristic("c2", 2)];
  if (locked) {
    project.numbering = {
      lock: {
        reason: "manual",
        locked_at: "2026-10-09T10:00:00Z",
        locked_by: "peter",
        highest_number: 2,
        given: [],
      },
    };
  }
  return project;
}

/** Stores over a fake Rust that records commands and preview requests. */
function setup(locked = false) {
  const commands: Command[] = [];
  const previews: NumberingStrategy[] = [];
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1, projectOf(locked)) }),
    newProject: () => Promise.resolve({ status: "ok", data: true }),
    openProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProjectAs: () => Promise.resolve({ status: "ok", data: true }),
    confirmClose: () => Promise.resolve({ status: "ok", data: null }),
    execute: (command) => {
      commands.push(command);
      return Promise.resolve({
        status: "ok",
        data: { session: 1, revision: 0, patch: { changes: [] }, status: status() },
      });
    },
    undo: () => Promise.reject(new Error("not used")),
    redo: () => Promise.reject(new Error("not used")),
  };
  const preview: NumberingApi = {
    previewNumbering: (strategy) => {
      previews.push(strategy);
      return Promise.resolve({
        status: "ok",
        data: {
          strategy,
          entries: [
            { id: "c2", number: "1" },
            { id: "c1", number: "2" },
          ],
          changed: 2,
        },
      });
    },
  };
  const projectStore = new ProjectStore(api);
  projectStore.load(loaded(1, projectOf(locked)));
  const numberingStore = new NumberingStore(preview, projectStore);
  return { projectStore, numberingStore, commands, previews };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

function render(stores: ReturnType<typeof setup>) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(NumberingPanel, {
    target,
    props: { numberingStore: stores.numberingStore, projectStore: stores.projectStore },
  });
  flushSync();
}

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

function button(text: string): HTMLButtonElement | undefined {
  return [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
}

describe("numbering panel (T2.7, FR-BAL-04, FR-BAL-05)", () => {
  it("previews the picked strategy and applies it as one command", async () => {
    const stores = setup();
    render(stores);
    const strategy = target.querySelector("select");
    expect(strategy?.value).toBe("sheet_zone");
    expect([...(strategy?.options ?? [])].map((o) => o.value)).toEqual([
      "sheet_zone",
      "view",
      "view_clockwise",
      "kind",
      "manual",
    ]);
    if (strategy) {
      strategy.value = "view_clockwise";
      strategy.dispatchEvent(new Event("change", { bubbles: true }));
    }
    const preview = target.querySelector<HTMLInputElement>("input[type=checkbox]");
    preview?.click();
    await settle();
    expect(stores.previews.at(-1)).toBe("view_clockwise");
    expect(stores.numberingStore.ghosts).toEqual({ c1: "2", c2: "1" });
    expect(target.textContent).toContain("Numbers that change: 2");

    button("Apply numbering")?.click();
    await settle();
    expect(stores.commands).toEqual([{ type: "apply_numbering", strategy: "view_clockwise" }]);
    expect(stores.numberingStore.previewing).toBe(false);
  });

  it("explains a locked project and offers no apply (D-23)", async () => {
    const stores = setup(true);
    render(stores);
    await settle();
    expect(target.textContent).toContain("Numbering is locked");
    expect(button("Apply numbering")).toBeUndefined();
    expect(stores.previews).toEqual([]);
  });

  it("sets multi-instance numbering (D-22)", async () => {
    const stores = setup();
    render(stores);
    const selects = [...target.querySelectorAll("select")];
    const multi = selects.find((s) => [...s.options].some((o) => o.value === "sub_number"));
    if (multi) {
      multi.value = "sub_number";
      multi.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await settle();
    expect(stores.commands).toEqual([
      {
        type: "set_numbering_settings",
        settings: {
          strategy: "sheet_zone",
          multi_instance: "sub_number",
          insert_when_locked: "next_free",
        },
      },
    ]);
  });
});
