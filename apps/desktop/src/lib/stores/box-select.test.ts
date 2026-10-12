// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProposalCard from "$lib/components/ProposalCard.svelte";
import type {
  BalloonPlacement,
  CommandError,
  ExplainLanguage,
  OrientedBox,
  Proposal,
  ProposalView,
  SheetId,
  TypedCallout,
} from "$lib/ipc/bindings";
import { FakeCommands } from "$lib/viewport/balloon-fixtures";
import type { Placement } from "$lib/viewport/balloons";
import { BalloonToolsStore } from "./balloon-tools.svelte";
import { BoxSelectStore, withReading, type RecognitionCommands } from "./box-select.svelte";
import { SHEET_ID, characteristic, emptyProject, loaded } from "./fixtures";
import { ProjectStore } from "./project.svelte";
import { SelectionStore } from "./selection.svelte";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };
const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: "ok", data });

const RECT = { x: 550, y: 145, width: 90, height: 28 };
const BOX: OrientedBox = {
  center: { x: 595, y: 159 },
  size: { width: 90, height: 28 },
  angle: 0,
};
const PLACEMENT: Placement = {
  position: { x: 660, y: 125 },
  anchor: { x: 640, y: 145 },
};

/** What Rust proposes for `Ø30 H7 +0.0203 -0` with the fallback interpreter. */
function proposal(text = "Ø30 H7 +0.0203 -0"): Proposal {
  return {
    kind: "diameter",
    requirement_text: text,
    nominal: "30",
    unit: "mm",
    upper_dev: "0.0203",
    lower_dev: "0",
    upper_limit: "30.0203",
    lower_limit: "30",
    fit: "H7",
    derivation: { rule: { rule: "explicit" }, draft: false, hints: [], conversion: null },
    quantity: 1,
    inspect: true,
    source: { sheet: SHEET_ID, region: BOX, text_source: { type: "pdf_text" }, raw_text: text },
    origin: "box_select",
    placement: PLACEMENT,
    parse_error: null,
    parse_hints: ["stacked_lines_joined"],
    job_id: 4,
    engines: [{ name: "dimo-notation", version: "0.0.0" }],
  };
}

/** Stand-in for the recognition commands: answers with prepared proposals and readings. */
class FakeRecognition implements RecognitionCommands {
  proposals: Proposal[] = [proposal()];
  reads: string[] = [];
  legs: (string | null)[] = [];
  refuseLeg = false;
  reading: (text: string) => TypedCallout = (text) => ({
    values: [
      { field: "requirement_text", value: text },
      { field: "kind", value: "linear" },
      { field: "nominal", value: "25" },
      { field: "upper_limit", value: "25.1" },
      { field: "lower_limit", value: "24.9" },
    ],
    parse_error: null,
    explanation: "Tolerance written on the drawing.",
    notes: [],
  });
  proposeFromRegion = (
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
    language: ExplainLanguage,
  ) => {
    expect([sheet, region, placement, language]).toEqual([SHEET_ID, BOX, PLACEMENT, "en"]);
    return ok<ProposalView[]>(
      this.proposals.map((proposal) => ({
        proposal,
        explanation: "Printed deviations differ from ISO 286 H7.",
        notes: ["fit_pair"],
      })),
    );
  };
  readCalloutText = (
    _sheet: SheetId,
    text: string,
    shorterLeg: string | null,
  ): Promise<Result<TypedCallout>> => {
    this.reads.push(text);
    this.legs.push(shorterLeg);
    if (this.refuseLeg) {
      return Promise.resolve({
        status: "error",
        error: { kind: "invalid_argument", message: "not a positive number" },
      });
    }
    return ok(this.reading(text));
  };
}

function setup() {
  const api = new FakeCommands();
  const project = new ProjectStore(api);
  project.load(loaded(1, emptyProject()));
  const selection = new SelectionStore();
  const recognition = new FakeRecognition();
  const store = new BoxSelectStore(project, selection, recognition);
  const tools = new BalloonToolsStore(project, selection, () => SHEET_ID, store, recognition);
  return { api, project, selection, recognition, store, tools };
}

