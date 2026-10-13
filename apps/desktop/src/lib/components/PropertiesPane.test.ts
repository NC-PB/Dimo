// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { CharId, Command, Project } from "$lib/ipc/bindings";
import { PaneFolds } from "$lib/pane/folds.svelte";
import { characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { NumberingStore, type NumberingApi } from "$lib/stores/numbering.svelte";
import {
  ProjectStore,
  projectStore as appProject,
  type ProjectCommands,
} from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";
import { sheetRegions } from "$lib/stores/sheet-regions.svelte";
import { ToleranceStore, type ToleranceApi } from "$lib/stores/tolerance.svelte";
import { ViewStore, view as appView } from "$lib/stores/view.svelte";
import PropertiesPane from "./PropertiesPane.svelte";

function projectOf(): Project {
  const project = emptyProject();
  project.info.part_number = "4711-B";
  project.characteristics = [
    { ...characteristic("c1", 1), requirement_text: "Ø8 f7", inspect: true },
    { ...characteristic("c2", 2), requirement_text: "20 ±0.1", inspect: false },
    { ...characteristic("c3", 3), requirement_text: "M8", inspect: true },
  ];
  return project;
}

function setup() {
  const commands: Command[] = [];
  const reinterpreted: CharId[][] = [];
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1, projectOf()) }),
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
  const toleranceApi: ToleranceApi = {
    toleranceTables: () => Promise.resolve({ status: "ok", data: [] }),
    explainCharacteristic: (id) => Promise.resolve({ status: "ok", data: `explained ${id}` }),
    characteristicHistory: () => Promise.resolve({ status: "ok", data: [] }),
    reinterpretCharacteristics: (ids) => {
      reinterpreted.push(ids);
      return Promise.resolve({
        status: "ok",
        data: {
          patched: { session: 1, revision: 0, patch: { changes: [] }, status: status() },
          reinterpreted: ids.length,
          skipped_manual: 0,
          skipped_unreadable: 0,
        },
      });
    },
    importToleranceTable: () => Promise.resolve({ status: "ok", data: { result: "cancelled" } }),
    setShorterLeg: () => Promise.reject(new Error("not used")),
    generalToleranceCandidates: () => Promise.resolve({ status: "ok", data: [] }),
    applyGeneralTolerance: () => Promise.reject(new Error("not used")),
  };
  const numberingApi: NumberingApi = {
    previewNumbering: (strategy) =>
      Promise.resolve({
        status: "ok",
        data: { strategy, entries: [{ id: "c1", number: "2" }], changed: 2 },
      }),
  };
  const project = new ProjectStore(api);
  project.load(loaded(1, projectOf()));
  return {
    project,
    selected: new SelectionStore(),
    tolerance: new ToleranceStore(toleranceApi, project),
    numbering: new NumberingStore(numberingApi, project),
    view: new ViewStore(),
    commands,
    reinterpreted,
  };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

function render(stores: ReturnType<typeof setup>) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(PropertiesPane, {
    target,
    props: {
      project: stores.project,
      selected: stores.selected,
      tolerance: stores.tolerance,
      numbering: stores.numbering,
      view: stores.view,
    },
  });
  flushSync();
}

async function settle() {
  for (let i = 0; i < 5; i++) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  appProject.load(loaded(9000, null));
  try {
    localStorage.clear();
  } catch {
    // No storage in this environment.
  }
});

function title(): string {
  return target.querySelector("aside h2")?.textContent.trim() ?? "";
}

function sections(): string[] {
  return [...target.querySelectorAll<HTMLElement>("[data-section]")].map(
    (s) => s.dataset.section ?? "",
  );
}

