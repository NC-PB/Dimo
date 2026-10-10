import { describe, expect, it } from "vitest";
import { UNITS_PER_MM } from "$lib/viewport/balloons";
import { twoBalloons } from "$lib/viewport/balloon-fixtures";
import {
  NO_OVERRIDE,
  NUDGE_MM,
  insertedCharacteristic,
  regionBox,
  typesCharacter,
  type TypedKey,
} from "./balloon-tools.svelte";
import { addChanges, characteristic } from "./fixtures";

const VIEW = { scale: 2, tx: 0, ty: 0, rotation: 0 } as const;
const PLACEMENT = { position: { x: 300, y: 80 }, anchor: { x: 280, y: 100 } };

function key(name: string, mods: Partial<TypedKey> = {}): TypedKey {
  return { key: name, ctrlKey: false, metaKey: false, isComposing: false, ...mods };
}

/** Two balloons; Rust answers `add_characteristic` with `c` only once `release` is called. */
function slowRust() {
  const setup = twoBalloons();
  const { api } = setup;
  api.reply = { changes: addChanges("c", 3, 2) };
  let release: () => void = () => undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const execute = api.execute;
  api.execute = (command) => gate.then(() => execute(command));
  return { ...setup, release };
}

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

  it("keeps every key typed before the placement resolves and hands it to the editor (T2.7a)", async () => {
    const { api, release, selection, tools } = slowRust();
    selection.select(["a"]);
    const placing = tools.place(PLACEMENT, null);
    // `0`, `-` and `+` are zoom shortcuts on the drawing; while placing they are text.
    const kept = [..."100 +0 -0.7", "Backspace", "6", "Enter"].map((k) => tools.typeAhead(key(k)));
    expect(kept.every(Boolean)).toBe(true);
    // After Enter, keys act on the drawing again.
    expect(tools.typeAhead(key("b"))).toBe(false);
    release();
    await placing;
    expect(tools.editing).toBe("c");
    expect(tools.takeTyped("a")).toBeNull();
    expect(tools.takeTyped("c")).toEqual({ text: "100 +0 -0.6", enter: true, cancel: false });
    expect(tools.takeTyped("c")).toBeNull();
    // The new characteristic follows the selected one when numbering is locked (FR-BAL-11).
    expect(api.commands).toMatchObject([{ type: "add_characteristic", insert_after: "a" }]);
  });

  it("keeps keys typed after the answer until the editor takes them, Escape included", async () => {
    const { release, tools } = slowRust();
    const placing = tools.place(PLACEMENT, null);
    expect(tools.typeAhead(key("Ø"))).toBe(true);
    release();
    await placing;
    // The editor has not taken the focus yet.
    expect(tools.typeAhead(key("8"))).toBe(true);
    expect(tools.typeAhead(key("Escape"))).toBe(true);
    expect(tools.takeTyped("c")).toEqual({ text: "Ø8", enter: false, cancel: true });
  });

  it("lets shortcuts through when no placement is pending or the editor is gone", async () => {
    const { api, release, tools } = slowRust();
    expect(tools.typeAhead(key("0"))).toBe(false);
    const placing = tools.place(PLACEMENT, null);
    // Cmd and Ctrl combinations and keys that type nothing stay shortcuts.
    expect(tools.typeAhead(key("z", { metaKey: true }))).toBe(false);
    expect(tools.typeAhead(key("s", { ctrlKey: true }))).toBe(false);
    expect(tools.typeAhead(key("ArrowLeft"))).toBe(false);
    expect(tools.typeAhead(key("a", { isComposing: true }))).toBe(false);
    release();
    await placing;
    tools.stopEditing();
    expect(tools.typeAhead(key("1"))).toBe(false);
    expect(tools.takeTyped("c")).toBeNull();

    // A placement Rust refuses drops what was typed.
    api.reply = { changes: [] };
    const refused = tools.place(PLACEMENT, null);
    expect(tools.typeAhead(key("1"))).toBe(true);
    await refused;
    expect(tools.typeAhead(key("2"))).toBe(false);
  });

  it("sends no insert anchor without a selection", async () => {
    const { api, tools } = twoBalloons();
    api.reply = { changes: addChanges("c", 3, 2) };
    await tools.place(PLACEMENT, null);
    expect(api.commands).toMatchObject([{ type: "add_characteristic", insert_after: null }]);
  });

  it("counts single characters as typing, also with Alt", () => {
    expect(typesCharacter(key("Ø", { metaKey: false }))).toBe(true);
    expect(typesCharacter(key(" "))).toBe(true);
    expect(typesCharacter(key("Enter"))).toBe(false);
    expect(typesCharacter(key("a", { ctrlKey: true }))).toBe(false);
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
