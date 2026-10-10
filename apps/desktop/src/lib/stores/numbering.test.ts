import { describe, expect, it } from "vitest";
import type {
  Command,
  CommandError,
  NumberingPreview,
  NumberingStrategy,
  Patch,
  Project,
} from "$lib/ipc/bindings";
import { characteristic, emptyProject } from "./fixtures";
import { NumberingStore, type NumberingApi, type NumberingProject } from "./numbering.svelte";

type Answer = { status: "ok"; data: NumberingPreview } | { status: "error"; error: CommandError };

/** Rust's side: records preview requests and commands, answers when the test says so. */
function fakeRust(project: Project | null = projectOf(3)) {
  const requests: NumberingStrategy[] = [];
  const commands: Command[] = [];
  const pending: ((answer: Answer) => void)[] = [];
  const api: NumberingApi = {
    previewNumbering: (strategy) => {
      requests.push(strategy);
      return new Promise((resolve) => pending.push(resolve));
    },
  };
  const state: NumberingProject & { project: Project | null } = {
    project,
    revision: 0,
    execute: (command: Command): Promise<Patch | undefined> => {
      commands.push(command);
      return Promise.resolve({ changes: [] });
    },
  };
  return {
    api,
    state,
    requests,
    commands,
    /** Answers the oldest open request. */
    answer: (answer: Answer) => pending.shift()?.(answer),
  };
}

function projectOf(count: number): Project {
  const project = emptyProject();
  project.characteristics = Array.from({ length: count }, (_, i) =>
    characteristic(`c${String(i + 1)}`, i + 1),
  );
  return project;
}

/** Reversed numbers for `c1..c3`. */
function reversed(strategy: NumberingStrategy): NumberingPreview {
  return {
    strategy,
    entries: [
      { id: "c3", number: "1" },
      { id: "c2", number: "2" },
      { id: "c1", number: "3" },
    ],
    changed: 2,
  };
}

describe("numbering store (T2.7, FR-BAL-04, FR-BAL-05)", () => {
  it("shows the project's strategy until another is picked", () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    expect(store.strategy).toBe("sheet_zone");
    store.pick("view_clockwise");
    expect(store.strategy).toBe("view_clockwise");
  });

  it("asks for a preview only while previewing and maps the ghost numbers", async () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    await store.refresh();
    expect(rust.requests).toEqual([]);
    expect(store.ghosts).toEqual({});

    store.setPreviewing(true);
    store.pick("kind");
    const done = store.refresh();
    rust.answer({ status: "ok", data: reversed("kind") });
    await done;
    expect(rust.requests).toEqual(["kind"]);
    expect(store.ghosts).toEqual({ c1: "3", c2: "2", c3: "1" });

    store.setPreviewing(false);
    expect(store.ghosts).toEqual({});
  });

  it("drops answers to older requests", async () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    store.setPreviewing(true);
    const first = store.refresh();
    store.pick("manual");
    const second = store.refresh();
    rust.answer({ status: "ok", data: reversed("sheet_zone") });
    rust.answer({
      status: "ok",
      data: {
        strategy: "manual",
        entries: [
          { id: "c1", number: "1" },
          { id: "c2", number: "2" },
          { id: "c3", number: "3" },
        ],
        changed: 0,
      },
    });
    await Promise.all([first, second]);
    expect(store.preview?.strategy).toBe("manual");
    expect(store.ghosts).toEqual({ c1: "1", c2: "2", c3: "3" });
  });

  it("does not preview a locked project (D-23)", async () => {
    const rust = fakeRust();
    const project = rust.state.project;
    if (project) {
      project.numbering = {
        lock: {
          reason: "issued_report",
          locked_at: "2026-10-09T10:00:00Z",
          locked_by: "peter",
          highest_number: 3,
          given: [],
        },
      };
    }
    const store = new NumberingStore(rust.api, rust.state);
    store.setPreviewing(true);
    await store.refresh();
    expect(store.locked).toBe(true);
    expect(rust.requests).toEqual([]);
    expect(store.preview).toBeNull();
  });

  it("shows a refused preview as an error", async () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    store.setPreviewing(true);
    const done = store.refresh();
    rust.answer({
      status: "error",
      error: { kind: "rejected", reason: "numbering_locked", message: "numbering is locked" },
    });
    await done;
    expect(store.error?.kind).toBe("rejected");
    expect(store.preview).toBeNull();
  });

  it("applies the shown strategy as one command and ends the preview", async () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    store.pick("view");
    store.setPreviewing(true);
    expect(await store.apply()).toBe(true);
    expect(rust.commands).toEqual([{ type: "apply_numbering", strategy: "view" }]);
    expect(store.previewing).toBe(false);
    expect(store.picked).toBeNull();
  });

  it("changes multi-instance numbering and the insert policy only when they differ", async () => {
    const rust = fakeRust();
    const store = new NumberingStore(rust.api, rust.state);
    await store.setSettings({ multi_instance: "quantity" });
    expect(rust.commands).toEqual([]);
    await store.setSettings({ multi_instance: "sub_number" });
    await store.setSettings({ insert_when_locked: "letter_suffix" });
    expect(rust.commands).toEqual([
      {
        type: "set_numbering_settings",
        settings: {
          strategy: "sheet_zone",
          multi_instance: "sub_number",
          insert_when_locked: "next_free",
        },
      },
      {
        type: "set_numbering_settings",
        settings: {
          strategy: "sheet_zone",
          multi_instance: "quantity",
          insert_when_locked: "letter_suffix",
        },
      },
    ]);
  });
});
