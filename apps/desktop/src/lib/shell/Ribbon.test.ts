// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { CommandId } from "$lib/commands.svelte";
import { m } from "$lib/i18n";
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

/** The ribbon button of a registry command (`CommandButton` sets `data-command`). */
function button(id: CommandId): HTMLButtonElement {
  const found = target.querySelector<HTMLButtonElement>(`button[data-command="${id}"]`);
  if (!found) {
    throw new Error(`no button ${id}`);
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

/** The ribbon groups shown in the row, by their `data-group` ID. */
function groups(): string[] {
  return [...target.querySelectorAll<HTMLElement>("[role=tabpanel] [role=group]")].map(
    (g) => g.dataset.group ?? "",
  );
}

describe("tab bar and ribbon (ADR 0007, T3.15)", () => {
  it("lists the tabs with File first and Home selected", () => {
    const tabs = [...target.querySelectorAll<HTMLElement>("[role=tab]")].map((t) => t.dataset.tab);
    expect(tabs).toEqual(["file", "home", "drawing", "review", "measure", "report", "view"]);
    expect(tab("home").getAttribute("aria-selected")).toBe("true");
    expect(tab("home").tabIndex).toBe(0);
    expect(tab("drawing").tabIndex).toBe(-1);
    expect(groups()).toEqual(["tools", "balloon", "numbering", "tolerance"]);
  });

  it("switches the ribbon and the work area with the tabs", () => {
    tab("drawing").click();
    flushSync();
    expect(view.tab).toBe("drawing");
    expect(view.current).toBe("drawing");
    expect(groups()).toEqual(["sheet", "zones", "tolerances", "project"]);
    tab("measure").click();
    flushSync();
    expect(view.current).toBe("measure");
    expect(groups()).toEqual(["parts", "values", "row", "show"]);
    tab("report").click();
    flushSync();
    expect(view.current).toBe("export");
    expect(groups()).toEqual(["document", "options", "output", "more"]);
    tab("view").click();
    flushSync();
    expect(groups()).toEqual(["zoom", "panes", "look"]);
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
    const place = button("place_tool");
    expect(place.getAttribute("aria-disabled")).toBe("true");
    expect(place.title).toContain(m.reason_no_project());
    place.click();
    flushSync();
    expect(balloonTools.tool).toBe("select");

    openProject();
    expect(place.getAttribute("aria-disabled")).toBeNull();
    expect(place.title).not.toContain(m.reason_no_project());
    expect(button("delete_selection").title).toContain(m.reason_no_selection());
  });

  it("clicking the active Place balloon button returns to Select (T3.13)", () => {
    openProject();
    button("place_tool").click();
    flushSync();
    expect(balloonTools.tool).toBe("place");
    expect(button("place_tool").getAttribute("aria-pressed")).toBe("true");
    button("place_tool").click();
    flushSync();
    expect(balloonTools.tool).toBe("select");
    expect(button("place_tool").getAttribute("aria-pressed")).toBe("false");
    expect(button("select_tool").getAttribute("aria-pressed")).toBe("true");
  });

  it("follows the tool when it changes elsewhere (Esc, shortcut)", () => {
    openProject();
    balloonTools.setTool("place");
    flushSync();
    expect(button("place_tool").getAttribute("aria-pressed")).toBe("true");
    balloonTools.setTool("select");
    flushSync();
    expect(button("select_tool").getAttribute("aria-pressed")).toBe("true");
  });

  it("shows the Review placeholder in the ribbon", () => {
    tab("review").click();
    flushSync();
    expect(target.querySelector("[role=tabpanel]")?.textContent).toContain(m.view_planned());
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
