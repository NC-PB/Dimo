// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { CharId, ToleranceTableInfo } from "$lib/ipc/bindings";
import type { Tool } from "$lib/stores/balloon-tools.svelte";
import { emptyProject, loaded } from "$lib/stores/fixtures";
import { ProjectStore } from "$lib/stores/project.svelte";
import { SetupStore } from "$lib/stores/setup.svelte";
import { ToleranceStore } from "$lib/stores/tolerance.svelte";
import SetupPane from "./SetupPane.svelte";
import { fakeProjectApi, fakeToleranceApi } from "$lib/test/fakes";

const ISO_2768_1: ToleranceTableInfo = {
  table: { id: "iso-2768-1", version: 1 },
  title: "ISO 2768-1 general tolerances",
  usage: "general",
  draft: true,
  classes: ["f", "m", "c", "v"],
};

/** A project store over a fake Rust that records every command and answers with no change. */
function stores(options: { created: boolean; candidates?: CharId[] } = { created: true }) {
  const asked: string[] = [];
  const rust = fakeProjectApi();
  const api = rust.api;
  const sent = rust.commands;

  const project = new ProjectStore(api);
  const toleranceApi = fakeToleranceApi({
    toleranceTables: () => Promise.resolve({ status: "ok", data: [ISO_2768_1] }),
    generalToleranceCandidates: () => {
      asked.push("candidates");
      return Promise.resolve({ status: "ok", data: options.candidates ?? [] });
    },
  });
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

    expect(s.sent.map((c) => c.type)).toEqual(["update_tolerance_settings", "batch"]);
    expect(s.sent[0]).toEqual({
      type: "update_tolerance_settings",
      values: [
        { field: "general", value: { table: { id: "iso-2768-1", version: 1 }, class: "m" } },
      ],
    });
    const batch = s.sent[1];
    expect(batch?.type === "batch" && batch.commands).toEqual([
      {
        type: "update_sheet",
        sheet: emptyProject().revisions[0]?.sheets[0]?.id,
        values: [
          { field: "unit", value: "in" },
          { field: "scale_drawing", value: 2 },
          { field: "scale_actual", value: 1 },
        ],
      },
      {
        type: "update_project_info",
        // The part number was already BR-2041: only the customer is sent (MF F4).
        values: [{ field: "customer", value: "Muster AG" }],
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
