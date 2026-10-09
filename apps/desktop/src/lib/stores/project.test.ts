import { describe, expect, it, vi } from "vitest";
import type {
  Command,
  CommandError,
  DocumentInfo,
  Patch,
  ProjectLoaded,
  ProjectPatched,
  ProjectStatusChanged,
} from "$lib/ipc/bindings";
import { SHEET_ID, addChanges, emptyProject, loaded, status, undoAddChanges } from "./fixtures";
import {
  ProjectStore,
  type ProjectCommands,
  type ProjectEvents,
  type UnsavedChoice,
} from "./project.svelte";

type Handler<T> = (event: { payload: T }) => void;

/**
 * Stand-in for the Rust side: commands resolve like the generated functions, and events reach
 * the store through the same listeners the app registers. Each `execute`, `undo` and `redo`
 * sends the next patch from `script`, as Rust would.
 */
class FakeRust implements ProjectCommands {
  state: ProjectLoaded = loaded(1);
  revision = 0;
  script: { changes: Patch["changes"]; canUndo: boolean; canRedo: boolean }[] = [];
  calls: string[] = [];
  handlers = {
    loaded: [] as Handler<ProjectLoaded>[],
    patched: [] as Handler<ProjectPatched>[],
    status: [] as Handler<ProjectStatusChanged>[],
    close: [] as Handler<null>[],
  };

  readonly events: ProjectEvents = {
    projectLoaded: this.listener(this.handlers.loaded),
    projectPatched: this.listener(this.handlers.patched),
    projectStatusChanged: this.listener(this.handlers.status),
    closeRequested: this.listener(this.handlers.close),
  };

  private listener<T>(list: Handler<T>[]) {
    return {
      listen: (handler: Handler<T>) => {
        list.push(handler);
        return Promise.resolve(() => {
          list.splice(list.indexOf(handler), 1);
        });
      },
    };
  }

  emitPatched(payload: ProjectPatched): void {
    for (const h of this.handlers.patched) {
      h({ payload });
    }
  }

  private next(name: string) {
    this.calls.push(name);
    const step = this.script.shift();
    if (!step) {
      return Promise.resolve({
        status: "error" as const,
        error: { kind: "rejected", message: "nothing scripted" } as CommandError,
      });
    }
    this.revision += 1;
    const patched: ProjectPatched = {
      session: this.state.session,
      revision: this.revision,
      patch: { changes: step.changes },
      status: status({ modified: true, can_undo: step.canUndo, can_redo: step.canRedo }),
    };
    if (this.eventsDelayed) {
      this.delayed.push(patched);
    } else {
      this.emitPatched(patched);
    }
    return Promise.resolve({ status: "ok" as const, data: patched });
  }

  /** When set, events are held back until `flushEvents`, as if they were still on their way. */
  eventsDelayed = false;
  delayed: ProjectPatched[] = [];

  flushEvents(): void {
    for (const patched of this.delayed.splice(0)) {
      this.emitPatched(patched);
    }
  }

  projectState() {
    return Promise.resolve({ status: "ok" as const, data: this.state });
  }
  newProject(discard: boolean) {
    this.calls.push(`new(${String(discard)})`);
    return Promise.resolve({ status: "ok" as const, data: true });
  }
  openProject(discard: boolean) {
    this.calls.push(`open(${String(discard)})`);
    return Promise.resolve({ status: "ok" as const, data: true });
  }
  saveProject() {
    this.calls.push("save");
    return Promise.resolve({ status: "ok" as const, data: true });
  }
  saveProjectAs() {
    this.calls.push("save_as");
    return Promise.resolve({ status: "ok" as const, data: false });
  }
  confirmClose(discard: boolean) {
    this.calls.push(`close(${String(discard)})`);
    return Promise.resolve({ status: "ok" as const, data: null });
  }
  execute(command: Command) {
    return this.next(command.type === "add_characteristic" ? "execute" : command.type);
  }
  undo() {
    return this.next("undo");
  }
  redo() {
    return this.next("redo");
  }
}

