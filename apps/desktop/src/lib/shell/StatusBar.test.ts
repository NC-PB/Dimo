// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import { characteristic, emptyProject, loaded } from "$lib/stores/fixtures";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { view } from "$lib/stores/view.svelte";
import PlacingBar from "./PlacingBar.svelte";
import StatusBar from "./StatusBar.svelte";
import TitleBar from "./TitleBar.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;
let session = 2000;

function openProject() {
  const project = emptyProject();
  project.characteristics = [characteristic("a", 1), characteristic("b", 2)];
  const state = loaded(++session, project);
  if (state.snapshot) {
    state.snapshot.drawing.sheets = [
      { width: 841.89, height: 595.276 },
      { width: 841.89, height: 595.276 },
    ];
  }
  projectStore.load(state);
  flushSync();
}

beforeEach(() => {
  target = document.createElement("div");
  document.body.append(target);
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

describe("status bar (ADR 0007, T3.15)", () => {
  beforeEach(() => {
    component = mount(StatusBar, { target });
    flushSync();
  });

  it("shows only the zoom, disabled, without a project", () => {
    expect(target.querySelector("[data-testid=tool-chip]")).toBeNull();
    expect(target.querySelector<HTMLInputElement>("input[type=range]")?.disabled).toBe(true);
  });

  it("shows sheet tabs, the tool, the counts and the general tolerance", () => {
    openProject();
    const tabs = [...target.querySelectorAll<HTMLButtonElement>("[data-sheet]")];
    expect(tabs.map((t) => t.textContent.trim())).toEqual(["Sheet 1", "Sheet 2"]);
    expect(tabs[0]?.getAttribute("aria-current")).toBe("page");
    tabs[1]?.click();
    flushSync();
    expect(documentStore.sheet).toBe(1);

    const chip = target.querySelector<HTMLElement>("[data-testid=tool-chip]");
    expect(chip?.textContent.trim()).toBe("Select");
    balloonTools.setTool("place");
    flushSync();
    expect(chip?.textContent.trim()).toBe("Place balloon");

    expect(target.textContent).toContain("2 characteristics");
    selection.select(["a"]);
    flushSync();
    expect(target.textContent).toContain("1 selected");
    expect(target.textContent).toContain("General tolerance: None");
  });
});

describe("title bar (ADR 0007)", () => {
  it("names the project and its saved state", () => {
    component = mount(TitleBar, { target });
    flushSync();
    const name = () => target.querySelector("[data-testid=document-name]")?.textContent ?? "";
    expect(name()).toContain("No project open");
    openProject(); // A new project without changes and without a file.
    expect(name()).toContain("Untitled");
    expect(name()).toContain("not saved yet");
    expect(target.querySelector("button[aria-label=Save]")).not.toBeNull();
    expect(target.querySelector("button[aria-label=Undo]")?.getAttribute("aria-disabled")).toBe(
      "true",
    );
  });
});

describe("placing bar (M3 decision 19)", () => {
  beforeEach(() => {
    component = mount(PlacingBar, { target });
    flushSync();
  });

  it("shows while Place balloon is on and stops it", () => {
    expect(target.querySelector("[data-testid=placing-bar]")).toBeNull();
    balloonTools.setTool("place");
    flushSync();
    const bar = target.querySelector("[data-testid=placing-bar]");
    expect(bar?.textContent).toContain("Placing balloons.");
    expect(bar?.textContent).toContain("Click a dimension, or drag a box around it.");
    const stop = [...target.querySelectorAll("button")].find(
      (b) => b.textContent.trim() === "Stop (Esc)",
    );
    stop?.click();
    flushSync();
    expect(balloonTools.tool).toBe("select");
    expect(target.querySelector("[data-testid=placing-bar]")).toBeNull();
  });
});
