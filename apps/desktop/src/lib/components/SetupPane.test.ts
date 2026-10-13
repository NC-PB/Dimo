// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { CharId, Command, ProjectPatched, ToleranceTableInfo } from "$lib/ipc/bindings";
import type { Tool } from "$lib/stores/balloon-tools.svelte";
import { emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { SetupStore } from "$lib/stores/setup.svelte";
import { ToleranceStore, type ToleranceApi } from "$lib/stores/tolerance.svelte";
import SetupPane from "./SetupPane.svelte";

const ISO_2768_1: ToleranceTableInfo = {
  table: { id: "iso-2768-1", version: 1 },
  title: "ISO 2768-1 general tolerances",
  usage: "general",
  draft: true,
  classes: ["f", "m", "c", "v"],
};

/** A project store over a fake Rust that records every command and answers with no change. */
function stores(options: { created: boolean; candidates?: CharId[] } = { created: true }) {
  const sent: Command[] = [];
  const asked: string[] = [];
  const patched = (): ProjectPatched => ({
    session: 1,
    revision: 0,
    patch: { changes: [] },
    status: status(),
  });
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1) }),
    newProject: () => Promise.resolve({ status: "ok", data: true }),
    openProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProjectAs: () => Promise.resolve({ status: "ok", data: true }),
    confirmClose: () => Promise.resolve({ status: "ok", data: null }),
    closeProject: () => Promise.resolve({ status: "ok", data: null }),
    openRecentProject: () => Promise.resolve({ status: "ok", data: true }),
    execute: (command) => {
      sent.push(command);
      return Promise.resolve({ status: "ok", data: patched() });
    },
    undo: () => Promise.reject(new Error("not used")),
    redo: () => Promise.reject(new Error("not used")),
  };
  const project = new ProjectStore(api);
  const unused = () => Promise.reject(new Error("not used"));
  const toleranceApi: ToleranceApi = {
    toleranceTables: () => Promise.resolve({ status: "ok", data: [ISO_2768_1] }),
    explainCharacteristic: unused,
    characteristicHistory: unused,
    reinterpretCharacteristics: unused,
    importToleranceTable: unused,
    setShorterLeg: unused,
    generalToleranceCandidates: () => {
      asked.push("candidates");
      return Promise.resolve({ status: "ok", data: options.candidates ?? [] });
    },
    applyGeneralTolerance: unused,
  };
  const tolerance = new ToleranceStore(toleranceApi, project);
  const tools: Tool[] = [];
  const shown: string[] = [];
  const setup = new SetupStore({
    project,
    tolerance,
    tools: { setTool: (tool) => tools.push(tool) },
    showDrawing: () => shown.push("drawing"),
  });
  const data = emptyProject();
  data.info.part_number = "BR-2041";
  project.load(loaded(1, data));
  if (options.created) {
    project.createdSession = 1;
  }
  return { project, tolerance, setup, sent, asked, tools, shown };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

async function render(s: ReturnType<typeof stores>) {
  await s.tolerance.refreshTables();
  target = document.createElement("div");
  document.body.append(target);
  component = mount(SetupPane, { target, props: { setup: s.setup, tables: s.tolerance } });
  flushSync();
}

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

function pane(): HTMLElement | null {
  return target.querySelector('[data-testid="setup-pane"]');
}

