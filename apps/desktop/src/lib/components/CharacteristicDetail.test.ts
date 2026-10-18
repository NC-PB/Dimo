// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type {
  Characteristic,
  CommandError,
  LegOutcome,
  CharId,
  HistoryEntry,
  Project,
  ProjectPatched,
  TableImport,
  ToleranceDerivation,
  Unit,
} from "$lib/ipc/bindings";
import { balloon, characteristic, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore } from "$lib/stores/project.svelte";
import { SelectionStore } from "$lib/stores/selection.svelte";
import { ToleranceStore } from "$lib/stores/tolerance.svelte";
import CharacteristicDetail from "./CharacteristicDetail.svelte";
import GeneralToleranceField from "$lib/shell/GeneralToleranceField.svelte";
import GeneralToleranceOffer from "./GeneralToleranceOffer.svelte";
import ToleranceSettings from "./ToleranceSettings.svelte";
import { fakeProjectApi, fakeToleranceApi } from "$lib/test/fakes";
import { m, unitLabel } from "$lib/i18n";

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
  project.characteristics.push({
    ...characteristic("c6", 6),
    kind: "angle",
    requirement_text: "45°",
    shorter_leg: { value: "1.5", unit: "in" },
  });
  project.balloons = [balloon("b5", "c5"), balloon("b6", "c6")];
  return project;
}

