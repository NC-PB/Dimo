// @vitest-environment happy-dom
/**
 * File > Info and the project settings dialogs of the Drawing tab (T3.16): every edit is an
 * undoable Rust command, nothing is changed in the frontend.
 */
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import ProjectBalloonStyle from "$lib/components/ProjectBalloonStyle.svelte";
import ToleranceSettings from "$lib/components/ToleranceSettings.svelte";
import type { Command, MeasurementState } from "$lib/ipc/bindings";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { MeasurementStore } from "$lib/stores/measurement.svelte";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { ToleranceStore, type ToleranceApi } from "$lib/stores/tolerance.svelte";
import FileInfo from "./FileInfo.svelte";

const ok = <T>(data: T) => Promise.resolve({ status: "ok" as const, data });
const unused = () => Promise.reject(new Error("not used"));

const MEASURED: MeasurementState = {
  rows: [],
  samples: [
    {
      sample: "s1",
      counts: { pass: 2, fail: 1, not_evaluated: 0, missing: 0 },
      verdict: "fail",
    },
    {
      sample: "s2",
      counts: { pass: 1, fail: 0, not_evaluated: 0, missing: 2 },
      verdict: "open",
    },
  ],
  counts: { pass: 3, fail: 1, not_evaluated: 0, missing: 2 },
  verdict: "fail",
  // Rust's counts, deliberately not derivable from the fixture: the page shows them as given.
  to_inspect: 4,
  complete_samples: 1,
};

function setup() {
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
  p.characteristics = [characteristic("c1", 1), { ...characteristic("c2", 2), inspect: false }];
  project.load(loaded(1, p));
  const toleranceApi: ToleranceApi = {
    toleranceTables: () =>
      ok([
        {
          table: { id: "iso-2768-1", version: 1 },
          title: "ISO 2768-1 general tolerances",
          usage: "general",
          draft: true,
          classes: ["f", "m"],
        },
      ]),
    explainCharacteristic: () => ok(null),
    characteristicHistory: () => ok([]),
    reinterpretCharacteristics: unused,
    importToleranceTable: () => ok({ result: "cancelled" }),
    setShorterLeg: unused,
    generalToleranceCandidates: () => ok([]),
    applyGeneralTolerance: unused,
  };
  const tolerance = new ToleranceStore(toleranceApi, project);
  const measurement = new MeasurementStore({
    measurementState: () => ok(MEASURED),
    changedSinceIssuedReport: () => ok([]),
  });
  return { sent, project, tolerance, measurement };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement = document.createElement("div");

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

async function settle(): Promise<void> {
  for (let i = 0; i < 4; i++) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

function change(selector: string, value: string): void {
  const field = target.querySelector<HTMLInputElement | HTMLSelectElement>(selector);
  expect(field, selector).not.toBeNull();
  if (field) {
    field.value = value;
    field.dispatchEvent(new Event("change", { bubbles: true }));
  }
}

function render(
  c: typeof FileInfo | typeof ToleranceSettings | typeof ProjectBalloonStyle,
  props: object,
) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(c as typeof FileInfo, { target, props: props as Record<string, never> });
  flushSync();
}

describe("File > Info (T3.16)", () => {
  it("sends part, revision, report and general tolerance edits as commands", async () => {
    const s = setup();
    render(FileInfo, {
      projectStore: s.project,
      toleranceStore: s.tolerance,
      measurementStore: s.measurement,
    });
    await s.tolerance.refreshTables();
    await settle();
    change('input[id$="-part_number"]', "4711");
    await settle();
    change('input[id$="-revision"]', "C");
    await settle();
    change('input[id$="-report_number"]', "IR-7");
    await settle();
    change("[data-field='general-tolerance']", "iso-2768-1|m");
    await settle();
    expect(s.sent.map((c) => c.type)).toEqual([
      "update_project_info",
      "update_revision_label",
      "update_report_info",
      "set_tolerance_settings",
    ]);
    expect(s.sent[0]).toMatchObject({ info: { part_number: "4711" } });
    expect(s.sent[1]).toMatchObject({ label: "C" });
    expect(s.sent[2]).toMatchObject({ info: { report_number: "IR-7" } });
    expect(s.sent[3]).toMatchObject({
      settings: { general: { table: { id: "iso-2768-1" }, class: "m" } },
    });
  });

  it("shows the progress counts Rust returns", async () => {
    const s = setup();
    render(FileInfo, {
      projectStore: s.project,
      toleranceStore: s.tolerance,
      measurementStore: s.measurement,
    });
    await settle();
    const progress = target.querySelector("[data-testid='info-progress']")?.textContent ?? "";
    expect(progress).toContain("Characteristics 2");
    expect(progress).toContain("To inspect 4");
    expect(progress).toContain("1 of 2");
    expect(progress).toContain("Not OK (1)");
  });
});

describe("project settings dialogs of the Drawing tab (T3.16)", () => {
  it("adds a decimal place rule with set_tolerance_settings", async () => {
    const s = setup();
    render(ToleranceSettings, {
      section: "decimal_rules",
      project: s.project,
      tolerance: s.tolerance,
    });
    await settle();
    const inputs = target.querySelectorAll<HTMLInputElement>("form input");
    const [placesInput, toleranceInput] = [...inputs];
    if (placesInput && toleranceInput) {
      placesInput.value = "2";
      placesInput.dispatchEvent(new Event("input", { bubbles: true }));
      toleranceInput.value = "0.05";
      toleranceInput.dispatchEvent(new Event("input", { bubbles: true }));
    }
    target.querySelector("form")?.dispatchEvent(new Event("submit", { bubbles: true }));
    await settle();
    expect(s.sent.at(-1)).toMatchObject({
      type: "set_tolerance_settings",
      settings: { decimal_rules: [{ places: 2, tolerance: "0.05" }] },
    });
  });

  it("changes the project balloon style with set_default_balloon_style", async () => {
    const s = setup();
    render(ProjectBalloonStyle, { project: s.project });
    await settle();
    const leader = target.querySelector<HTMLInputElement>("input[type=checkbox]");
    leader?.click();
    await settle();
    expect(s.sent.at(-1)).toMatchObject({
      type: "set_default_balloon_style",
      style: { leader: false, shape: "circle" },
    });
  });
});