const ADD: Command = {
  type: "add_characteristic",
  sheet: SHEET_ID,
  position: { x: 100, y: 100 },
  anchor: { x: 120, y: 110 },
  region: null,
  values: [],
};

async function connected(answer: UnsavedChoice = "cancel") {
  const rust = new FakeRust();
  const shown: (DocumentInfo | null)[] = [];
  const ask = vi.fn(() => Promise.resolve(answer));
  const store = new ProjectStore(rust, ask, (d) => shown.push(d));
  await store.connect(rust.events);
  return { rust, store, shown, ask };
}

describe("project store (T1.5)", () => {
  it("fetches the state after it listens, and shows the drawing", async () => {
    const { rust, store, shown } = await connected();
    expect(Object.values(rust.handlers).every((list) => list.length === 1)).toBe(true);
    expect(store.session).toBe(1);
    expect(store.isOpen).toBe(true);
    expect(store.sheets.map((s) => s.id)).toEqual([SHEET_ID]);
    expect(shown.map((d) => d?.doc)).toEqual(["a".repeat(64)]);
  });

  it("applies command, undo and redo as patches from events", async () => {
    const { rust, store } = await connected();
    rust.script = [
      { changes: addChanges("c1", 1, 0), canUndo: true, canRedo: false },
      { changes: undoAddChanges("c1", 1, 0), canUndo: false, canRedo: true },
      { changes: addChanges("c1", 1, 0), canUndo: true, canRedo: false },
    ];

    const patch = await store.execute(ADD);
    expect(patch?.changes).toHaveLength(2);
    expect(store.revision).toBe(1);
    expect(store.characteristics.map((c) => c.id)).toEqual(["c1"]);
    expect(store.balloonsOnSheet(SHEET_ID).map((b) => b.id)).toEqual(["b-c1"]);
    expect(store.characteristicById.get("c1")?.number).toBe(1);
    expect(store.canUndo).toBe(true);

    await store.undo();
    expect(store.revision).toBe(2);
    expect(store.characteristics).toEqual([]);
    expect(store.balloonsOnSheet(SHEET_ID)).toEqual([]);
    expect(store.canUndo).toBe(false);
    expect(store.canRedo).toBe(true);

    await store.redo();
    expect(store.revision).toBe(3);
    expect(store.characteristics.map((c) => c.id)).toEqual(["c1"]);
    expect(store.canRedo).toBe(false);
    expect(rust.calls).toEqual(["execute", "undo", "redo"]);
  });

  it("shows the change when execute resolves, even before the event", async () => {
    const { rust, store } = await connected();
    rust.eventsDelayed = true;
    rust.script = [
      { changes: addChanges("c1", 1, 0), canUndo: true, canRedo: false },
      { changes: undoAddChanges("c1", 1, 0), canUndo: false, canRedo: true },
    ];
    const patch = await store.execute(ADD);
    expect(patch?.changes[0]?.type).toBe("characteristic_inserted");
    expect(store.characteristicById.has("c1")).toBe(true);
    // Undo waits for the command before it, so it sees the step to undo.
    await store.undo();
    expect(store.characteristics).toEqual([]);
    rust.flushEvents();
    expect(store.revision).toBe(2);
    expect(store.characteristics).toEqual([]);
  });

  it("runs edits in call order", async () => {
    const { rust, store } = await connected();
    rust.script = [
      { changes: addChanges("c1", 1, 0), canUndo: true, canRedo: false },
      { changes: undoAddChanges("c1", 1, 0), canUndo: false, canRedo: true },
      { changes: addChanges("c1", 1, 0), canUndo: true, canRedo: false },
    ];
    // Undo and redo are asked for before the command they depend on has finished.
    await Promise.all([store.execute(ADD), store.undo(), store.redo()]);
    expect(rust.calls).toEqual(["execute", "undo", "redo"]);
    expect(store.revision).toBe(3);
    expect(store.characteristics.map((c) => c.id)).toEqual(["c1"]);
  });

  it("does not call undo or redo without a step", async () => {
    const { rust, store } = await connected();
    await store.undo();
    await store.redo();
    expect(rust.calls).toEqual([]);
  });

  it("ignores old and repeated patches and resyncs after a gap", async () => {
    const { rust, store } = await connected();
    const event = (session: number, revision: number): ProjectPatched => ({
      session,
      revision,
      patch: { changes: addChanges(`c${String(revision)}`, revision, revision - 1) },
      status: status({ modified: true, can_undo: true }),
    });
    rust.emitPatched(event(1, 1));
    rust.emitPatched(event(1, 1));
    rust.emitPatched(event(0, 2));
    expect(store.characteristics.map((c) => c.id)).toEqual(["c1"]);

    // Revision 2 got lost: the store fetches the full state instead of applying 3.
    const project = { ...emptyProject(), characteristics: [] };
    const snapshot = loaded(1, project).snapshot;
    if (!snapshot) {
      throw new Error("fixture has a snapshot");
    }
    rust.state = { session: 1, snapshot: { ...snapshot, revision: 3 }, notice: null };
    rust.emitPatched(event(1, 3));
    await vi.waitFor(() => {
      expect(store.revision).toBe(3);
    });
    expect(store.characteristics).toEqual([]);
  });

  it("replaces the project on load and ignores older sessions", async () => {
    const { rust, store, shown } = await connected();
    for (const h of rust.handlers.loaded) {
      h({ payload: loaded(3, null) });
    }
    expect(store.isOpen).toBe(false);
    expect(store.drawing).toBeNull();
    expect(shown.at(-1)).toBeNull();
    for (const h of rust.handlers.loaded) {
      h({ payload: loaded(2) });
    }
    expect(store.isOpen).toBe(false);
  });

  it("keeps the recovery notice until dismissed", async () => {
    const { store } = await connected();
    store.load({
      ...loaded(2),
      notice: {
        restored_unsaved: true,
        recovered_changes: 4,
        dropped_incomplete_change: false,
        set_aside_journal: null,
        migrated_from: null,
      },
    });
    expect(store.notice?.recovered_changes).toBe(4);
    store.dismissNotice();
    expect(store.notice).toBeNull();
  });

  it("applies status events of its session", async () => {
    const { rust, store } = await connected();
    for (const h of rust.handlers.status) {
      h({ payload: { session: 1, status: status({ file_name: "part.dimo" }) } });
      h({ payload: { session: 0, status: status({ file_name: "old.dimo" }) } });
    }
    expect(store.status?.file_name).toBe("part.dimo");
  });

  it("asks before unsaved changes are dropped", async () => {
    const discard = await connected("discard");
    discard.store.status = status({ modified: true });
    await discard.store.newProject();
    expect(discard.ask).toHaveBeenCalledOnce();
    expect(discard.rust.calls).toEqual(["new(true)"]);

    const save = await connected("save");
    save.store.status = status({ modified: true });
    await save.store.open();
    expect(save.rust.calls).toEqual(["save", "open(false)"]);

    const cancel = await connected("cancel");
    cancel.store.status = status({ modified: true });
    await cancel.store.open();
    expect(cancel.rust.calls).toEqual([]);

    const clean = await connected("cancel");
    await clean.store.newProject();
    expect(clean.ask).not.toHaveBeenCalled();
    expect(clean.rust.calls).toEqual(["new(false)"]);
  });

  it("answers a window close request after asking", async () => {
    const { rust, store } = await connected("discard");
    store.status = status({ modified: true });
    for (const h of rust.handlers.close) {
      h({ payload: null });
    }
    await vi.waitFor(() => {
      expect(rust.calls).toEqual(["close(true)"]);
    });
  });

  it("keeps the error of a refused command", async () => {
    const { store } = await connected();
    expect(await store.execute(ADD)).toBeUndefined();
    expect(store.error).toEqual({ kind: "rejected", message: "nothing scripted" });
    store.dismissError();
    expect(store.error).toBeNull();
  });
});