describe("box select (T2.6, FR-REC-01, ADR 0006)", () => {
  it("opens the card with the proposals and changes nothing until accepted", async () => {
    const { api, store, tools } = setup();
    const id = await tools.place(PLACEMENT, RECT);
    expect(id).toBeNull();
    expect(store.open).toBe(true);
    expect(store.region).toEqual(RECT);
    expect(store.proposals[0]?.upper_limit).toBe("30.0203");
    expect(api.commands).toEqual([]);
  });

  it("accepts all proposals as one command and selects the new characteristics", async () => {
    const { api, selection, store, tools } = setup();
    await tools.place(PLACEMENT, RECT);
    api.reply = {
      changes: [
        { type: "characteristic_inserted", index: 0, characteristic: characteristic("n1", 1) },
      ],
    };
    const ids = await store.accept();
    expect(ids).toEqual(["n1"]);
    expect(api.commands).toEqual([
      { type: "accept_proposals", proposals: [proposal()], insert_after: null },
    ]);
    expect(selection.ids.has("n1")).toBe(true);
    expect(store.open).toBe(false);
  });

  it("accepts after the primary selection, so locked numbers follow it (FR-BAL-11)", async () => {
    const { api, project, selection, store, tools } = setup();
    const p = emptyProject();
    p.characteristics = [characteristic("a", 1), characteristic("b", 2)];
    project.load(loaded(1, p), true);
    selection.select(["b", "a"]);
    await tools.place(PLACEMENT, RECT);
    await store.accept();
    expect(api.commands).toMatchObject([{ type: "accept_proposals", insert_after: "a" }]);
  });

  it("discards without a command", async () => {
    const { api, store, tools } = setup();
    await tools.place(PLACEMENT, RECT);
    expect(tools.escape()).toBe(true);
    expect(store.open).toBe(false);
    expect(api.commands).toEqual([]);
  });

  it("reads edited text again through Rust before accepting", async () => {
    const { api, recognition, store, tools } = setup();
    await tools.place(PLACEMENT, RECT);
    store.edit(0, { requirement_text: "25±0.1" });
    await store.accept();
    expect(recognition.reads).toEqual(["25±0.1"]);
    const sent = api.commands[0];
    expect(sent?.type).toBe("accept_proposals");
    const accepted = sent?.type === "accept_proposals" ? sent.proposals[0] : undefined;
    expect(accepted?.kind).toBe("linear");
    expect([accepted?.upper_limit, accepted?.lower_limit]).toEqual(["25.1", "24.9"]);
    // Box and placement stay those of the box.
    expect(accepted?.source.region).toEqual(BOX);
    expect(accepted?.origin).toBe("box_select");
  });

  it("falls back to manual capture when the box holds no text", async () => {
    const { api, recognition, store, tools } = setup();
    recognition.proposals = [];
    await tools.place(PLACEMENT, RECT);
    expect(store.open).toBe(false);
    expect(api.commands[0]?.type).toBe("add_characteristic");
  });

  it("a text that does not parse clears values and keeps the parse error", () => {
    const next = withReading(proposal(), {
      values: [{ field: "requirement_text", value: "SEE NOTE" }],
      parse_error: { position: 0, expected: "dimension" },
      explanation: null,
      notes: [],
    });
    expect(next.requirement_text).toBe("SEE NOTE");
    expect([next.nominal, next.upper_limit, next.lower_limit, next.derivation]).toEqual([
      null,
      null,
      null,
      null,
    ]);
    expect(next.parse_error?.position).toBe(0);
  });
});

