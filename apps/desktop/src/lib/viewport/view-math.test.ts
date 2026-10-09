import { describe, expect, it } from "vitest";
import {
  MAX_SCALE,
  MIN_SCALE,
  centerOn,
  constrainPan,
  fitSheet,
  matrix,
  panBy,
  rotateView,
  rotatedBy,
  rotatedSize,
  screenToSheet,
  sheetScreenRect,
  sheetToScreen,
  snapToDevicePixels,
  storedRotation,
  viewRotation,
  visibleSheetRect,
  gestureZoomFactor,
  wheelZoomFactor,
  zoomAt,
  zoomPercent,
  type ViewRotation,
  type ViewTransform,
} from "./view-math";

/** A4 landscape in sheet units. */
const A4 = { width: 841.89, height: 595.276 };
const VIEWPORT = { width: 1200, height: 800 };

const close = (a: number, b: number) => {
  expect(Math.abs(a - b)).toBeLessThan(1e-9);
};

describe("screen and sheet space (T0.8, rule 4)", () => {
  const view: ViewTransform = { scale: 2.5, tx: -130, ty: 42, rotation: 0 };

  it("maps sheet to screen with one scale and translation", () => {
    expect(sheetToScreen(view, { x: 100, y: 10 })).toEqual({ x: 120, y: 67 });
  });

  it("screen to sheet is the inverse", () => {
    for (const p of [
      { x: 0, y: 0 },
      { x: 333.3, y: 12.5 },
      { x: -50, y: 900 },
    ]) {
      const back = sheetToScreen(view, screenToSheet(view, p));
      close(back.x, p.x);
      close(back.y, p.y);
    }
  });

  it("formats the transform as a matrix", () => {
    expect(matrix(view)).toBe("matrix(2.5,0,0,2.5,-130,42)");
  });
});

describe("zoom to cursor (FR-DOC-04)", () => {
  const start: ViewTransform = { scale: 0.8, tx: 35, ty: -12, rotation: 0 };

  it("keeps the sheet point under the cursor fixed", () => {
    const cursor = { x: 412, y: 233 };
    const before = screenToSheet(start, cursor);
    for (const factor of [1.2, 0.5, 3, 1 / 7]) {
      const after = zoomAt(start, factor, cursor);
      close(after.scale, start.scale * factor);
      const p = screenToSheet(after, cursor);
      close(p.x, before.x);
      close(p.y, before.y);
    }
  });

  it("clamps the scale and still keeps the point fixed", () => {
    const cursor = { x: 10, y: 700 };
    const before = screenToSheet(start, cursor);
    const big = zoomAt(start, 1e6, cursor);
    expect(big.scale).toBe(MAX_SCALE);
    const small = zoomAt(start, 1e-6, cursor);
    expect(small.scale).toBe(MIN_SCALE);
    for (const v of [big, small]) {
      const p = screenToSheet(v, cursor);
      close(p.x, before.x);
      close(p.y, before.y);
    }
  });

  it("zooming in and out again returns to the start", () => {
    const cursor = { x: 600, y: 400 };
    const back = zoomAt(zoomAt(start, 2, cursor), 0.5, cursor);
    close(back.scale, start.scale);
    close(back.tx, start.tx);
    close(back.ty, start.ty);
  });
});

describe("wheel zoom", () => {
  const wheel = (deltaY: number, deltaMode = 0, ctrlKey = false) =>
    wheelZoomFactor({ deltaY, deltaMode, ctrlKey }, 800);

  it("zooms in on wheel up and out on wheel down by the same factor", () => {
    const up = wheel(-100);
    const down = wheel(100);
    expect(up).toBeGreaterThan(1);
    close(up * down, 1);
  });

  it("converts lines and pages to pixels and limits one event", () => {
    close(wheel(-3, 1), wheel(-48));
    close(wheel(-1, 2), wheel(-150));
    close(wheel(-5000), wheel(-150));
  });

  it("zooms a pinch faster per pixel than a wheel and limits it separately", () => {
    expect(wheel(-5, 0, true)).toBeGreaterThan(wheel(-5));
    close(wheel(-5, 0, true) * wheel(5, 0, true), 1);
    close(wheel(-5000, 0, true), wheel(-40, 0, true));
  });

  it("makes many small trackpad events add up like one large step", () => {
    let product = 1;
    for (let i = 0; i < 20; i++) {
      product *= wheel(-2, 0, true);
    }
    close(product, wheel(-40, 0, true));
  });

  it("does nothing for a zero delta", () => {
    close(wheel(0), 1);
    close(wheel(0, 0, true), 1);
  });
});

