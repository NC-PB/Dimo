import { describe, expect, it } from "vitest";
import { VIEWS, ViewStore, isView } from "./view.svelte";

describe("view store (D-50)", () => {
  it("lists the five views in order", () => {
    expect(VIEWS).toEqual(["drawing", "review", "measure", "export", "settings"]);
  });

  it("starts on the drawing view", () => {
    expect(new ViewStore().current).toBe("drawing");
  });

  it("switches to a valid view and ignores anything else", () => {
    const view = new ViewStore();
    view.set("measure");
    expect(view.current).toBe("measure");
    view.set("");
    view.set("unknown");
    expect(view.current).toBe("measure");
    expect(isView("export")).toBe(true);
  });
});
