import { describe, expect, it } from "vitest";
import { afterBox, afterClick, forDrag } from "./pick";

const set = (...ids: string[]) => new Set(ids);

describe("multi select rules (FR-BAL-12)", () => {
  it("click selects one balloon, additive click toggles it", () => {
    expect(afterClick(set("a", "b"), "c", false)).toEqual(["c"]);
    expect(afterClick(set("a", "b"), "c", true)).toEqual(["a", "b", "c"]);
    expect(afterClick(set("a", "b"), "a", true)).toEqual(["b"]);
    expect(afterClick(set("a"), "a", false)).toEqual(["a"]);
  });

  it("click on empty drawing clears, unless additive", () => {
    expect(afterClick(set("a"), null, false)).toEqual([]);
    expect(afterClick(set("a"), null, true)).toEqual(["a"]);
  });

  it("dragging a selected balloon moves the whole selection", () => {
    expect(forDrag(set("a", "b"), "b", false)).toEqual(["a", "b"]);
    expect(forDrag(set("a", "b"), "c", false)).toEqual(["c"]);
    expect(forDrag(set("a", "b"), "c", true)).toEqual(["a", "b", "c"]);
  });

  it("box select adds or replaces", () => {
    expect(afterBox(set("a"), ["b", "a"], true)).toEqual(["a", "b"]);
    expect(afterBox(set("a"), ["b"], false)).toEqual(["b"]);
  });
});
