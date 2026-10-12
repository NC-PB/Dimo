// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Command, Sample } from "$lib/ipc/bindings";
import { emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
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
  component = mount(SampleBar, { target, props: { project, activeSample } });
  flushSync();
  await tick();
  return { commands };
}

function button(text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

describe("sample bar (T3.5)", () => {
  it("adds the chosen number of samples", async () => {
    const { commands } = await render(null);
    const input = target.querySelector<HTMLInputElement>("input[type=number]");
    if (!input) {
      throw new Error("no input");
    }
    input.value = "3";
    input.dispatchEvent(new Event("input", { bubbles: true }));
    button("Add samples").click();
    await vi.waitFor(() => {
      expect(commands).toEqual([{ type: "add_samples", count: 3 }]);
    });
  });

  it("moves the active sample left and right", async () => {
    const { commands } = await render("s2");
    button("Move left").click();
    button("Move right").click();
    await vi.waitFor(() => {
      expect(commands).toEqual([
        { type: "move_samples", ids: ["s2"], before: "s1" },
        { type: "move_samples", ids: ["s2"], before: null },
      ]);
    });
  });

  it("asks before deleting a sample with results", async () => {
    const { commands } = await render("s2");
    button("Delete sample").click();
    flushSync();
    expect(target.textContent).toContain("Delete sample 2 with its 1 results?");
    expect(commands).toEqual([]);
    button("Delete with results").click();
    await vi.waitFor(() => {
      expect(commands).toEqual([{ type: "delete_samples", ids: ["s2"] }]);
    });
  });

  it("deletes a sample without results at once", async () => {
    const { commands } = await render("s3");
    button("Delete sample").click();
    await vi.waitFor(() => {
      expect(commands).toEqual([{ type: "delete_samples", ids: ["s3"] }]);
    });
  });
});
