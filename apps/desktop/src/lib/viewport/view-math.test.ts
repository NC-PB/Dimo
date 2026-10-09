import { describe, expect, it } from "vitest";
import {
  MAX_SCALE,
  MIN_SCALE,
  centerOn,
  constrainPan,
  fitSheet,
  matrix,
  panBy,
  screenToSheet,
  sheetToScreen,
  snapToDevicePixels,
  visibleSheetRect,
  gestureZoomFactor,
  wheelZoomFactor,
  zoomAt,
  zoomPercent,
  type ViewTransform,
} from "./view-math";

/** A4 landscape in sheet units. */
const A4 = { width: 841.89, height: 595.276 };
const VIEWPORT = { width: 1200, height: 800 };

const close = (a: number, b: number) => {
  expect(Math.abs(a - b)).toBeLessThan(1e-9);
};

describe("screen and sheet space (T0.8, rule 4)", () => {
  const view: ViewTransform = { scale: 2.5, tx: -130, ty: 42 };

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
  const start: ViewTransform = { scale: 0.8, tx: 35, ty: -12 };

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
    const view = fitSheet(A4, VIEWPORT, 16);
    close(view.scale, Math.min((1200 - 32) / A4.width, (800 - 32) / A4.height));
    const topLeft = sheetToScreen(view, { x: 0, y: 0 });
    const bottomRight = sheetToScreen(view, { x: A4.width, y: A4.height });
    close(topLeft.x, VIEWPORT.width - bottomRight.x);
    close(topLeft.y, VIEWPORT.height - bottomRight.y);
    expect(Math.min(topLeft.x, topLeft.y)).toBeGreaterThanOrEqual(16 - 1e-9);
  });

  it("returns the identity for empty sizes", () => {
    expect(fitSheet(A4, { width: 0, height: 0 })).toEqual({ scale: 1, tx: 0, ty: 0 });
  });

  it("centers a sheet point", () => {
    const view = centerOn({ x: 100, y: 50 }, 2, VIEWPORT);
    expect(sheetToScreen(view, { x: 100, y: 50 })).toEqual({ x: 600, y: 400 });
  });

  it("pans by screen pixels", () => {
    expect(panBy({ scale: 2, tx: 1, ty: 2 }, 10, -5)).toEqual({ scale: 2, tx: 11, ty: -3 });
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
    const rect = visibleSheetRect({ scale: 2, tx: -100, ty: 50 }, VIEWPORT);
    expect(rect).toEqual({ x: 50, y: -25, width: 600, height: 400 });
  });

  it("snaps the translation to device pixels", () => {
    expect(snapToDevicePixels({ scale: 1.3, tx: 10.3, ty: -4.8 }, 2)).toEqual({
      scale: 1.3,
      tx: 10.5,
      ty: -5,
    });
  });

  it("shows zoom relative to the printed size", () => {
    expect(zoomPercent(96 / 72)).toBe(100);
    expect(zoomPercent(96 / 72 / 2)).toBe(50);
  });
});