describe("properties pane follows the selection (T3.18)", () => {
  it("shows the project summary with the way to File > Info when nothing is selected", async () => {
    const stores = setup();
    render(stores);
    await settle();
    expect(title()).toBe("Project");
    expect(target.querySelector("aside")?.getAttribute("aria-label")).toBe("Project summary");
    expect(target.querySelector("[data-field='part_number']")?.textContent.trim()).toBe("4711-B");
    expect(target.querySelector("[data-field='customer']")?.textContent.trim()).toBe("not set");
    expect(target.querySelector("[data-field='characteristics']")?.textContent.trim()).toBe("3");
    expect(target.querySelector('[data-testid="empty-state"]')).not.toBeNull();
    // Read only: no field edits a project value here.
    expect(
      target.querySelectorAll("[data-testid='project-properties'] :is(input, select, textarea)"),
    ).toHaveLength(0);
    const link = [...target.querySelectorAll("button")].find((b) =>
      b.textContent.includes("File > Info"),
    );
    link?.click();
    expect(appView.tab).toBe("file");
    expect(appView.filePane).toBe("info");
    appView.setTab("home");
    expect(stores.commands).toEqual([]);
  });

  it("shows one characteristic with its four sections", async () => {
    const stores = setup();
    stores.selected.select(["c2"]);
    render(stores);
    await settle();
    expect(title()).toBe("Characteristic 2");
    expect(sections()).toEqual([
      "characteristic.requirement",
      "characteristic.tolerance",
      "characteristic.inspection",
      "characteristic.history",
    ]);
    expect(target.querySelector<HTMLInputElement>("[data-field='requirement_text']")?.value).toBe(
      "20 ±0.1",
    );
    expect(target.querySelector("[data-field='explanation']")?.textContent.trim()).toBe(
      "explained c2",
    );
  });

  it("shows several characteristics with the list and the actions for all of them", async () => {
    const stores = setup();
    stores.selected.select(["c1", "c2", "c3"]);
    render(stores);
    await settle();
    expect(title()).toBe("3 characteristics selected");
    expect(sections()).toEqual(["selection.list", "selection.actions"]);
    expect(target.querySelectorAll("[data-section='selection.list'] li")).toHaveLength(3);

    target.querySelector<HTMLButtonElement>("[data-action='update_limits']")?.click();
    await settle();
    expect(stores.reinterpreted).toEqual([["c1", "c2", "c3"]]);
    expect(target.querySelector("[role='status']")?.textContent).toContain("Re-interpreted: 3");

    // Inspect yes sends one command for the ones that are not inspected yet.
    target.querySelector<HTMLButtonElement>("[data-inspect='yes']")?.click();
    await settle();
    expect(stores.commands).toEqual([
      { type: "update_fields", ids: ["c2"], values: [{ field: "inspect", value: true }] },
    ]);
  });

  it("follows a changed selection and gives way to it from a ribbon section", async () => {
    const stores = setup();
    render(stores);
    stores.view.paneTool = "views";
    appProject.load(loaded(9001, projectOf()));
    await settle();
    expect(title()).toBe("Views");
    expect(sections()).toEqual(["tool.views"]);
    sheetRegions.toggle("view");
    stores.selected.select(["c1"]);
    await settle();
    expect(stores.view.paneTool).toBeNull();
    // The drawing tool of the closed section stops too, so no drag draws a hidden region.
    expect(sheetRegions.tool).toBeNull();
    expect(title()).toBe("Characteristic 1");
    stores.selected.clear();
    await settle();
    expect(title()).toBe("Project");
  });

  it("closes a ribbon section with its close button", async () => {
    const stores = setup();
    appProject.load(loaded(9002, projectOf()));
    render(stores);
    stores.view.paneTool = "zones";
    await settle();
    expect(title()).toBe("Zone grid");
    expect(target.textContent).toContain("No zone grid");
    target.querySelector<HTMLButtonElement>("button[aria-label='Close Zone grid']")?.click();
    await settle();
    expect(stores.view.paneTool).toBeNull();
    expect(title()).toBe("Project");
  });

  it("says how many numbers change while the preview is on, and closes it", async () => {
    const stores = setup();
    render(stores);
    stores.numbering.setPreviewing(true);
    await stores.numbering.refresh();
    await settle();
    expect(sections()[0]).toBe("numbering.preview");
    expect(target.querySelector("[data-testid='numbering-changes']")?.textContent).toContain("2");
    target
      .querySelector<HTMLButtonElement>("button[aria-label='Close Numbering preview']")
      ?.click();
    await settle();
    expect(stores.numbering.previewing).toBe(false);
    expect(sections()).not.toContain("numbering.preview");
  });

  it("hides itself with the close button of the header", async () => {
    const stores = setup();
    render(stores);
    target.querySelector<HTMLButtonElement>("[data-testid='pane-close']")?.click();
    expect(stores.view.showSidePanel).toBe(false);
  });
});

describe("folding pane sections (T3.18)", () => {
  it("folds a section, remembers it in storage and starts folded the next time", async () => {
    const stores = setup();
    stores.selected.select(["c1"]);
    render(stores);
    await settle();
    const heading = (): HTMLButtonElement | undefined =>
      [...target.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find(
        (b) => b.textContent.trim() === "Inspection",
      );
    expect(heading()?.getAttribute("aria-expanded")).toBe("true");
    heading()?.click();
    await settle();
    expect(heading()?.getAttribute("aria-expanded")).toBe("false");
    expect(target.querySelector("[data-field='gauge']")).toBeNull();
    expect(JSON.parse(localStorage.getItem("dimo.pane.folded") ?? "[]")).toEqual([
      "characteristic.inspection",
    ]);
    // A new folds store (the next start of the app) reads the remembered state.
    const next = new PaneFolds();
    expect(next.isOpen("characteristic.inspection")).toBe(false);
    expect(next.isOpen("characteristic.tolerance")).toBe(true);
    heading()?.click();
    await settle();
    expect(target.querySelector("[data-field='gauge']")).not.toBeNull();
  });

  it("starts unfolded without storage or with broken storage", () => {
    expect(new PaneFolds(() => null).isOpen("x")).toBe(true);
    const broken = {
      getItem: () => "not json",
      setItem: () => {
        throw new Error("full");
      },
    } as unknown as Storage;
    const folds = new PaneFolds(() => broken);
    expect(folds.isOpen("x")).toBe(true);
    folds.toggle("x");
    expect(folds.isOpen("x")).toBe(false);
  });
});
