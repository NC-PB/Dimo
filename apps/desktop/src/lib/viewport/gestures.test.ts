import { describe, expect, it } from "vitest";
import type {
  Command,
  CommandError,
  Patch,
  ProjectLoaded,
  ProjectPatched,
} from "$lib/ipc/bindings";
import { BalloonToolsStore } from "$lib/stores/balloon-tools.svelte";
import {
  SHEET_ID,
  addChanges,
  balloon,
  characteristic,
  emptyProject,
  loaded,
  status,
} from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";
import { BalloonGestures, upRight, type PointerInput } from "./gestures.svelte";
import type { ViewTransform } from "./view-math";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };
const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: "ok", data });

/** Records commands and answers each with the patch the test prepared. */
class FakeCommands implements ProjectCommands {
  commands: Command[] = [];
  revision = 0;
  reply: Patch = { changes: [] };
  projectState = () => ok<ProjectLoaded>(loaded(1));
  newProject = () => ok(true);
  openProject = () => ok(true);
  saveProject = () => ok(true);
  saveProjectAs = () => ok(true);
  confirmClose = () => ok(null);
  undo = () => ok(this.#patched());
  redo = () => ok(this.#patched());
  execute = (command: Command) => {
    this.commands.push(command);
    return ok(this.#patched());
  };

  #patched(): ProjectPatched {
    this.revision += 1;
    return { session: 1, revision: this.revision, patch: this.reply, status: status() };
  }
}

/** Two balloons: `b-a` centered at (100, 100) with anchor (60, 140), `b-b` at (200, 100). */
function setup(view: ViewTransform = { scale: 1, tx: 0, ty: 0, rotation: 0 }) {
  const api = new FakeCommands();
  const project = new ProjectStore(api);
  const p = emptyProject();
  p.characteristics = [characteristic("a", 1), characteristic("b", 2)];
  p.balloons = [
    { ...balloon("b-a", "a"), position: { x: 100, y: 100 }, anchor: { x: 60, y: 140 } },
    { ...balloon("b-b", "b"), position: { x: 200, y: 100 }, anchor: { x: 240, y: 140 } },
  ];
  project.load(loaded(1, p));
  const selection = new SelectionStore();
  const tools = new BalloonToolsStore(project, selection, () => SHEET_ID);
  const pans: [number, number][] = [];
  const gestures = new BalloonGestures({
    project,
    selection,
    tools,
    view: () => view,
    sheet: () => ({ width: 800, height: 600 }),
    panBy: (dx, dy) => pans.push([dx, dy]),
  });
  return { api, project, selection, tools, gestures, pans };
}

let time = 0;
function at(x: number, y: number, mods: Partial<PointerInput> = {}): PointerInput {
  time += 1000;
  return {
    pointerId: 1,
    button: 0,
    shiftKey: false,
    metaKey: false,
    ctrlKey: false,
    x,
    y,
    timeStamp: time,
    ...mods,
  };
}

async function click(g: BalloonGestures, x: number, y: number, mods: Partial<PointerInput> = {}) {
  g.down(at(x, y, mods));
  await g.up(at(x, y, mods));
}

async function drag(
  g: BalloonGestures,
  from: [number, number],
  to: [number, number],
  mods: Partial<PointerInput> = {},
) {
  g.down(at(...from, mods));
  g.move(at((from[0] + to[0]) / 2, (from[1] + to[1]) / 2, mods));
  g.move(at(...to, mods));
  await g.up(at(...to, mods));
}

describe("balloon gestures (T1.6)", () => {
  it("click selects a balloon, additive click adds and toggles", async () => {
    const { gestures, selection } = setup();
    await click(gestures, 102, 98);
    expect([...selection.ids]).toEqual(["a"]);
    await click(gestures, 200, 100, { shiftKey: true });
    expect([...selection.ids]).toEqual(["a", "b"]);
    await click(gestures, 100, 100, { metaKey: true });
    expect([...selection.ids]).toEqual(["b"]);
  });

  it("click on empty drawing clears the selection, a drag pans instead", async () => {
    const { gestures, selection, pans, api } = setup();
    selection.select(["a"]);
    await drag(gestures, [400, 400], [420, 410]);
    expect(pans.length).toBeGreaterThan(0);
    expect([...selection.ids]).toEqual(["a"]);
    await click(gestures, 400, 400);
    expect(selection.isEmpty).toBe(true);
    expect(api.commands).toEqual([]);
  });

  it("drags the whole selection with a preview and one command on release", async () => {
    const { gestures, selection, api } = setup();
    selection.select(["a", "b"]);
    gestures.down(at(100, 100));
    gestures.move(at(110, 105));
    gestures.move(at(130, 120));
    expect(gestures.shown.map((b) => b.position)).toEqual([
      { x: 130, y: 120 },
      { x: 230, y: 120 },
    ]);
    // Anchors stay on the drawing during the preview.
    expect(gestures.shown[0]?.anchor).toEqual({ x: 60, y: 140 });
    await gestures.up(at(130, 120));
    expect(api.commands).toEqual([
      {
        type: "move_balloons",
        moves: [
          { id: "b-a", position: { x: 130, y: 120 }, anchor: null },
          { id: "b-b", position: { x: 230, y: 120 }, anchor: null },
        ],
      },
    ]);
    expect(gestures.active).toBe(false);
  });

  it("dragging an unselected balloon selects it alone and moves only it", async () => {
    const { gestures, selection, api } = setup();
    selection.select(["b"]);
    await drag(gestures, [100, 100], [100, 150]);
    expect([...selection.ids]).toEqual(["a"]);
    expect(api.commands).toEqual([
      { type: "move_balloons", moves: [{ id: "b-a", position: { x: 100, y: 150 }, anchor: null }] },
    ]);
  });

  it("works in screen pixels at any zoom: a 2 px wiggle is still a click", async () => {
    const { gestures, selection, api } = setup({ scale: 4, tx: -200, ty: -200, rotation: 0 });
    // Sheet (100, 100) is at screen (200, 200).
    gestures.down(at(200, 200));
    gestures.move(at(202, 201));
    await gestures.up(at(202, 201));
    expect([...selection.ids]).toEqual(["a"]);
    expect(api.commands).toEqual([]);
  });

  it("moves the leader anchor of a selected balloon by its handle", async () => {
    const { gestures, selection, api } = setup();
    selection.select(["a"]);
    await drag(gestures, [61, 139], [80, 160]);
    expect(api.commands).toEqual([
      {
        type: "move_balloons",
        moves: [{ id: "b-a", position: { x: 100, y: 100 }, anchor: { x: 80, y: 160 } }],
      },
    ]);
  });

  it("Shift drag on empty drawing box selects by balloon center", async () => {
    const { gestures, selection } = setup();
    selection.select(["b"]);
    await drag(gestures, [50, 50], [150, 150], { shiftKey: true });
    expect([...selection.ids]).toEqual(["b", "a"]);
  });

  it("Escape drops a drag without a command", async () => {
    const { gestures, selection, api } = setup();
    selection.select(["a"]);
    gestures.down(at(100, 100));
    gestures.move(at(150, 150));
    expect(gestures.cancel()).toBe(true);
    await gestures.up(at(150, 150));
    expect(api.commands).toEqual([]);
    expect(gestures.shown[0]?.position).toEqual({ x: 100, y: 100 });
  });

  it("place tool: a click adds a balloon up and right, selects it and opens the editor", async () => {
    const { gestures, selection, tools, api } = setup();
    tools.setTool("place");
    api.reply = { changes: addChanges("c", 3, 2) };
    await click(gestures, 400, 300);
    const command = api.commands[0];
    expect(command?.type).toBe("add_characteristic");
    if (command?.type === "add_characteristic") {
      expect(command.anchor).toEqual({ x: 400, y: 300 });
      expect(command.position.x).toBeGreaterThan(400);
      expect(command.position.y).toBeLessThan(300);
      expect(command.region).toBeNull();
      expect(command.sheet).toBe(SHEET_ID);
    }
    expect([...selection.ids]).toEqual(["c"]);
    expect(selection.focusRequest?.from).toBe("viewport");
    expect(tools.editing).toBe("c");
  });

  it("place tool: a drag stores the region as source (FR-CHR-09)", async () => {
    const { gestures, tools, api } = setup();
    tools.setTool("place");
    api.reply = { changes: addChanges("c", 3, 2) };
    await drag(gestures, [420, 310], [380, 290]);
    const command = api.commands[0];
    expect(command?.type === "add_characteristic" && command.region).toEqual({
      center: { x: 400, y: 300 },
      size: { width: 40, height: 20 },
      angle: 0,
    });
  });

  it("knows which sheet direction is up and right on screen", () => {
    const view = (rotation: ViewTransform["rotation"]) => ({ scale: 1, tx: 0, ty: 0, rotation });
    expect(upRight(view(0))).toEqual({ x: 1, y: -1 });
    expect(upRight(view(90))).toEqual({ x: -1, y: -1 });
    expect(upRight(view(180))).toEqual({ x: -1, y: 1 });
    expect(upRight(view(270))).toEqual({ x: 1, y: 1 });
  });
});
