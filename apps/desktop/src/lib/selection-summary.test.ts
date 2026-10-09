import { describe, expect, it } from "vitest";
import { characteristic } from "$lib/stores/fixtures";
import { selectedCharacteristics } from "./selection-summary";

describe("selection summary (T1.9)", () => {
  it("lists existing selected characteristics in number order", () => {
    const byId = new Map([
      ["a", characteristic("a", 3)],
      ["b", characteristic("b", 1)],
      ["c", characteristic("c", 2)],
    ]);
    const chosen = selectedCharacteristics(new Set(["a", "b", "gone"]), byId);
    expect(chosen.map((c) => c.number)).toEqual([1, 3]);
    expect(selectedCharacteristics(new Set(), byId)).toEqual([]);
  });
});
