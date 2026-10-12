import { describe, expect, it } from "vitest";
import type { MeasurementResult } from "$lib/ipc/bindings";
import { editText, parseBlock, pasteEntries, toggledText } from "./grid";

function result(value: MeasurementResult["value"]): MeasurementResult {
  return { sample: "s1", characteristic: "c1", value, source: { type: "manual" }, comment: "" };
}

describe("measurement grid helpers (T3.5)", () => {
  it.each([
    [
      "1\t2\n3\t4\n",
      [
        ["1", "2"],
        ["3", "4"],
      ],
    ],
    [
      "1\t2\r\n3\t4",
      [
        ["1", "2"],
        ["3", "4"],
      ],
    ],
    ["12,5", [["12,5"]]],
    ["a\t\tb\n", [["a", "", "b"]]],
    ["\n", []],
    ["", []],
  ])("parses the block %j", (text, block) => {
    expect(parseBlock(text)).toEqual(block);
  });

  it("maps a block onto rows and samples from the active cell", () => {
    const entries = pasteEntries([["1", "2"], ["3"]], ["c1", "c2", "c3"], ["s1", "s2"], 1, 0);
    expect(entries).toEqual([
      { sample: "s1", characteristic: "c2", text: "1", comment: null },
      { sample: "s2", characteristic: "c2", text: "2", comment: null },
      { sample: "s1", characteristic: "c3", text: "3", comment: null },
    ]);
  });

  it("refuses a block that reaches past the grid", () => {
    expect(pasteEntries([["1", "2"]], ["c1"], ["s1", "s2"], 0, 1)).toEqual({ rows: 1, columns: 2 });
    expect(pasteEntries([["1"], ["2"]], ["c1"], ["s1"], 0, 0)).toEqual({ rows: 2, columns: 1 });
  });

  it("gives the editor text and the toggled attribute", () => {
    expect(editText(undefined)).toBe("");
    expect(editText(result({ type: "variable", value: "12.10" }))).toBe("12.10");
    expect(editText(result({ type: "attribute", value: "not_ok" }))).toBe("not ok");
    expect(toggledText(undefined)).toBe("ok");
    expect(toggledText(result({ type: "attribute", value: "ok" }))).toBe("not ok");
    expect(toggledText(result({ type: "attribute", value: "not_ok" }))).toBe("ok");
  });
});
