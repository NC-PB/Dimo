// @vitest-environment happy-dom
/**
 * The part pane of the Measure tab (T3.19): fields of the active part and its counts per
 * verdict as Rust computes them.
 */
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { Command, MeasurementState } from "$lib/ipc/bindings";
import { emptyProject, loaded, status } from "$lib/stores/fixtures";
import { MeasurementStore } from "$lib/stores/measurement.svelte";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import SamplePanel from "./SamplePanel.svelte";

const ok = <T>(data: T) => Promise.resolve({ status: "ok" as const, data });
const unused = () => Promise.reject(new Error("not used"));

const STATE: MeasurementState = {
  rows: [],
  samples: [
    { sample: "s1", counts: { pass: 4, fail: 2, not_evaluated: 0, missing: 0 }, verdict: "fail" },
  ],
  counts: { pass: 4, fail: 2, not_evaluated: 0, missing: 0 },
  verdict: "fail",
  to_inspect: 6,
  complete_samples: 1,
};

let target: HTMLElement | null = null;
let component: ReturnType<typeof mount> | null = null;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target?.remove();
});

async function render() {
  const sent: Command[] = [];
  const api: ProjectCommands = {
    projectState: unused,
    newProject: () => ok(true),
    openProject: () => ok(true),
    saveProject: () => ok(true),
    saveProjectAs: () => ok(true),
    confirmClose: () => ok(null),
    closeProject: () => ok(null),
    openRecentProject: () => ok(true),
    execute: (command) => {
      sent.push(command);
      return ok({ session: 1, revision: 0, patch: { changes: [] }, status: status() });
    },
    undo: unused,
    redo: unused,
  };
  const project = new ProjectStore(api);
  const p = emptyProject();
  p.samples = [
    { id: "s1", serial: "S-002", date: "", inspector: "A. Weber", gauges: "", comment: "" },
  ];
  project.load(loaded(1, p));
  const measurement = new MeasurementStore({
    measurementState: () => ok(STATE),
    changedSinceIssuedReport: () => ok([]),
  });
  await measurement.refresh(false);
  target = document.createElement("div");
  document.body.append(target);
  component = mount(SamplePanel, {
    target,
    props: { project, measurement, activeSample: "s1" },
  });
  flushSync();
  return { sent, root: target };
}

describe("part pane (T3.19)", () => {
  it("names the part and shows its counts from Rust", async () => {
    const { root } = await render();
    expect(root.querySelector("h2")?.textContent.trim()).toBe("Part S-002");
    const counts = root.querySelector("[data-testid=part-counts]")?.textContent ?? "";
    expect(counts.replace(/\s+/g, " ")).toContain("4 pass");
    expect(counts.replace(/\s+/g, " ")).toContain("2 fail");
    expect(counts.replace(/\s+/g, " ")).toContain("0 missing");
  });

  it("sends a changed field as one update_sample", async () => {
    const { root, sent } = await render();
    const gauges = [...root.querySelectorAll("label")].find(
      (l) => l.textContent.trim() === "Gauges",
    );
    const input = root.querySelector<HTMLInputElement>(`#${gauges?.htmlFor ?? "none"}`);
    if (!input) {
      throw new Error("no gauges field");
    }
    input.value = "Caliper 150";
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await tick();
    expect(sent).toEqual([
      {
        type: "update_sample",
        sample: {
          id: "s1",
          serial: "S-002",
          date: "",
          inspector: "A. Weber",
          gauges: "Caliper 150",
          comment: "",
        },
      },
    ]);
  });
});