describe("gesture zoom", () => {
  it("is the ratio of two gesture scales", () => {
    close(gestureZoomFactor(1, 1.5), 1.5);
    close(gestureZoomFactor(1.5, 1.2), 0.8);
    close(gestureZoomFactor(0, 1.2), 1);
  });
});

describe("fit and pan", () => {
  it("fits the sheet centered with padding", () => {
    const view = fitSheet(A4, VIEWPORT, 0, 16);
    close(view.scale, Math.min((1200 - 32) / A4.width, (800 - 32) / A4.height));
    const topLeft = sheetToScreen(view, { x: 0, y: 0 });
    const bottomRight = sheetToScreen(view, { x: A4.width, y: A4.height });
    close(topLeft.x, VIEWPORT.width - bottomRight.x);
    close(topLeft.y, VIEWPORT.height - bottomRight.y);
    expect(Math.min(topLeft.x, topLeft.y)).toBeGreaterThanOrEqual(16 - 1e-9);
  });

  it("returns the identity for empty sizes", () => {
    expect(fitSheet(A4, { width: 0, height: 0 })).toEqual({ scale: 1, tx: 0, ty: 0, rotation: 0 });
  });

  it("centers a sheet point", () => {
    const view = centerOn({ x: 100, y: 50 }, 2, VIEWPORT);
    expect(sheetToScreen(view, { x: 100, y: 50 })).toEqual({ x: 600, y: 400 });
  });

  it("pans by screen pixels", () => {
    expect(panBy({ scale: 2, tx: 1, ty: 2, rotation: 90 }, 10, -5)).toEqual({
      scale: 2,
      tx: 11,
      ty: -3,
      rotation: 90,
    });
  });

  it("keeps part of the sheet visible", () => {
    const view = fitSheet(A4, VIEWPORT);
    const far = constrainPan(panBy(view, 1e5, -1e5), A4, VIEWPORT, 48);
    expect(far.tx).toBe(VIEWPORT.width - 48);
    close(far.ty, 48 - A4.height * view.scale);
    // A view inside the limits is not changed.
    expect(constrainPan(view, A4, VIEWPORT)).toEqual(view);
  });

  it("reports the visible sheet area", () => {
    const rect = visibleSheetRect({ scale: 2, tx: -100, ty: 50, rotation: 0 }, VIEWPORT);
    expect(rect).toEqual({ x: 50, y: -25, width: 600, height: 400 });
  });

  it("snaps the translation to device pixels", () => {
    expect(snapToDevicePixels({ scale: 1.3, tx: 10.3, ty: -4.8, rotation: 180 }, 2)).toEqual({
      scale: 1.3,
      tx: 10.5,
      ty: -5,
      rotation: 180,
    });
  });

  it("shows zoom relative to the printed size", () => {
    expect(zoomPercent(96 / 72)).toBe(100);
    expect(zoomPercent(96 / 72 / 2)).toBe(50);
  });
});

const ROTATIONS: readonly ViewRotation[] = [0, 90, 180, 270];

