// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type {
  Characteristic,
  CommandError,
  TypedCallout,
  CharId,
  Command,
  HistoryEntry,
  Project,
  ProjectPatched,
  TableImport,
  ToleranceDerivation,
} from "$lib/ipc/bindings";
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
import { ToleranceStore, type ToleranceApi } from "$lib/stores/tolerance.svelte";
import CharacteristicDetail from "./CharacteristicDetail.svelte";
import ToleranceSettings from "./ToleranceSettings.svelte";

const DRAFT_GENERAL: ToleranceDerivation = {
  rule: {
    rule: "general",
    lookup: {
      table: { id: "iso-2768-1", version: 1 },
      part: "linear",
      class: "m",
      range: { min: null, max: null },
    },
  },
  draft: true,
  hints: [],
  conversion: null,
};

function withDerivation(id: string, number: number, derivation: ToleranceDerivation) {
  return { ...characteristic(id, number), derivation } satisfies Characteristic;
}

function projectOf(): Project {
  const project = emptyProject();
  project.characteristics = [
    withDerivation("c1", 1, DRAFT_GENERAL),
    withDerivation("c2", 2, {
      rule: { rule: "no_tolerance_defined" },
      draft: false,
      hints: [],
      conversion: null,
    }),
    withDerivation("c3", 3, {
      rule: { rule: "explicit" },
      draft: false,
      hints: [
        {
          hint: "fit_deviations_differ",
          fit: "H7",
          table_upper_dev: "0.021",
          table_lower_dev: "0",
        },
      ],
      conversion: null,
    }),
    withDerivation("c4", 4, {
      rule: { rule: "no_tolerance_defined" },
      draft: false,
      hints: [{ hint: "reference_dimension" }],
      conversion: null,
    }),
    {
      ...characteristic("c5", 5),
      kind: "angle",
      requirement_text: "30°",
      nominal: "30",
      unit: "deg",
      quantity: 2,
      inspect: false,
    },
  ];
  project.balloons = [balloon("b5", "c5")];
  return project;
}

/** What Rust reads for `30°` with a shorter leg of 40 mm and ISO 2768-m (T2.9a). */
const ANGLE_READING: TypedCallout = {
  values: [
    { field: "requirement_text", value: "30°" },
    { field: "kind", value: "angle" },
    { field: "nominal", value: "30" },
    { field: "unit", value: "deg" },
    { field: "upper_limit", value: "30.5" },
    { field: "lower_limit", value: "29.5" },
    { field: "quantity", value: 1 },
    { field: "inspect", value: true },
    { field: "shorter_leg", value: { value: "40", unit: "mm" } },
  ],
  parse_error: null,
  explanation: "The shorter leg is 40 mm.",
  notes: [],
};

function patched(): ProjectPatched {
  return { session: 1, revision: 1, patch: { changes: [] }, status: status() };
}

const HISTORY: HistoryEntry[] = [
  {
    timestamp: "2026-10-10T08:00:00Z",
    user: "anna",
    action: "command",
    command: { type: "unlock_numbering" },
    source: "recognition",
    before: null,
    after: characteristic("c1", 1),
    fields: [],
  },
  {
    timestamp: "2026-10-10T09:00:00Z",
    user: "ben",
    action: "command",
    command: { type: "unlock_numbering" },
    source: "rule",
    before: characteristic("c1", 1),
    after: characteristic("c1", 1),
    fields: ["upper_limit", "derivation"],
  },
];

