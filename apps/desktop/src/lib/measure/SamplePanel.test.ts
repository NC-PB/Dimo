// @vitest-environment happy-dom
/**
 * The part pane of the Measure tab (T3.19): fields of the active part and its counts per
 * verdict as Rust computes them.
 */
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MeasurementState } from "$lib/ipc/bindings";
import { emptyProject, loaded } from "$lib/stores/fixtures";
import { MeasurementStore } from "$lib/stores/measurement.svelte";
import { ProjectStore } from "$lib/stores/project.svelte";
import SamplePanel from "./SamplePanel.svelte";
import { fakeProjectApi, ok } from "$lib/test/fakes";

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
  const rust = fakeProjectApi();
  const api = rust.api;
  const sent = rust.commands;

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

  it("sends each changed field alone, also when typed fast (T3.18, MF F4)", async () => {
    const { root, sent } = await render();
    const field = (label: string): HTMLInputElement => {
      const l = [...root.querySelectorAll("label")].find((e) => e.textContent.trim() === label);
      const input = root.querySelector<HTMLInputElement>(`#${l?.htmlFor ?? "none"}`);
      if (!input) {
        throw new Error(`no ${label} field`);
      }
      return input;
    };
    // Two fields changed before the first command has finished: two commands with one field.
    const gauges = field("Gauges");
    gauges.value = "Caliper 150";
    gauges.dispatchEvent(new Event("change", { bubbles: true }));
    const comment = field("Comment");
    comment.value = "first off";
    comment.dispatchEvent(new Event("change", { bubbles: true }));
    await vi.waitFor(() => {
      expect(sent).toHaveLength(2);
    });
    expect(sent).toEqual([
      {
        type: "update_sample",
        sample: "s1",
        values: [{ field: "gauges", value: "Caliper 150" }],
      },
      {
        type: "update_sample",
        sample: "s1",
        values: [{ field: "comment", value: "first off" }],
      },
    ]);
  });
});