/** What Rust proposes for `30°` under ISO 2768-m without a shorter leg (T2.9a). */
function angle(): Proposal {
  return {
    ...proposal("30°"),
    kind: "angle",
    nominal: "30",
    unit: "deg",
    upper_dev: null,
    lower_dev: null,
    upper_limit: null,
    lower_limit: null,
    fit: null,
    derivation: {
      rule: { rule: "no_tolerance_defined" },
      draft: false,
      hints: [],
      conversion: null,
    },
    parse_hints: [],
  };
}

/** What Rust reads for `30°` with a shorter leg of 40 mm. */
function angleReading(): TypedCallout {
  return {
    values: [
      { field: "requirement_text", value: "30°" },
      { field: "kind", value: "angle" },
      { field: "upper_limit", value: "30.5" },
      { field: "lower_limit", value: "29.5" },
      { field: "shorter_leg", value: { value: "40", unit: "mm" } },
    ],
    parse_error: null,
    explanation: "The shorter leg is 40 mm. Limits 29.5° to 30.5°.",
    notes: [],
  };
}

describe("shorter leg on the card (T2.9a)", () => {
  it("reads the proposal again with the typed leg and accepts it with the leg", async () => {
    const { api, recognition, store, tools } = setup();
    recognition.proposals = [angle()];
    recognition.reading = angleReading;
    await tools.place(PLACEMENT, RECT);
    expect(store.legs).toEqual([""]);
    store.editLeg(0, "40");
    await store.reread(0);
    expect(recognition.legs).toEqual(["40"]);
    expect(store.proposals[0]?.upper_limit).toBe("30.5");
    expect(store.proposals[0]?.shorter_leg).toEqual({ value: "40", unit: "mm" });
    expect(store.explanations[0]).toContain("Limits 29.5° to 30.5°");
    // Nothing changed since: accepting does not read again.
    await store.accept();
    expect(recognition.legs).toEqual(["40"]);
    const sent = api.commands[0];
    const accepted = sent?.type === "accept_proposals" ? sent.proposals[0] : undefined;
    expect(accepted?.shorter_leg).toEqual({ value: "40", unit: "mm" });
  });

  it("keeps the values and shows why when Rust refuses the leg", async () => {
    const { recognition, store, tools } = setup();
    recognition.proposals = [angle()];
    recognition.refuseLeg = true;
    await tools.place(PLACEMENT, RECT);
    store.editLeg(0, "abc");
    await store.reread(0);
    expect(store.legErrors[0]?.kind).toBe("invalid_argument");
    expect(store.proposals[0]?.upper_limit).toBeNull();
  });
});

describe("typed values (M2 decision 5)", () => {
  it("sets the fields Rust read from the text in one command", async () => {
    const { api, project, recognition, tools } = setup();
    const p = emptyProject();
    p.characteristics = [characteristic("a", 1)];
    project.load(loaded(1, p), true);
    await tools.commitText("a", "25±0.1");
    expect(recognition.reads).toEqual(["25±0.1"]);
    expect(recognition.legs).toEqual([null]);
    expect(api.commands).toEqual([
      { type: "update_fields", ids: ["a"], values: recognition.reading("25±0.1").values },
    ]);
  });
});

