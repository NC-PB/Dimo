import { describe, expect, it } from "vitest";
import { UNITS_PER_MM } from "$lib/viewport/balloons";
import { twoBalloons } from "$lib/viewport/balloon-fixtures";
import { NO_OVERRIDE, NUDGE_MM, insertedCharacteristic, regionBox } from "./balloon-tools.svelte";
import { addChanges, characteristic } from "./fixtures";

const VIEW = { scale: 2, tx: 0, ty: 0, rotation: 0 } as const;

describe("balloon tools (T1.6)", () => {
  it("restyles and resets the balloons of the selection as one command each", async () => {
    const { api, selection, tools } = twoBalloons();
    await tools.restyle({ shape: "flag" });
    expect(api.commands).toEqual([]);
    selection.select(["a", "b"]);
    await tools.restyle({ shape: "flag" });
    await tools.resetStyle();
    expect(api.commands).toEqual([
      { type: "restyle_balloons", ids: ["b-a", "b-b"], style: { ...NO_OVERRIDE, shape: "flag" } },
      { type: "reset_balloon_style", ids: ["b-a", "b-b"] },
    ]);
  });

  it("deletes the selected characteristics and closes the editor", async () => {
    const { api, selection, tools } = twoBalloons();
    selection.select(["b"]);
    tools.editing = "b";
    await tools.deleteSelection();
    expect(api.commands).toEqual([{ type: "delete_characteristics", ids: ["b"] }]);
    expect(tools.editing).toBeNull();
  });

  it("stores typed text only when it changed", async () => {
    const { api, tools } = twoBalloons();
    await tools.commitText("a", "  ");
    await tools.commitText("a", "Ø8 f7");
    expect(api.commands).toEqual([
      {
        type: "update_fields",
        ids: ["a"],
        values: [{ field: "requirement_text", value: "Ø8 f7" }],
      },
    ]);
  });

  it("selects all balloons of the sheet and steps back with Escape", () => {
    const { selection, tools } = twoBalloons();
    tools.setTool("place");
    tools.selectAll();
    expect([...selection.ids]).toEqual(["a", "b"]);
    tools.styleOpen = true;
    expect(tools.escape()).toBe(true);
    expect(tools.styleOpen).toBe(false);
    expect(selection.size).toBe(2);
    expect(tools.escape()).toBe(true);
    expect(selection.isEmpty).toBe(true);
    expect(tools.escape()).toBe(true);
    expect(tools.tool).toBe("select");
    expect(tools.escape()).toBe(false);
  });

  it("opens the editor of the primary selection and asks the table to scroll to it", () => {
    const { selection, tools } = twoBalloons();
    expect(tools.editPrimary()).toBe(false);
    selection.select(["a"]);
    expect(tools.editPrimary()).toBe(true);
    expect(tools.editing).toBe("a");
    expect(selection.focusRequest).toMatchObject({ id: "a", from: "viewport" });
  });

  it("nudges the selection by 1 mm in screen directions, also on a rotated sheet", async () => {
    const { api, selection, tools } = twoBalloons();
    expect(tools.nudge(1, 0, VIEW)).toBe(false);
    selection.select(["a"]);
    expect(tools.nudge(1, 0, VIEW)).toBe(true);
    // Screen right on a sheet turned 90 degrees clockwise is sheet up.
    expect(tools.nudge(1, 0, { ...VIEW, rotation: 90 })).toBe(true);
    // Commands reach Rust one after another, after the current task.
    await new Promise((resolve) => setTimeout(resolve, 0));
    const step = NUDGE_MM * UNITS_PER_MM;
    const moves = api.commands.map((c) => (c.type === "move_balloons" ? c.moves[0] : null));
    expect(moves[0]?.position.x).toBeCloseTo(100 + step, 9);
    expect(moves[0]?.position.y).toBeCloseTo(100, 9);
    expect(moves[1]?.position.x).toBeCloseTo(100, 9);
    expect(moves[1]?.position.y).toBeCloseTo(100 - step, 9);
  });

  it("finds the new characteristic in a patch and builds the source region", () => {
    expect(insertedCharacteristic({ changes: addChanges("c", 3, 2) })).toBe("c");
    expect(
      insertedCharacteristic({
        changes: [
          {
            type: "characteristic_changed",
            before: characteristic("a", 1),
            after: characteristic("a", 2),
          },
        ],
      }),
    ).toBeNull();
    expect(regionBox({ x: 10, y: 20, width: 30, height: 4 })).toEqual({
      center: { x: 25, y: 22 },
      size: { width: 30, height: 4 },
      angle: 0,
    });
  });
});
