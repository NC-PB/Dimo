import { describe, expect, it } from "vitest";
import { characteristic } from "$lib/stores/fixtures";
import { SelectionStore } from "$lib/stores/selection.svelte";
import { COLUMNS, NO_UNIT, choiceOptions, displayText, fieldValue, rawText } from "./columns";
import { lockExplanation, refusalDetail } from "./refusal";
import { moveStep, moveToGap } from "./reorder";

describe("table columns (T1.7, FR-CHR-02)", () => {
  it("has one column per M1 field, editable except the number", () => {
    expect(COLUMNS.map((c) => c.id)).toEqual([
      "number",
      "kind",
      "requirement_text",
      "nominal",
      "upper_dev",
      "lower_dev",
      "upper_limit",
      "lower_limit",
      "unit",
      "fit",
      "quantity",
      "classification",
      "inspection_method",
      "gauge",
      "sampling",
      "frequency",
      "comment",
      "inspect",
    ]);
    expect(COLUMNS.filter((c) => c.editor === "none").map((c) => c.id)).toEqual(["number"]);
  });

  it("shows stored decimals unchanged", () => {
    const c = {
      ...characteristic("a", 3),
      nominal: "90.0",
      upper_dev: "0.020",
      lower_dev: "-0.1",
      upper_limit: "90.020",
      lower_limit: "89.9",
    };
    expect(displayText(c, "nominal")).toBe("90.0");
    expect(displayText(c, "upper_dev")).toBe("0.020");
    expect(displayText(c, "lower_limit")).toBe("89.9");
    expect(displayText(c, "number")).toBe("3");
    expect(rawText({ ...c, nominal: null }, "nominal")).toBe("");
  });

  it("sends decimals as typed text, without parsing them (rule 5)", () => {
    expect(fieldValue("nominal", " 12.50 ")).toEqual({ field: "nominal", value: "12.50" });
    expect(fieldValue("upper_dev", "1,5")).toEqual({ field: "upper_dev", value: "1,5" });
    expect(fieldValue("lower_limit", "")).toEqual({ field: "lower_limit", value: null });
    expect(fieldValue("fit", " H7 ")).toEqual({ field: "fit", value: "H7" });
    expect(fieldValue("fit", "")).toEqual({ field: "fit", value: null });
  });

  it("maps text, quantity and choices to field values", () => {
    expect(fieldValue("requirement_text", "Ø8 f7")).toEqual({
      field: "requirement_text",
      value: "Ø8 f7",
    });
    expect(fieldValue("inspection_method", "CMM")).toEqual({
      field: "inspection_method",
      value: "CMM",
    });
    expect(fieldValue("quantity", "4")).toEqual({ field: "quantity", value: 4 });
    expect(fieldValue("quantity", "")).toBeNull();
    expect(fieldValue("kind", "diameter")).toEqual({ field: "kind", value: "diameter" });
    expect(fieldValue("kind", "bogus")).toBeNull();
    expect(fieldValue("classification", "key")).toEqual({ field: "classification", value: "key" });
    expect(fieldValue("unit", NO_UNIT)).toEqual({ field: "unit", value: null });
    expect(fieldValue("unit", "in")).toEqual({ field: "unit", value: "in" });
    expect(fieldValue("number", "5")).toBeNull();
  });

  it("offers every kind, classification and unit", () => {
    expect(choiceOptions("kind")).toHaveLength(16);
    expect(choiceOptions("classification").map((o) => o.value)).toEqual([
      "none",
      "critical",
      "major",
      "minor",
      "key",
    ]);
    expect(choiceOptions("unit").map((o) => o.value)).toEqual([NO_UNIT, "mm", "in", "deg"]);
  });
});

describe("reorder (T1.7, FR-BAL-06)", () => {
  const order = ["a", "b", "c", "d", "e"];

  it("moves rows into a gap before the next row that stays", () => {
    expect(moveToGap(order, new Set(["d"]), 1)).toEqual({
      type: "move_characteristics",
      ids: ["d"],
      before: "b",
    });
    expect(moveToGap(order, new Set(["a"]), 5)).toEqual({
      type: "move_characteristics",
      ids: ["a"],
      before: null,
    });
  });

  it("keeps the placement order of several moved rows", () => {
    expect(moveToGap(order, new Set(["e", "b"]), 0)).toEqual({
      type: "move_characteristics",
      ids: ["b", "e"],
      before: "a",
    });
    // A gap inside the moved rows goes before the next row that is not moved.
    expect(moveToGap(order, new Set(["b", "c"]), 2)).toBeNull();
    expect(moveToGap(order, new Set(["a", "c"]), 2)).toEqual({
      type: "move_characteristics",
      ids: ["a", "c"],
      before: "d",
    });
  });

  it("sends nothing when the order would not change", () => {
    expect(moveToGap(order, new Set(["c"]), 2)).toBeNull();
    expect(moveToGap(order, new Set(["c"]), 3)).toBeNull();
    expect(moveToGap(order, new Set(), 0)).toBeNull();
  });

  it("moves one step up and down", () => {
    expect(moveStep(order, new Set(["c"]), -1)).toEqual({
      type: "move_characteristics",
      ids: ["c"],
      before: "b",
    });
    expect(moveStep(order, new Set(["c"]), 1)).toEqual({
      type: "move_characteristics",
      ids: ["c"],
      before: "e",
    });
    expect(moveStep(order, new Set(["d"]), 1)).toEqual({
      type: "move_characteristics",
      ids: ["d"],
      before: null,
    });
    expect(moveStep(order, new Set(["a"]), -1)).toBeNull();
    expect(moveStep(order, new Set(["e"]), 1)).toBeNull();
  });
});

describe("selection store (T1.6, T1.7)", () => {
  it("replaces, adds and toggles", () => {
    const s = new SelectionStore();
    s.select(["a", "b"]);
    expect([...s.ids]).toEqual(["a", "b"]);
    s.select(["c"], "add");
    expect(s.has("c")).toBe(true);
    s.select(["a", "d"], "toggle");
    expect([...s.ids].sort()).toEqual(["b", "c", "d"]);
    s.select(["x"], "replace");
    expect([...s.ids]).toEqual(["x"]);
    s.clear();
    expect(s.ids.size).toBe(0);
  });

  it("counts focus requests so the same characteristic can be asked for again", () => {
    const s = new SelectionStore();
    s.focus("a");
    s.focus("a");
    expect(s.focusRequest).toEqual({ id: "a", seq: 2 });
  });
});

describe("refusals", () => {
  it("shows Rust's reason without the Tauri prefix", () => {
    expect(
      refusalDetail({
        kind: "invalid_argument",
        message:
          'invalid args `command` for command `execute`: invalid decimal string "1,5", expected digits',
      }),
    ).toBe('invalid decimal string "1,5", expected digits');
    expect(refusalDetail({ kind: "rejected", message: "numbering is locked" })).toBe(
      "numbering is locked",
    );
  });

  it("explains a numbering lock with who and when", () => {
    const text = lockExplanation({
      reason: "issued_report",
      locked_at: "2026-10-09T10:00:00Z",
      locked_by: "peter",
      insert_policy: "next_free",
      highest_number: 12,
    });
    expect(text).toContain("peter");
    expect(text).toContain("2026");
  });
});
