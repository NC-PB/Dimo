import { describe, expect, it } from "vitest";
import { characteristic } from "$lib/stores/fixtures";
import { selectedCharacteristics } from "./selection-summary";

describe("selection summary (T1.9)", () => {
  it("lists existing selected characteristics in placement order, which is number order", () => {
    const byId = new Map([
      ["b", characteristic("b", 1)],
      ["c", { ...characteristic("c", 1), number: "1A" }],
      ["a", characteristic("a", 2)],
    ]);
    const chosen = selectedCharacteristics(new Set(["a", "c", "gone"]), byId);
    expect(chosen.map((c) => c.number)).toEqual(["1A", "2"]);
    expect(selectedCharacteristics(new Set(), byId)).toEqual([]);
  });
});
