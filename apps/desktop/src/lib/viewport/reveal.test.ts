import { describe, expect, it } from "vitest";
import { SHEET_ID, balloon } from "$lib/stores/fixtures";
import { twoBalloons } from "./balloon-fixtures";
import { BalloonGestures } from "./gestures.svelte";
import { isInside, revealPlan, viewAfterSwitch } from "./reveal";
import { fitSheet, sheetToScreen, visibleSheetRect, type ViewTransform } from "./view-math";

const OTHER_SHEET = "00000000-0000-4000-8000-0000000000ff";
const SHEETS = [{ id: SHEET_ID }, { id: OTHER_SHEET }];
const VIEWPORT = { width: 800, height: 600 };
const SHEET = { width: 800, height: 600 };

/** Balloons `b-a` (100, 100) and `b-b` (200, 100) on the first sheet, `b-c` (700, 500) on the other. */
const BALLOONS = [
  { ...balloon("b-a", "a"), position: { x: 100, y: 100 } },
  { ...balloon("b-b", "b"), position: { x: 200, y: 100 } },
  { ...balloon("b-c", "c"), sheet: OTHER_SHEET, position: { x: 700, y: 500 } },
];

describe("selection sync, viewport side (T1.6, T1.7)", () => {
  it("a click on a balloon of a rotated sheet selects its characteristic, the view stays", async () => {
    const { project, selection, tools } = twoBalloons();
    const view: ViewTransform = { scale: 1, tx: 600, ty: 0, rotation: 90 };
    const gestures = new BalloonGestures({
      project,
      selection,
      tools,
      view: () => view,
      sheet: () => SHEET,
      panBy: () => undefined,
    });
    const screen = sheetToScreen(view, { x: 200, y: 100 });
    gestures.down({
      pointerId: 1,
      button: 0,
      shiftKey: false,
      metaKey: false,
      ctrlKey: false,
      ...screen,
      timeStamp: 1000,
    });
    await gestures.up({
      pointerId: 1,
      button: 0,
      shiftKey: false,
      metaKey: false,
      ctrlKey: false,
      ...screen,
      timeStamp: 1100,
    });
    expect([...selection.ids]).toEqual(["b"]);
    // The clicked balloon is in view, so the reveal leaves the view alone.
    const plan = revealPlan(selection.ids, BALLOONS, SHEETS, 0, visibleSheetRect(view, VIEWPORT));
    expect(plan).toEqual({ kind: "none" });
  });

  it("a row on another sheet switches the sheet and names the balloon to show", () => {
    const visible = { x: 0, y: 0, width: 400, height: 300 };
    expect(revealPlan(new Set(["c"]), BALLOONS, SHEETS, 0, visible)).toEqual({
      kind: "switch",
      sheet: 1,
      center: { x: 700, y: 500 },
    });
  });

  it("centers a balloon of the shown sheet only when it is out of view", () => {
    const visible = { x: 150, y: 50, width: 400, height: 300 };
    expect(revealPlan(new Set(["a"]), BALLOONS, SHEETS, 0, visible)).toEqual({
      kind: "center",
      center: { x: 100, y: 100 },
    });
    expect(revealPlan(new Set(["b"]), BALLOONS, SHEETS, 0, visible)).toEqual({ kind: "none" });
    expect(isInside({ x: 150, y: 50 }, visible)).toBe(true);
  });

  it("leaves the view alone for an empty or group selection", () => {
    const visible = { x: 0, y: 0, width: 10, height: 10 };
    expect(revealPlan(new Set(), BALLOONS, SHEETS, 0, visible).kind).toBe("none");
    expect(revealPlan(new Set(["a", "c"]), BALLOONS, SHEETS, 0, visible).kind).toBe("none");
    expect(revealPlan(new Set(["unknown"]), BALLOONS, SHEETS, 0, visible).kind).toBe("none");
  });

  it("after the switch keeps the zoom and centers the balloon, also on a turned sheet", () => {
    const center = { x: 700, y: 500 };
    const view = viewAfterSwitch(center, 4, SHEET, VIEWPORT, 90);
    expect(view.scale).toBe(4);
    expect(view.rotation).toBe(90);
    const shown = sheetToScreen(view, center);
    expect(shown.x).toBeCloseTo(400);
    expect(shown.y).toBeCloseTo(300);
  });

  it("after the switch fits the sheet when the zoom was not larger than the fit", () => {
    const fitted = fitSheet(SHEET, VIEWPORT, 180);
    expect(viewAfterSwitch({ x: 700, y: 500 }, fitted.scale / 2, SHEET, VIEWPORT, 180)).toEqual(
      fitted,
    );
  });
});
