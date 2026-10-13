// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { characteristic, emptyProject, loaded } from "$lib/stores/fixtures";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { view } from "$lib/stores/view.svelte";
import { fitGroups } from "./ribbon-fit";
import TabBar from "./TabBar.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;
let session = 1000;

function openProject() {
  const project = emptyProject();
  project.characteristics = [characteristic("a", 1)];
  projectStore.load(loaded(++session, project));
  flushSync();
}

beforeEach(() => {
  view.setTab("home");
  balloonTools.setTool("select");
  target = document.createElement("div");
  document.body.append(target);
  component = mount(TabBar, { target });
  flushSync();
});

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  projectStore.load(loaded(++session, null));
  selection.clear();
  balloonTools.setTool("select");
  view.setTab("home");
});

function button(text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

function tab(name: string): HTMLButtonElement {
  const found = target.querySelector<HTMLButtonElement>(`[role=tab][data-tab=${name}]`);
  if (!found) {
    throw new Error(`no tab ${name}`);
  }
  return found;
}

function groups(): string[] {
  return [...target.querySelectorAll("[role=tabpanel] [role=group]")].map(
    (g) => g.getAttribute("aria-label") ?? "",
  );
}

describe("tab bar and ribbon (ADR 0007, T3.15)", () => {
  it("lists the tabs with File first and Home selected", () => {
    const tabs = [...target.querySelectorAll("[role=tab]")].map((t) => t.textContent.trim());
    expect(tabs).toEqual(["File", "Home", "Drawing", "Review", "Measure", "Report", "View"]);
    expect(tab("home").getAttribute("aria-selected")).toBe("true");
    expect(tab("home").tabIndex).toBe(0);
    expect(tab("drawing").tabIndex).toBe(-1);
    expect(groups()).toEqual(["Tools", "Balloon", "Numbering", "Tolerance"]);
  });

  it("switches the ribbon and the work area with the tabs", () => {
    tab("drawing").click();
    flushSync();
    expect(view.tab).toBe("drawing");
    expect(view.current).toBe("drawing");
    expect(groups()).toEqual(["Sheet", "Zones and views", "Tolerances", "Project"]);
    tab("measure").click();
    flushSync();
    expect(view.current).toBe("measure");
    expect(groups()).toEqual(["Parts", "Values", "Row", "Show"]);
    tab("report").click();
    flushSync();
    expect(view.current).toBe("export");
    expect(groups()).toEqual(["Document", "Options", "Output", "Other exports"]);
    tab("view").click();
    flushSync();
    expect(groups()).toEqual(["Zoom", "Panes", "Look"]);
    tab("file").click();
    flushSync();
    expect(view.current).toBe("file");
    expect(target.querySelector("[role=tabpanel]")).toBeNull();
  });

  it("moves between tabs with the arrow keys", () => {
    tab("home").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    flushSync();
    expect(view.tab).toBe("drawing");
    tab("drawing").dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
    flushSync();
    expect(view.tab).toBe("view");
    tab("view").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    flushSync();
    expect(view.tab).toBe("file");
  });

  it("keeps disabled commands visible with the reason in the tooltip", () => {
    const place = button("Place balloon");
    expect(place.getAttribute("aria-disabled")).toBe("true");
    expect(place.title).toContain("Open or start a project first.");
    place.click();
    flushSync();
    expect(balloonTools.tool).toBe("select");

    openProject();
    expect(place.getAttribute("aria-disabled")).toBeNull();
    expect(place.title).not.toContain("Open or start a project first.");
    expect(button("Delete").title).toContain("Select a balloon first.");
  });

  it("clicking the active Place balloon button returns to Select (T3.13)", () => {
    openProject();
    button("Place balloon").click();
    flushSync();
    expect(balloonTools.tool).toBe("place");
    expect(button("Place balloon").getAttribute("aria-pressed")).toBe("true");
    button("Place balloon").click();
    flushSync();
    expect(balloonTools.tool).toBe("select");
    expect(button("Place balloon").getAttribute("aria-pressed")).toBe("false");
    expect(button("Select").getAttribute("aria-pressed")).toBe("true");
  });

  it("follows the tool when it changes elsewhere (Esc, shortcut)", () => {
    openProject();
    balloonTools.setTool("place");
    flushSync();
    expect(button("Place balloon").getAttribute("aria-pressed")).toBe("true");
    balloonTools.setTool("select");
    flushSync();
    expect(button("Select").getAttribute("aria-pressed")).toBe("true");
  });

  it("shows the Review placeholder in the ribbon", () => {
    tab("review").click();
    flushSync();
    expect(target.querySelector("[role=tabpanel]")?.textContent).toContain(
      "This view comes in a later version.",
    );
  });
});

describe("ribbon overflow (T3.15)", () => {
  it("shows every group while they fit or are not measured", () => {
    expect(fitGroups([100, 200, 150], 450, 80)).toBe(3);
    expect(fitGroups([0, 0, 0], 0, 80)).toBe(3);
  });

  it("moves the groups that do not fit into More, in order", () => {
    expect(fitGroups([100, 200, 150], 449, 80)).toBe(2);
    expect(fitGroups([100, 200, 150], 300, 80)).toBe(1);
    expect(fitGroups([100, 200, 150], 100, 80)).toBe(0);
  });
});