function setup(importResult: TableImport = { result: "cancelled" }) {
  const commands: Command[] = [];
  const reinterpreted: CharId[][] = [];
  const reads: [string, string, string | null, string][] = [];
  const leg = { refuse: false };
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1, projectOf()) }),
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
  const toleranceApi: ToleranceApi = {
    toleranceTables: () =>
      Promise.resolve({
        status: "ok",
        data: [
          {
            table: { id: "iso-2768-1", version: 1 },
            title: "ISO 2768-1 general tolerances",
            usage: "general",
            draft: true,
            classes: ["f", "m", "c", "v"],
          },
          {
            table: { id: "iso-286", version: 2 },
            title: "ISO 286",
            usage: "fit",
            draft: true,
            classes: [],
          },
        ],
      }),
    explainCharacteristic: (id, language) =>
      Promise.resolve({ status: "ok", data: `explained ${id} in ${language}` }),
    characteristicHistory: () => Promise.resolve({ status: "ok", data: HISTORY }),
    reinterpretCharacteristics: (ids) => {
      reinterpreted.push(ids);
      return Promise.resolve({
        status: "ok",
        data: { patched: patched(), reinterpreted: 1, skipped_manual: 1, skipped_unreadable: 0 },
      });
    },
    importToleranceTable: () => Promise.resolve({ status: "ok", data: importResult }),
    readCalloutText: (sheet, text, shorterLeg, language) => {
      reads.push([sheet, text, shorterLeg, language]);
      const refused: CommandError = {
        kind: "invalid_argument",
        message: "the shorter leg is not a positive number",
      };
      return Promise.resolve(
        leg.refuse ? { status: "error", error: refused } : { status: "ok", data: ANGLE_READING },
      );
    },
  };
  const projectStore = new ProjectStore(api);
  projectStore.load(loaded(1, projectOf()));
  const selection = new SelectionStore();
  const tolerance = new ToleranceStore(toleranceApi, projectStore);
  return { projectStore, selection, tolerance, commands, reinterpreted, reads, leg };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

function render(stores: ReturnType<typeof setup>) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(CharacteristicDetail, {
    target,
    props: {
      project: stores.projectStore,
      selected: stores.selection,
      tolerance: stores.tolerance,
    },
  });
  flushSync();
}

async function settle() {
  for (let i = 0; i < 4; i++) {
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
});

function badges(): string[] {
  return [...target.querySelectorAll<HTMLElement>("[data-badge]")].map(
    (b) => b.dataset.badge ?? "",
  );
}

describe("characteristic detail panel (T2.8)", () => {
  it("shows the rule, the draft badge and Rust's explanation", async () => {
    const stores = setup();
    stores.selection.select(["c1"]);
    render(stores);
    await settle();
    expect(target.querySelector("h2")?.textContent).toContain("1");
    expect(target.querySelector("[data-field='rule']")?.textContent).toContain("General tolerance");
    expect(badges()).toEqual(["draft"]);
    expect(target.querySelector("[data-field='explanation']")?.textContent?.trim()).toBe(
      "explained c1 in en",
    );
  });

  it("marks no tolerance, hints and reference dimensions with their own shapes", async () => {
    const stores = setup();
    render(stores);
    stores.selection.select(["c2"]);
    await settle();
    expect(badges()).toEqual(["no_tolerance"]);
    expect(target.querySelector("[data-badge='no_tolerance'] svg")).not.toBeNull();
    stores.selection.select(["c3"]);
    await settle();
    expect(badges()).toEqual(["hint"]);
    expect(target.querySelector("[data-badge='hint']")?.getAttribute("title")).toContain("H7");
    stores.selection.select(["c4"]);
    await settle();
    // A reference dimension is not "no tolerance": it gets the reference badge only.
    expect(badges()).toEqual(["reference"]);
  });

  it("keeps inspect editable", async () => {
    const stores = setup();
    stores.selection.select(["c4"]);
    render(stores);
    await settle();
    const box = target.querySelector<HTMLInputElement>("input[type='checkbox']");
    box?.click();
    await settle();
    expect(stores.commands).toContainEqual({
      type: "update_fields",
      ids: ["c4"],
      values: [{ field: "inspect", value: false }],
    });
  });

  it("lists the history newest first with source and fields (FR-CHR-10)", async () => {
    const stores = setup();
    stores.selection.select(["c1"]);
    render(stores);
    await settle();
    target.querySelector<HTMLButtonElement>("#detail-tab-history")?.click();
    await settle();
    const items = [...target.querySelectorAll<HTMLElement>("[data-testid='detail-history'] li")];
    expect(items.map((li) => li.dataset.source)).toEqual(["rule", "recognition"]);
    expect(items[0]?.textContent).toContain("Upper limit, Rule");
    expect(items[0]?.textContent).toContain("ben");
    expect(items[1]?.textContent).toContain("Created");
  });

  it("re-interprets the selection with one request", async () => {
    const stores = setup();
    stores.selection.select(["c1", "c3"]);
    render(stores);
    await settle();
    const button = [...target.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Re-interpret"),
    );
    expect(button?.textContent).toContain("(2)");
    button?.click();
    await settle();
    expect(stores.reinterpreted).toEqual([["c1", "c3"]]);
    expect(target.querySelector("[role='status']")?.textContent).toContain("by hand: 1");
  });
});