function patched(): ProjectPatched {
  return { session: 1, revision: 1, patch: { changes: [] }, status: status(), drawing: null };
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
  const reinterpreted: CharId[][] = [];
  const legs: [CharId, string | null][] = [];
  const leg: { refuse: boolean; outcome: LegOutcome } = { refuse: false, outcome: "interpreted" };
  /** The unit Rust reads a typed leg in (`shorter_leg_unit`), and who asked for it. */
  const legUnit: { unit: Unit; asked: CharId[] } = { unit: "mm", asked: [] };
  /** What the fake Rust lists for the general tolerance offer, and how often it applied it. */
  const general: { candidates: CharId[]; asked: number; applied: number } = {
    candidates: ["c2"],
    asked: 0,
    applied: 0,
  };
  const rust = fakeProjectApi({ state: loaded(1, projectOf()) });
  const api = rust.api;
  const commands = rust.commands;

  const toleranceApi = fakeToleranceApi({
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
    setShorterLeg: (id, text) => {
      legs.push([id, text]);
      const refused: CommandError = {
        kind: "invalid_argument",
        reason: "shorter_leg",
        message: "the shorter leg is not a positive number",
      };
      return Promise.resolve(
        leg.refuse
          ? { status: "error", error: refused }
          : { status: "ok", data: { patched: patched(), outcome: leg.outcome } },
      );
    },
    shorterLegUnit: (id) => {
      legUnit.asked.push(id);
      return Promise.resolve({ status: "ok", data: legUnit.unit });
    },
    generalToleranceCandidates: () => {
      general.asked += 1;
      return Promise.resolve({ status: "ok", data: general.candidates });
    },
    applyGeneralTolerance: () => {
      general.applied += 1;
      return Promise.resolve({
        status: "ok",
        data: {
          patched: patched(),
          reinterpreted: general.candidates.length,
          skipped_manual: 0,
          skipped_unreadable: 0,
        },
      });
    },
  });
  const projectStore = new ProjectStore(api);
  projectStore.load(loaded(1, projectOf()));
  const selection = new SelectionStore();
  const tolerance = new ToleranceStore(toleranceApi, projectStore);
  return {
    projectStore,
    selection,
    tolerance,
    commands,
    reinterpreted,
    legs,
    leg,
    legUnit,
    general,
  };
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
    expect(
      target.querySelector<HTMLElement>("[data-testid='characteristic-properties']")?.dataset
        .characteristic,
    ).toBe("c1");
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
    const items = [...target.querySelectorAll<HTMLElement>("[data-testid='detail-history'] li")];
    expect(items.map((li) => li.dataset.source)).toEqual(["rule", "recognition"]);
    expect(items[0]?.textContent).toContain("Upper limit, Rule");
    expect(items[0]?.textContent).toContain("ben");
    expect(items[1]?.textContent).toContain("Created");
  });

  it("updates the limits of the shown characteristic with one request", async () => {
    const stores = setup();
    stores.selection.select(["c3"]);
    render(stores);
    await settle();
    target.querySelector<HTMLButtonElement>("[data-action='update_limits']")?.click();
    await settle();
    expect(stores.reinterpreted).toEqual([["c3"]]);
    expect(target.querySelector("[role='status']")?.textContent).toContain("by hand: 1");
  });

  it("edits text, kind, quantity, method and gauge as update_fields commands", async () => {
    const stores = setup();
    stores.selection.select(["c1"]);
    render(stores);
    await settle();
    const set = (field: string, value: string) => {
      const control = target.querySelector<HTMLInputElement | HTMLSelectElement>(
        `[data-field='${field}']`,
      );
      if (control) {
        control.value = value;
        control.dispatchEvent(new Event("change", { bubbles: true }));
      }
    };
    set("requirement_text", "Ø8 f7");
    await settle();
    set("kind", "diameter");
    await settle();
    set("quantity", "4");
    await settle();
    set("inspection_method", "CMM");
    await settle();
    set("gauge", "Plug gauge");
    await settle();
    expect(stores.commands).toEqual([
      {
        type: "update_fields",
        ids: ["c1"],
        values: [{ field: "requirement_text", value: "Ø8 f7" }],
      },
      { type: "update_fields", ids: ["c1"], values: [{ field: "kind", value: "diameter" }] },
      { type: "update_fields", ids: ["c1"], values: [{ field: "quantity", value: 4 }] },
      {
        type: "update_fields",
        ids: ["c1"],
        values: [{ field: "inspection_method", value: "CMM" }],
      },
      { type: "update_fields", ids: ["c1"], values: [{ field: "gauge", value: "Plug gauge" }] },
    ]);
  });

  it("asks for the history only while its section is open", async () => {
    const stores = setup();
    stores.selection.select(["c1"]);
    render(stores);
    await settle();
    expect(target.querySelector("[data-testid='detail-history']")).not.toBeNull();
    const heading = [...target.querySelectorAll<HTMLButtonElement>("button[aria-expanded]")].find(
      (b) => b.textContent.trim() === "History",
    );
    heading?.click();
    await settle();
    expect(heading?.getAttribute("aria-expanded")).toBe("false");
    expect(target.querySelector("[data-testid='detail-history']")).toBeNull();
    heading?.click();
    await settle();
    expect(target.querySelector("[data-testid='detail-history']")).not.toBeNull();
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
    expect(target.textContent).toContain(m.shorter_leg_label({ unit: unitLabel("mm") }));
  });

  it("labels the field with the unit Rust names for the characteristic (D-20, MF F8)", async () => {
    const stores = setup();
    stores.legUnit.unit = "in";
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    expect(stores.legUnit.asked).toContain("c5");
    expect(target.textContent).toContain(m.shorter_leg_label({ unit: unitLabel("in") }));
  });

  it("sends the typed leg to Rust, which reads and stores it as one undo step", async () => {
    const stores = setup();
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("40");
    await settle();
    expect(stores.legs).toEqual([["c5", "40"]]);
    // No document command from TypeScript: Rust applies the change and returns the patch.
    expect(stores.commands).toEqual([]);
    expect(target.querySelector("[role='alert']")).toBeNull();
    expect(target.querySelector("[role='status']")).toBeNull();
  });

  it("clears the leg with empty text", async () => {
    const stores = setup();
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("  ");
    await settle();
    expect(stores.legs).toEqual([["c5", null]]);
  });

  it("says when hand entered limits were kept or the text gives no limits", async () => {
    const stores = setup();
    stores.leg.outcome = "manual_kept";
    stores.selection.select(["c5"]);
    render(stores);
    await settle();
    enter("40");
    await settle();
    expect(target.querySelector("[role='status']")?.textContent).toContain("entered by hand");
    stores.leg.outcome = "unreadable";
    enter("8");
    await settle();
    expect(target.querySelector("[role='status']")?.textContent).toContain("not a dimension");
  });

  it("shows a stored leg in another unit beside the field, which takes the sheet unit", async () => {
    const stores = setup();
    stores.selection.select(["c6"]);
    render(stores);
    await settle();
    expect(target.textContent).toContain("Shorter leg (mm)");
    expect(legInput()?.value).toBe("");
    expect(target.querySelector("[data-field='shorter_leg_stored']")?.textContent).toContain(
      "1.5 in",
    );
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

describe("tolerance settings (T2.8, T3.16)", () => {
  /** The company tables dialog of the Drawing tab (T3.16). */
  function renderSettings(stores: ReturnType<typeof setup>) {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(ToleranceSettings, {
      target,
      props: {
        section: "company_tables",
        project: stores.projectStore,
        tolerance: stores.tolerance,
      },
    });
    flushSync();
  }

  /** The general tolerance field of the Home and Drawing tabs with the offer above the drawing. */
  let offer: ReturnType<typeof mount> | null = null;
  function renderGeneral(stores: ReturnType<typeof setup>) {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(GeneralToleranceField, { target, props: { tolerance: stores.tolerance } });
    offer = mount(GeneralToleranceOffer, { target, props: { tolerance: stores.tolerance } });
    void stores.tolerance.refreshTables(); // App.svelte does this for the ribbon field.
    flushSync();
  }

  afterEach(() => {
    if (offer) {
      void unmount(offer);
      offer = null;
    }
  });

  it("sets the general standard and class as one command", async () => {
    const stores = setup();
    renderGeneral(stores);
    await settle();
    const general = target.querySelector<HTMLSelectElement>("[data-field='general-tolerance']");
    const options = [...(general?.options ?? [])].map((o) => o.value);
    // Fit tables are not general tolerances.
    expect(options[0]).toBe("");
    expect(options.slice(1).every((o) => o.startsWith("iso-2768-1|"))).toBe(true);
    if (general) {
      general.value = "iso-2768-1|f";
      general.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await settle();
    expect(stores.commands.at(-1)).toEqual({
      type: "update_tolerance_settings",
      values: [
        { field: "general", value: { table: { id: "iso-2768-1", version: 1 }, class: "f" } },
      ],
    });
  });

  it("says that an identical table changed nothing", async () => {
    const stores = setup({ result: "unchanged", table: { id: "shop", version: 1 } });
    renderSettings(stores);
    await settle();
    const button = [...target.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Import"),
    );
    button?.click();
    await settle();
    const status = target.querySelector("[data-testid='import-status']")?.textContent ?? "";
    expect(status).toContain("shop v1 is already in the project. Nothing changed.");
    expect(status).not.toContain("Imported");
  });

  it("shows a refused import with its line", async () => {
    const stores = setup({
      result: "invalid",
      reason: "invalid_content",
      line: 6,
      message: "shop.toml: bad value",
    });
    renderSettings(stores);
    await settle();
    const button = [...target.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Import"),
    );
    button?.click();
    await settle();
    const refusal = target.querySelector("[data-testid='import-refusal']")?.textContent ?? "";
    expect(refusal).toContain("line 6: A value or a row breaks the rules for tables.");
    expect(refusal).not.toContain("shop.toml");
    // Rust's text is only the diagnostic detail, collapsed under "Details".
    expect(target.querySelector("[data-testid='error-details']")?.textContent).toContain(
      "shop.toml: bad value",
    );
  });

  function pickGeneral(value: string): void {
    const select = target.querySelector<HTMLSelectElement>("[data-field='general-tolerance']");
    if (select) {
      select.value = `${value}|f`;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }
  }

  function buttonNamed(text: string): HTMLButtonElement | undefined {
    return [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  }

  // T3.13, M3 decision 22: after the general tolerance changed, DimoMark offers the update.
  it("offers to update characteristics after the general tolerance changed", async () => {
    const stores = setup();
    stores.general.candidates = ["c1", "c2"];
    renderGeneral(stores);
    await settle();
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();
    pickGeneral("iso-2768-1");
    await settle();
    expect(stores.general.asked).toBe(1);
    const offer = target.querySelector("[data-testid='general-offer']")?.textContent ?? "";
    expect(offer).toContain(
      "2 characteristics have no tolerance yet or the previous general tolerance.",
    );
    buttonNamed("Update 2")?.click();
    await settle();
    expect(stores.general.applied).toBe(1);
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();
    expect(target.querySelector("[data-testid='general-updated']")?.textContent).toContain(
      "Characteristics updated: 2.",
    );
  });

  it("Not now closes the offer and changes nothing", async () => {
    const stores = setup();
    renderGeneral(stores);
    await settle();
    pickGeneral("iso-2768-1");
    await settle();
    expect(target.querySelector("[data-testid='general-offer']")?.textContent).toContain(
      "1 characteristic has no tolerance yet",
    );
    const commands = stores.commands.length;
    buttonNamed("Not now")?.click();
    await settle();
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();
    expect(stores.general.applied).toBe(0);
    expect(stores.commands.length).toBe(commands);
  });

  it("withdraws the offer when the project changes (undo, redo, edits)", async () => {
    const stores = setup();
    renderGeneral(stores);
    await settle();
    pickGeneral("iso-2768-1");
    await settle();
    expect(target.querySelector("[data-testid='general-offer']")).not.toBeNull();
    stores.projectStore.applyPatched({
      session: 1,
      revision: stores.projectStore.revision + 1,
      patch: { changes: [] },
      status: status(),
      drawing: null,
    });
    await settle();
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();
  });

  it("makes no offer when nothing would change or another setting changed", async () => {
    const stores = setup();
    stores.general.candidates = [];
    renderGeneral(stores);
    await settle();
    pickGeneral("iso-2768-1");
    await settle();
    expect(stores.general.asked).toBe(1);
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();

    stores.general.candidates = ["c2"];
    await stores.tolerance.setSettings([{ field: "mm_places", value: 3 }]);
    await settle();
    expect(stores.general.asked).toBe(1);
    expect(target.querySelector("[data-testid='general-offer']")).toBeNull();
  });
});