describe("rotated view (T1.8, FR-DOC-05)", () => {
  const at = (rotation: ViewRotation): ViewTransform => ({
    scale: 1.7,
    tx: 210,
    ty: -35,
    rotation,
  });

  it("turns the sheet clockwise", () => {
    const origin = { scale: 1, tx: 0, ty: 0 };
    // The top left corner of the sheet moves to the top right, then bottom right, bottom left.
    expect(sheetToScreen({ ...origin, rotation: 90 }, { x: 0, y: 0 })).toEqual({ x: 0, y: 0 });
    // A point to the right of the origin ends up below it at 90 degrees clockwise.
    expect(sheetToScreen({ ...origin, rotation: 90 }, { x: 10, y: 0 })).toEqual({ x: 0, y: 10 });
    // A point below the origin ends up to its left.
    expect(sheetToScreen({ ...origin, rotation: 90 }, { x: 0, y: 10 })).toEqual({ x: -10, y: 0 });
    expect(sheetToScreen({ ...origin, rotation: 180 }, { x: 10, y: 4 })).toEqual({ x: -10, y: -4 });
    expect(sheetToScreen({ ...origin, rotation: 270 }, { x: 10, y: 0 })).toEqual({ x: 0, y: -10 });
  });

  it("maps screen to sheet and back for all four rotations", () => {
    for (const rotation of ROTATIONS) {
      const view = at(rotation);
      for (const p of [
        { x: 0, y: 0 },
        { x: 333.3, y: 12.5 },
        { x: -50, y: 900 },
      ]) {
        const there = sheetToScreen(view, screenToSheet(view, p));
        close(there.x, p.x);
        close(there.y, p.y);
        const back = screenToSheet(view, sheetToScreen(view, p));
        close(back.x, p.x);
        close(back.y, p.y);
      }
    }
  });

  it("formats a matrix that agrees with the point mapping", () => {
    for (const rotation of ROTATIONS) {
      const view = at(rotation);
      const numbers = /^matrix\((.*)\)$/.exec(matrix(view))?.[1]?.split(",").map(Number) ?? [];
      const [a, b, c, d, e, f] = numbers as [number, number, number, number, number, number];
      const p = { x: 123, y: 45 };
      const mapped = sheetToScreen(view, p);
      close(a * p.x + c * p.y + e, mapped.x);
      close(b * p.x + d * p.y + f, mapped.y);
    }
    expect(matrix({ scale: 2, tx: 0, ty: 0, rotation: 90 })).toBe("matrix(0,2,-2,0,0,0)");
    expect(matrix({ scale: 2, tx: 0, ty: 0, rotation: 0 })).toBe("matrix(2,0,0,2,0,0)");
  });

  it("swaps width and height of the sheet at 90 and 270 degrees", () => {
    expect(rotatedSize(A4, 0)).toEqual(A4);
    expect(rotatedSize(A4, 180)).toEqual(A4);
    expect(rotatedSize(A4, 90)).toEqual({ width: A4.height, height: A4.width });
    expect(rotatedSize(A4, 270)).toEqual({ width: A4.height, height: A4.width });
    for (const rotation of ROTATIONS) {
      const rect = sheetScreenRect(at(rotation), A4);
      const turned = rotatedSize(A4, rotation);
      close(rect.width, turned.width * 1.7);
      close(rect.height, turned.height * 1.7);
    }
  });

  it("fits the rotated sheet centered with padding", () => {
    for (const rotation of ROTATIONS) {
      const view = fitSheet(A4, VIEWPORT, rotation, 16);
      expect(view.rotation).toBe(rotation);
      const turned = rotatedSize(A4, rotation);
      close(view.scale, Math.min((1200 - 32) / turned.width, (800 - 32) / turned.height));
      const rect = sheetScreenRect(view, A4);
      // Centered, and inside the padding on every side.
      close(rect.x, VIEWPORT.width - (rect.x + rect.width));
      close(rect.y, VIEWPORT.height - (rect.y + rect.height));
      expect(Math.min(rect.x, rect.y)).toBeGreaterThanOrEqual(16 - 1e-9);
      // The corners of the unrotated sheet all lie inside the viewport.
      for (const corner of [
        { x: 0, y: 0 },
        { x: A4.width, y: A4.height },
        { x: A4.width, y: 0 },
        { x: 0, y: A4.height },
      ]) {
        const p = sheetToScreen(view, corner);
        expect(p.x).toBeGreaterThanOrEqual(-1e-9);
        expect(p.x).toBeLessThanOrEqual(VIEWPORT.width + 1e-9);
        expect(p.y).toBeGreaterThanOrEqual(-1e-9);
        expect(p.y).toBeLessThanOrEqual(VIEWPORT.height + 1e-9);
      }
    }
  });

  it("fits a landscape sheet larger when it is turned to portrait in a tall viewport", () => {
    const tall = { width: 600, height: 1000 };
    expect(fitSheet(A4, tall, 90).scale).toBeGreaterThan(fitSheet(A4, tall, 0).scale);
  });

  it("keeps the point under the cursor fixed while zooming, whatever the rotation", () => {
    const cursor = { x: 412, y: 233 };
    for (const rotation of ROTATIONS) {
      const start = at(rotation);
      const before = screenToSheet(start, cursor);
      for (const factor of [1.2, 0.5, 3, 1e6, 1e-6]) {
        const after = zoomAt(start, factor, cursor);
        expect(after.rotation).toBe(rotation);
        const p = screenToSheet(after, cursor);
        close(p.x, before.x);
        close(p.y, before.y);
      }
    }
  });

  it("rotating the view keeps the sheet point at the center of the viewport", () => {
    const center = { x: VIEWPORT.width / 2, y: VIEWPORT.height / 2 };
    for (const from of ROTATIONS) {
      for (const to of ROTATIONS) {
        const start = at(from);
        const turned = rotateView(start, to, VIEWPORT);
        expect(turned.rotation).toBe(to);
        expect(turned.scale).toBe(start.scale);
        const before = screenToSheet(start, center);
        const after = screenToSheet(turned, center);
        close(after.x, before.x);
        close(after.y, before.y);
      }
    }
  });

  it("centers a sheet point at any rotation", () => {
    for (const rotation of ROTATIONS) {
      const view = centerOn({ x: 100, y: 50 }, 2, VIEWPORT, rotation);
      const p = sheetToScreen(view, { x: 100, y: 50 });
      close(p.x, 600);
      close(p.y, 400);
    }
  });

  it("keeps part of the rotated sheet visible", () => {
    for (const rotation of ROTATIONS) {
      const view = fitSheet(A4, VIEWPORT, rotation);
      const rect = sheetScreenRect(view, A4);
      const right = sheetScreenRect(constrainPan(panBy(view, 1e5, 0), A4, VIEWPORT, 48), A4);
      close(right.x, VIEWPORT.width - 48);
      const left = sheetScreenRect(constrainPan(panBy(view, -1e5, 0), A4, VIEWPORT, 48), A4);
      close(left.x + left.width, 48);
      const down = sheetScreenRect(constrainPan(panBy(view, 0, 1e5), A4, VIEWPORT, 48), A4);
      close(down.y, VIEWPORT.height - 48);
      const up = sheetScreenRect(constrainPan(panBy(view, 0, -1e5), A4, VIEWPORT, 48), A4);
      close(up.y + up.height, 48);
      // A view inside the limits is not changed.
      expect(constrainPan(view, A4, VIEWPORT)).toEqual(view);
      expect(rect.width).toBeGreaterThan(0);
    }
  });

  it("reports the visible sheet area as the bounding rectangle of the viewport", () => {
    // At 90 degrees clockwise, screen right is sheet up and screen down is sheet right.
    const rect = visibleSheetRect({ scale: 2, tx: 0, ty: 0, rotation: 90 }, VIEWPORT);
    expect(rect).toEqual({ x: 0, y: -600, width: 400, height: 600 });
    for (const rotation of ROTATIONS) {
      const view = at(rotation);
      const area = visibleSheetRect(view, VIEWPORT);
      for (const corner of [
        { x: 0, y: 0 },
        { x: VIEWPORT.width, y: VIEWPORT.height },
      ]) {
        const p = screenToSheet(view, corner);
        expect(p.x).toBeGreaterThanOrEqual(area.x - 1e-9);
        expect(p.x).toBeLessThanOrEqual(area.x + area.width + 1e-9);
        expect(p.y).toBeGreaterThanOrEqual(area.y - 1e-9);
        expect(p.y).toBeLessThanOrEqual(area.y + area.height + 1e-9);
      }
    }
  });

  it("converts between stored rotations and view rotations", () => {
    for (const rotation of ROTATIONS) {
      expect(viewRotation(storedRotation(rotation))).toBe(rotation);
    }
    expect(viewRotation("deg270")).toBe(270);
    expect(rotatedBy(0, 1)).toBe(90);
    expect(rotatedBy(0, -1)).toBe(270);
    expect(rotatedBy(270, 1)).toBe(0);
    expect(rotatedBy(90, 2)).toBe(270);
    expect(rotatedBy(180, -5)).toBe(90);
  });
});