describe("proposal card", () => {
  let card: ReturnType<typeof mount> | null = null;
  afterEach(() => {
    if (card) {
      void unmount(card);
    }
    card = null;
    document.body.innerHTML = "";
  });

  /** Places a box; `typing` types keys on the drawing while Rust still reads the box. */
  async function shown(typing: string[] = []) {
    const s = setup();
    const placing = s.tools.place(PLACEMENT, RECT);
    const kept = typing.map((k) =>
      s.tools.typeAhead({ key: k, ctrlKey: false, metaKey: false, isComposing: false }),
    );
    expect(kept.every(Boolean)).toBe(true);
    await placing;
    let done = 0;
    const target = document.createElement("div");
    document.body.append(target);
    card = mount(ProposalCard, {
      target,
      props: {
        store: s.store,
        view: { scale: 1, tx: 0, ty: 0, rotation: 0 },
        width: 1600,
        height: 900,
        onDone: () => {
          done += 1;
        },
        takeTyped: () => s.tools.takeTyped("card"),
      },
    });
    flushSync();
    await tick();
    return { ...s, target, done: () => done };
  }

  it("shows text, kind, nominal, limits and rule, with the text field focused", async () => {
    const { target } = await shown();
    const input = target.querySelector("input");
    expect(input?.value).toBe("Ø30 H7 +0.0203 -0");
    expect(document.activeElement).toBe(input);
    expect(target.querySelector("select")?.value).toBe("diameter");
    const field = (name: string) =>
      target.querySelector(`[data-field=${name}]`)?.textContent.replace(/\s+/g, " ").trim();
    expect(field("nominal")).toBe("30 mm H7");
    expect(field("upper_limit")).toBe("30.0203");
    expect(field("lower_limit")).toBe("30");
    expect(field("rule")).toBe("Explicit (on the drawing)");
    expect(field("explanation")).toBe("Printed deviations differ from ISO 286 H7.");
    expect(target.textContent).toContain("Fit pair");
  });

  it("takes over keys typed before it opened: the text, then Enter accepts (T2.7a)", async () => {
    const s = await shown([..."25±0.1", "Enter"]);
    await vi.waitFor(() => {
      expect(s.done()).toBe(1);
    });
    expect(s.recognition.reads).toEqual(["25±0.1"]);
    const sent = s.api.commands[0];
    expect(sent?.type === "accept_proposals" && sent.proposals[0]?.requirement_text).toBe("25±0.1");
  });

  it("takes over typed text without Enter and keeps the cursor at its end", async () => {
    const { target, store } = await shown([..."25±0.2", "Backspace", "1"]);
    const input = target.querySelector("input");
    expect(input?.value).toBe("25±0.1");
    expect(document.activeElement).toBe(input);
    expect(input?.selectionStart).toBe(6);
    expect(store.proposals[0]?.requirement_text).toBe("25±0.1");
  });

  it("offers the shorter leg for an angle in the sheet unit and reads it on change", async () => {
    const plain = await shown();
    expect(plain.target.querySelector("[data-field=shorter_leg]")).toBeNull();
    if (card) {
      void unmount(card);
    }
    card = null;
    document.body.innerHTML = "";

    const s = setup();
    s.recognition.proposals = [angle()];
    s.recognition.reading = angleReading;
    await s.tools.place(PLACEMENT, RECT);
    const target = document.createElement("div");
    document.body.append(target);
    card = mount(ProposalCard, {
      target,
      props: {
        store: s.store,
        view: { scale: 1, tx: 0, ty: 0, rotation: 0 },
        width: 1600,
        height: 900,
        onDone: () => undefined,
      },
    });
    flushSync();
    await tick();
    expect(target.textContent).toContain("Shorter leg (mm)");
    const leg = target.querySelector<HTMLInputElement>("input[data-field=shorter_leg]");
    expect(leg?.value).toBe("");
    if (leg) {
      leg.value = "40";
      leg.dispatchEvent(new Event("input", { bubbles: true }));
      leg.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await vi.waitFor(() => {
      expect(target.querySelector("[data-field=upper_limit]")?.textContent.trim()).toBe("30.5");
    });
    expect(s.recognition.legs).toEqual(["40"]);
    expect(target.querySelector("[data-field=explanation]")?.textContent).toContain(
      "Limits 29.5° to 30.5°",
    );
  });

  it("Enter accepts and Esc discards", async () => {
    const first = await shown();
    first.target
      .querySelector("input")
      ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await vi.waitFor(() => {
      expect(first.done()).toBe(1);
    });
    expect(first.api.commands[0]?.type).toBe("accept_proposals");
    if (card) {
      void unmount(card);
    }
    card = null;

    const second = await shown();
    second.target
      .querySelector("input")
      ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flushSync();
    expect(second.store.open).toBe(false);
    expect(second.api.commands).toEqual([]);
    expect(second.target.querySelector("section")).toBeNull();
  });
});