describe("shorter leg of an angle (T2.9a)", () => {
  function legInput(): HTMLInputElement | null {
    return target.querySelector<HTMLInputElement>("input[data-field='shorter_leg']");
  }

  function enter(text: string) {
    const input = legInput();
    if (input) {
      input.value = text;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }
  }

  it("is offered for angles only, in the sheet unit", async () => {
    const stores = setup();
    stores.selection.select(["c1"]);
    render(stores);
    await settle();
    expect(legInput()).toBeNull();
    stores.selection.select(["c5"]);
    await settle();
    expect(legInput()?.value).toBe("");
    expect(target.textContent).toContain("Shorter leg (mm)");
  });

  it("reads the text again with the leg and stores leg and limits in one command", async () => {
    const stores = setup();
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("40");
    await settle();
    expect(stores.reads).toEqual([[SHEET_ID, "30°", "40", "en"]]);
    // Kind, quantity and inspect stay as the user left them.
    expect(stores.commands).toEqual([
      {
        type: "update_fields",
        ids: ["c5"],
        values: [
          { field: "nominal", value: "30" },
          { field: "unit", value: "deg" },
          { field: "upper_limit", value: "30.5" },
          { field: "lower_limit", value: "29.5" },
          { field: "shorter_leg", value: { value: "40", unit: "mm" } },
        ],
      },
    ]);
    expect(target.querySelector("[role='alert']")).toBeNull();
  });

  it("clears the leg with empty text", async () => {
    const stores = setup();
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("  ");
    await settle();
    expect(stores.reads).toEqual([[SHEET_ID, "30°", null, "en"]]);
  });

  it("shows a leg Rust refuses and changes nothing", async () => {
    const stores = setup();
    stores.leg.refuse = true;
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("abc");
    await settle();
    expect(stores.commands).toEqual([]);
    expect(legInput()?.getAttribute("aria-invalid")).toBe("true");
    expect(target.querySelector("[role='alert']")?.textContent).toContain(
      "Enter a positive number",
    );
  });
});

describe("tolerance settings (T2.8)", () => {
  function renderSettings(stores: ReturnType<typeof setup>) {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(ToleranceSettings, {
      target,
      props: { project: stores.projectStore, tolerance: stores.tolerance },
    });
    flushSync();
  }

  it("sets the general standard and class as one command", async () => {
    const stores = setup();
    renderSettings(stores);
    await settle();
    const general = target.querySelector<HTMLSelectElement>("[data-setting='general']");
    const options = [...(general?.options ?? [])].map((o) => o.value);
    // Fit tables are not general tolerances.
    expect(options).toEqual(["", "iso-2768-1"]);
    if (general) {
      general.value = "iso-2768-1";
      general.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await settle();
    expect(stores.commands.at(-1)).toMatchObject({
      type: "set_tolerance_settings",
      settings: { general: { table: { id: "iso-2768-1", version: 1 }, class: "f" } },
    });
  });

  it("shows a refused import with its line", async () => {
    const stores = setup({ result: "invalid", message: "shop.toml: bad value", line: 6 });
    renderSettings(stores);
    await settle();
    const button = [...target.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Import"),
    );
    button?.click();
    await settle();
    expect(target.querySelector("[data-testid='import-refusal']")?.textContent).toContain(
      "line 6: shop.toml: bad value",
    );
  });
});