function button(text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

function radio(label: string): HTMLInputElement {
  const found = [...target.querySelectorAll("label")].find((l) =>
    l.textContent.trim().startsWith(label),
  );
  const input = found?.querySelector<HTMLInputElement>("input[type=radio]");
  if (!input) {
    throw new Error(`no radio ${label}`);
  }
  return input;
}

function field(label: string): HTMLInputElement | HTMLSelectElement {
  const found = [...target.querySelectorAll("label")].find((l) =>
    l.textContent.trim().startsWith(label),
  );
  const input = found?.querySelector<HTMLInputElement | HTMLSelectElement>("input, select");
  if (!input) {
    throw new Error(`no field ${label}`);
  }
  return input;
}

describe("setup pane (M3 decision 23, T3.17)", () => {
  it("lists None and the classes of Rust's general tables, and the project's values", async () => {
    const s = stores();
    await render(s);
    expect(pane()?.querySelector("h2")?.textContent).toBe("Set up this drawing");
    const labels = [...target.querySelectorAll<HTMLInputElement>("input[type=radio]")].map(
      (r) => r.closest("label")?.textContent.replace(/\s+/g, " ").trim() ?? "",
    );
    expect(labels).toEqual([
      "None Untoleranced dimensions get no limits",
      "ISO 2768-1 f fine",
      "ISO 2768-1 m medium",
      "ISO 2768-1 c coarse",
      "ISO 2768-1 v very coarse",
    ]);
    expect(radio("None").checked).toBe(true);
    expect(field("Unit").value).toBe("mm");
    expect(field("Scale").value).toBe("1:1");
    expect(field("Part number").value).toBe("BR-2041");
    expect(field("Customer").value).toBe("");
  });

  it("applies the answers through the existing commands and switches on Place balloon", async () => {
    const s = stores();
    await render(s);
    radio("ISO 2768-1 m").click();
    const unit = field("Unit") as HTMLSelectElement;
    unit.value = "in";
    unit.dispatchEvent(new Event("change", { bubbles: true }));
    const scale = field("Scale") as HTMLSelectElement;
    scale.value = "2:1";
    scale.dispatchEvent(new Event("change", { bubbles: true }));
    const customer = field("Customer") as HTMLInputElement;
    customer.value = "Muster AG";
    customer.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    button("Start ballooning").click();
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();

    expect(s.sent.map((c) => c.type)).toEqual(["set_tolerance_settings", "batch"]);
    const settings = s.sent[0];
    expect(settings?.type === "set_tolerance_settings" && settings.settings.general).toEqual({
      table: { id: "iso-2768-1", version: 1 },
      class: "m",
    });
    const batch = s.sent[1];
    expect(batch?.type === "batch" && batch.commands).toEqual([
      {
        type: "update_sheet",
        sheet: emptyProject().revisions[0]?.sheets[0]?.id,
        rotation: null,
        unit: "in",
        scale: { drawing: 2, actual: 1 },
      },
      {
        type: "update_project_info",
        info: { ...emptyProject().info, part_number: "BR-2041", customer: "Muster AG" },
      },
    ]);
    // The general tolerance changed, so the offer of decision 22 was asked for as usual.
    expect(s.asked).toEqual(["candidates"]);
    expect(s.tools).toEqual(["place"]);
    expect(s.setup.visible).toBe(false);
    expect(pane()).toBeNull();
  });

  it("sends nothing it did not change", async () => {
    const s = stores();
    await render(s);
    button("Start ballooning").click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(s.sent).toEqual([]);
    expect(s.tools).toEqual(["place"]);
    expect(s.setup.visible).toBe(false);
  });

  it("closes with Skip and changes nothing", async () => {
    const s = stores();
    await render(s);
    radio("ISO 2768-1 c").click();
    flushSync();
    button("Skip").click();
    flushSync();
    expect(pane()).toBeNull();
    expect(s.sent).toEqual([]);
    expect(s.tools).toEqual([]);
  });

  it("is not shown for an opened project, but opens on request", async () => {
    const s = stores({ created: false });
    await render(s);
    expect(pane()).toBeNull();
    s.setup.show();
    flushSync();
    expect(pane()).not.toBeNull();
    expect(s.shown).toEqual(["drawing"]);
    s.setup.toggle();
    flushSync();
    expect(pane()).toBeNull();
  });

  it("can be reopened after Skip and goes away with the project", async () => {
    const s = stores();
    await render(s);
    button("Skip").click();
    flushSync();
    s.setup.show();
    flushSync();
    expect(pane()).not.toBeNull();
    s.project.load(loaded(2));
    flushSync();
    expect(pane()).toBeNull();
  });
});
