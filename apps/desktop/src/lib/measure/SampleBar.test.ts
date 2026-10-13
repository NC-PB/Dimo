// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Command, Sample } from "$lib/ipc/bindings";
import { emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { PartsStore } from "./parts.svelte";
import SampleBar from "./SampleBar.svelte";

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

let target: HTMLElement;
let component: ReturnType<typeof mount> | null = null;

beforeEach(() => {
  target = document.createElement("div");
  document.body.appendChild(target);
});

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

async function render(activeSample: string | null) {
  const commands: Command[] = [];
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1) }),
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
  const data = emptyProject();
  data.samples = [1, 2, 3].map(sample);
  data.results = [
    {
      sample: "s2",
      characteristic: "c1",
      value: { type: "variable", value: "1" },
      source: { type: "manual" },
      comment: "",
    },
  ];
  const project = new ProjectStore(api);
  project.load(loaded(1, data));
  const parts = new PartsStore(project, () => activeSample);
  component = mount(SampleBar, { target, props: { parts } });
  flushSync();
  await tick();
  return { commands, parts };
}

function button(text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

describe("parts of the Measure tab (T3.5, T3.15)", () => {
  it("shows nothing while there is nothing to say", async () => {
    await render("s2");
    expect(target.textContent.trim()).toBe("");
  });

  it("adds the chosen number of parts", async () => {
    const { commands, parts } = await render(null);
    parts.count = 3;
    await parts.add();
    expect(commands).toEqual([{ type: "add_samples", count: 3 }]);
  });

  it("moves the active part left and right", async () => {
    const { commands, parts } = await render("s2");
    await parts.moveBy(-1);
    await parts.moveBy(1);
    expect(commands).toEqual([
      { type: "move_samples", ids: ["s2"], before: "s1" },
      { type: "move_samples", ids: ["s2"], before: null },
    ]);
  });

  it("asks before deleting a part with results", async () => {
    const { commands, parts } = await render("s2");
    await parts.remove();
    flushSync();
    expect(target.textContent).toContain("2");
    expect(target.querySelector("[role=alert]")).not.toBeNull();
    expect(commands).toEqual([]);
    button("Delete with results").click();
    await vi.waitFor(() => {
      expect(commands).toEqual([{ type: "delete_samples", ids: ["s2"] }]);
    });
  });

  it("keeps the part when the question is answered with Keep", async () => {
    const { commands, parts } = await render("s2");
    await parts.remove();
    flushSync();
    button("Keep").click();
    flushSync();
    expect(parts.confirming).toBeNull();
    expect(commands).toEqual([]);
  });

  it("deletes a part without results at once", async () => {
    const { commands, parts } = await render("s3");
    await parts.remove();
    expect(commands).toEqual([{ type: "delete_samples", ids: ["s3"] }]);
  });
});
