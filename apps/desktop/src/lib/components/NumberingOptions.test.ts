// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { Command, NumberingStrategy, Project } from "$lib/ipc/bindings";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { NumberingStore, type NumberingApi } from "$lib/stores/numbering.svelte";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import NumberingOptions from "./NumberingOptions.svelte";

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
    closeProject: () => Promise.resolve({ status: "ok", data: null }),
    openRecentProject: () => Promise.resolve({ status: "ok", data: true }),
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
  component = mount(NumberingOptions, {
    target,
    props: { numberingStore: stores.numberingStore },
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

function choose(value: string, option: string): void {
  const select = [...target.querySelectorAll("select")].find((s) =>
    [...s.options].some((o) => o.value === option),
  );
  if (select) {
    select.value = value;
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }
}

describe("numbering options (D-22, D-23, T3.18)", () => {
  it("explains a locked project and still offers the insert policy (D-23)", async () => {
    const stores = setup(true);
    render(stores);
    await settle();
    expect(target.textContent).toContain("Numbering is locked");
    choose("letter_suffix", "letter_suffix");
    await settle();
    expect(stores.commands).toEqual([
      {
        type: "set_numbering_settings",
        settings: {
          strategy: "sheet_zone",
          multi_instance: "quantity",
          insert_when_locked: "letter_suffix",
        },
      },
    ]);
    expect(stores.previews).toEqual([]);
  });

  it("sets multi-instance numbering (D-22)", async () => {
    const stores = setup();
    render(stores);
    choose("sub_number", "sub_number");
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
