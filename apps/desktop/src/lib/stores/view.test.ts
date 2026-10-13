import { describe, expect, it } from "vitest";
import { TABS, VIEWS, ViewStore, isTab, isView } from "./view.svelte";

describe("view store (ADR 0007)", () => {
  it("lists the tabs and work areas in order", () => {
    expect(TABS).toEqual(["file", "home", "drawing", "review", "measure", "report", "view"]);
    expect(VIEWS).toEqual(["drawing", "review", "measure", "export", "settings", "file"]);
  });

  it("starts on the Home tab with the drawing", () => {
    const view = new ViewStore();
    expect(view.tab).toBe("home");
    expect(view.current).toBe("drawing");
  });

  it("shows the work area of each tab", () => {
    const view = new ViewStore();
    const areas = TABS.map((tab) => {
      view.setTab(tab);
      return `${tab}:${view.current}`;
    });
    expect(areas).toEqual([
      "file:file",
      "home:drawing",
      "drawing:drawing",
      "review:review",
      "measure:measure",
      "report:export",
      // The View tab keeps the area shown before (here the export view of Report).
      "view:export",
    ]);
  });

  it("puts the drawing under the View tab when it opens from a page", () => {
    const view = new ViewStore();
    view.setTab("file");
    view.setTab("view");
    expect(view.current).toBe("drawing");
  });

  it("switches to a valid work area, the tab follows, anything else is ignored", () => {
    const view = new ViewStore();
    view.set("measure");
    expect([view.tab, view.current]).toEqual(["measure", "measure"]);
    view.set("");
    view.set("unknown");
    view.setTab("nonsense");
    expect(view.current).toBe("measure");
    view.set("settings");
    expect([view.tab, view.current]).toEqual(["file", "settings"]);
    view.set("export");
    expect(view.tab).toBe("report");
    expect(isView("export")).toBe(true);
    expect(isTab("report")).toBe(true);
    expect(isTab("export")).toBe(false);
  });

  it("keeps the Drawing or View tab when the drawing is asked for", () => {
    const view = new ViewStore();
    view.setTab("drawing");
    view.set("drawing");
    expect(view.tab).toBe("drawing");
    view.setTab("view");
    view.set("drawing");
    expect(view.tab).toBe("view");
    view.setTab("measure");
    view.set("drawing");
    expect(view.tab).toBe("home");
  });
});
