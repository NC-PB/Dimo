import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Command, Patch, Sheet, SheetField } from "$lib/ipc/bindings";

const mocks = vi.hoisted(() => ({
  sheets: [] as Sheet[],
  executed: [] as Command[],
}));

/** Applies one field like Rust does. */
function apply(sheet: Sheet, value: SheetField): Sheet {
  switch (value.field) {
    case "rotation":
      return { ...sheet, rotation: value.value };
    case "unit":
      return { ...sheet, unit: value.value };
    case "scale_drawing":
      return { ...sheet, scale: { ...sheet.scale, drawing: value.value } };
    case "scale_actual":
      return { ...sheet, scale: { ...sheet.scale, actual: value.value } };
  }
}

// The singletons talk to Tauri; the tests give them a fake project that applies the command.
vi.mock("$lib/stores/project.svelte", () => ({
  projectStore: {
    get sheets() {
      return mocks.sheets;
    },
    execute(command: Command): Promise<Patch | undefined> {
      mocks.executed.push(command);
      if (command.type === "update_sheet") {
        mocks.sheets = mocks.sheets.map((s) =>
          s.id === command.sheet ? command.values.reduce(apply, s) : s,
        );
      }
      // Rust answers after a moment; the store shows the change when this resolves.
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve({ changes: [] });
        }, 1);
      });
    },
  },
}));
vi.mock("$lib/stores/document.svelte", () => ({ documentStore: { sheet: 0 } }));

import {
  SCALE_PRESETS,
  parseScalePart,
  presetFor,
  rotateShownSheet,
  scaleText,
  setShownScalePart,
  updateShownSheet,
} from "./sheet-properties";

function sheet(id: string, rotation: Sheet["rotation"] = "deg0"): Sheet {
  return {
    id,
    index: 0,
    size: { width: 842, height: 595 },
    rotation,
    kind: "vector_text",
    raster_dpi: null,
    unit: "mm",
    scale: { drawing: 1, actual: 1 },
    zone_grid: null,
    views: [],
  };
}

beforeEach(() => {
  mocks.sheets = [sheet("s1")];
  mocks.executed = [];
});

describe("scale input (FR-DOC-05)", () => {
  it("accepts whole numbers from 1 only", () => {
    expect(parseScalePart("1")).toBe(1);
    expect(parseScalePart(" 20 ")).toBe(20);
    expect(parseScalePart("1000000")).toBe(1_000_000);
    for (const bad of ["", "0", "-1", "1.5", "1e3", "abc", "1000001", "99999999"]) {
      expect(parseScalePart(bad)).toBeNull();
    }
  });

  it("knows presets as exact ratios", () => {
    expect(scaleText({ drawing: 2, actual: 1 })).toBe("2:1");
    expect(presetFor({ drawing: 1, actual: 2 })).toEqual({ drawing: 1, actual: 2 });
    expect(presetFor({ drawing: 3, actual: 7 })).toBeUndefined();
    expect(presetFor({ drawing: 1, actual: 1 })).toBeDefined();
    expect(new Set(SCALE_PRESETS.map(scaleText)).size).toBe(SCALE_PRESETS.length);
  });
});

describe("sheet commands (T1.8)", () => {
  it("sends only the changed values (MF F4)", async () => {
    await updateShownSheet({ unit: "in" });
    expect(mocks.executed).toEqual([
      { type: "update_sheet", sheet: "s1", values: [{ field: "unit", value: "in" }] },
    ]);
    await updateShownSheet({ scale: { drawing: 2, actual: 1 } });
    expect(mocks.executed[1]).toEqual({
      type: "update_sheet",
      sheet: "s1",
      values: [
        { field: "scale_drawing", value: 2 },
        { field: "scale_actual", value: 1 },
      ],
    });
  });

  it("rotates clockwise and counterclockwise in quarter turns", async () => {
    await rotateShownSheet(1);
    expect(mocks.sheets[0]?.rotation).toBe("deg90");
    await rotateShownSheet(-1);
    await rotateShownSheet(-1);
    expect(mocks.sheets[0]?.rotation).toBe("deg270");
    await rotateShownSheet(1);
    await rotateShownSheet(1);
    expect(mocks.sheets[0]?.rotation).toBe("deg90");
  });

  it("adds up quick presses instead of reading a stale rotation", async () => {
    await Promise.all([rotateShownSheet(1), rotateShownSheet(1), rotateShownSheet(1)]);
    expect(mocks.sheets[0]?.rotation).toBe("deg270");
    expect(mocks.executed.map((c) => (c.type === "update_sheet" ? c.values : null))).toEqual([
      [{ field: "rotation", value: "deg90" }],
      [{ field: "rotation", value: "deg180" }],
      [{ field: "rotation", value: "deg270" }],
    ]);
  });

  it("keeps both parts of a custom scale typed fast, one part per command (T3.18, MF F4)", async () => {
    await Promise.all([setShownScalePart("drawing", 3), setShownScalePart("actual", 7)]);
    expect(mocks.sheets[0]?.scale).toEqual({ drawing: 3, actual: 7 });
    expect(mocks.executed).toEqual([
      { type: "update_sheet", sheet: "s1", values: [{ field: "scale_drawing", value: 3 }] },
      { type: "update_sheet", sheet: "s1", values: [{ field: "scale_actual", value: 7 }] },
    ]);
  });

  it("does nothing without a sheet", async () => {
    mocks.sheets = [];
    await rotateShownSheet(1);
    await updateShownSheet({ unit: "in" });
    expect(mocks.executed).toEqual([]);
  });
});
