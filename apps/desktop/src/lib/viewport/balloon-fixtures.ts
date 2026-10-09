/**
 * Test helpers for the balloon tools and gestures (T1.6): a stand-in for the Rust commands and
 * a project with two balloons. Used by tests only.
 */

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
  balloon,
  characteristic,
  emptyProject,
  loaded,
  status,
} from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };
const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: "ok", data });

/** Records commands and answers each with the patch the test prepared in `reply`. */
export class FakeCommands implements ProjectCommands {
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

/**
 * A project with two balloons on `SHEET_ID`: `b-a` (characteristic `a`) centered at (100, 100)
 * with anchor (60, 140), `b-b` (characteristic `b`) at (200, 100) with anchor (240, 140).
 */
export function twoBalloons() {
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
  return { api, project, selection, tools };
}
