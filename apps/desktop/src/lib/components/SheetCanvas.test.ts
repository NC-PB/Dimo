// @vitest-environment happy-dom
import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ViewportStore } from "$lib/stores/viewport.svelte";
import type { ViewTransform } from "$lib/viewport/view-math";
import type { TileSource } from "./SheetCanvas.svelte";
import SheetCanvas from "./SheetCanvas.svelte";

const DOC = "d".repeat(64);
const SOURCE = { doc: DOC, page: 0, size: { width: 841.89, height: 595.276 } };

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  vi.restoreAllMocks();
});

/** Mounts a canvas of 800 by 600 px; `overlay` is the overlay snippet, if any. */
function render(props: { source: TileSource | null } & Record<string, unknown>) {
  // The parts of Tauri the canvas uses: the tile URL base and a tile interest call that fails.
  Object.assign(window, {
    __TAURI_INTERNALS__: {
      convertFileSrc: (path: string, scheme: string) => `${scheme}://localhost/${path}`,
      invoke: () => Promise.reject(new Error("outside Tauri")),
    },
  });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(600);
  target = document.createElement("div");
  document.body.append(target);
  const store = new ViewportStore();
  component = mount(SheetCanvas, {
    target,
    props: { store, label: "Sheet", ...props },
  });
  flushSync();
  // happy-dom does no layout: give the canvas the size the browser would measure.
  store.size = { width: 800, height: 600 };
  flushSync();
  return store;
}

describe("sheet canvas (MF F8)", () => {
  it("renders a sheet with tiles and no balloon layer", async () => {
    const store = render({ source: SOURCE });
    // Outside Tauri the tile interest call fails; the canvas shows the tiles anyway.
    await vi.waitFor(() => {
      expect(target.querySelectorAll("img.tile").length).toBeGreaterThan(0);
    });
    const canvas = target.querySelector("[data-sheet-canvas]");
    expect(canvas?.getAttribute("aria-label")).toBe("Sheet");
    expect(target.querySelector(".sheet")).not.toBeNull();
    const urls = [...target.querySelectorAll("img.tile")].map((img) => img.getAttribute("src"));
    expect(urls.every((url) => url?.includes(`/tile/${DOC}/0/`))).toBe(true);
    // The canvas draws only the sheet: balloons, editor and gestures belong to the Viewport.
    expect(target.querySelector("svg")).toBeNull();
    expect(target.querySelector("[role=application]")).toBeNull();
    // The first show fits the sheet into the canvas.
    expect(store.sheet).toEqual(SOURCE.size);
    expect(store.view.scale).toBeGreaterThan(0);
    expect(store.view.scale).toBeLessThan(1);
  });

  it("draws the overlay with the view of the tiles and leaves the fit to onFirstShow", async () => {
    let seen: ViewTransform | null = null;
    const overlay = createRawSnippet((view: () => ViewTransform) => ({
      render: () => `<div data-overlay></div>`,
      setup: () => {
        seen = view();
      },
    }));
    const store = render({ source: SOURCE, overlay, onFirstShow: () => true });
    await vi.waitFor(() => {
      expect(target.querySelector("[data-overlay]")).not.toBeNull();
    });
    expect(seen).toEqual(store.view);
    // onFirstShow set the view itself, so the canvas did not fit.
    expect(store.view.scale).toBe(1);
  });

  it("shows the empty snippet without a page and requests no tiles", () => {
    const empty = createRawSnippet(() => ({ render: () => `<p data-empty>none</p>` }));
    render({ source: null, empty });
    expect(target.querySelector("[data-empty]")).not.toBeNull();
    expect(target.querySelectorAll("img.tile")).toHaveLength(0);
  });

  it("zooms with the wheel around the cursor", async () => {
    const store = render({ source: SOURCE });
    await vi.waitFor(() => {
      expect(store.view.scale).toBeLessThan(1);
    });
    const before = store.view.scale;
    target
      .querySelector("[data-sheet-canvas]")
      ?.dispatchEvent(
        new WheelEvent("wheel", { deltaY: -100, clientX: 400, clientY: 300, cancelable: true }),
      );
    expect(store.view.scale).toBeGreaterThan(before);
  });
});
