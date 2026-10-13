import { afterEach, describe, expect, it } from "vitest";
import {
  COMMAND_IDS,
  command,
  commandForShortcut,
  commandKeys,
  commandTooltip,
  disabledReason,
  runCommand,
} from "./commands.svelte";
import { locales, overwriteGetLocale } from "./paraglide/runtime.js";
import { SHORTCUTS } from "./shortcuts";
import { balloonTools } from "./stores/balloon-tools.svelte";
import { characteristic, emptyProject, loaded } from "./stores/fixtures";
import { projectStore } from "./stores/project.svelte";
import { selection } from "./stores/selection.svelte";
import { setupStore } from "./stores/setup.svelte";
import { sheetRegions } from "./stores/sheet-regions.svelte";
import { view } from "./stores/view.svelte";

let session = 100;

afterEach(() => {
  projectStore.load(loaded(++session, null));
  selection.clear();
  balloonTools.setTool("select");
  view.setTab("home");
  view.paneTool = null;
  sheetRegions.cancel();
  overwriteGetLocale(() => "en");
});

function openProject() {
  const project = emptyProject();
  project.characteristics = [characteristic("a", 1)];
  projectStore.load(loaded(++session, project));
}

describe("command registry (ADR 0007, M3 decisions 26 and 27)", () => {
  it("gives every command a label, an icon and a tooltip sentence in every language", () => {
    for (const locale of locales) {
      overwriteGetLocale(() => locale);
      for (const id of COMMAND_IDS) {
        const c = command(id);
        expect(c.label().trim(), `${id} label (${locale})`).not.toBe("");
        expect(c.description().trim(), `${id} description (${locale})`).not.toBe("");
        expect(typeof c.icon, `${id} icon`).toBe("function");
      }
    }
  });

  it("names only shortcuts of shortcuts.ts, each for one command at most", () => {
    const actions = new Set(SHORTCUTS.map((s) => s.action));
    const used = COMMAND_IDS.flatMap((id) => command(id).shortcut ?? []);
    for (const action of used) {
      expect(actions.has(action), action).toBe(true);
    }
    expect(new Set(used).size).toBe(used.length);
    expect(commandForShortcut("save_project")).toBe("save");
    expect(commandForShortcut("pan_left")).toBeUndefined();
  });

  it("shows the keys of shortcuts.ts in the tooltip", () => {
    expect(commandKeys("save", true)).toBe("⌘S");
    expect(commandKeys("save", false)).toBe("Ctrl+S");
    expect(commandKeys("renumber", true)).toBe("");
    expect(commandTooltip("place_tool", false).split("\n")[0]).toBe("Place balloon (B)");
  });

  it("keeps commands visible but disabled with a reason without a project", () => {
    expect(disabledReason("save")).toBe("Open or start a project first.");
    expect(disabledReason("place_tool")).toBe("Open or start a project first.");
    expect(disabledReason("new_project")).toBeNull();
    expect(commandTooltip("save", true).split("\n")).toEqual([
      "Save (⌘S)",
      "Saves the project in its file.",
      "Open or start a project first.",
    ]);
    expect(runCommand("place_tool")).toBe(false);
    expect(balloonTools.tool).toBe("select");
  });

  it("explains the selection and undo state once a project is open", () => {
    openProject();
    expect(disabledReason("save")).toBeNull();
    expect(disabledReason("undo")).toBe("Nothing to undo.");
    expect(disabledReason("delete_selection")).toBe("Select a balloon first.");
    expect(disabledReason("update_limits")).toBe("Select a balloon first.");
    expect(disabledReason("move_part_left")).toBe("Click a cell of a part in the grid first.");
    selection.select(["a"]);
    expect(disabledReason("delete_selection")).toBeNull();
    expect(commandTooltip("delete_selection", false).split("\n")).toHaveLength(2);
  });

  it("toggles Place balloon and shows it as pressed", () => {
    openProject();
    expect(runCommand("place_tool")).toBe(true);
    expect(balloonTools.tool).toBe("place");
    expect(command("place_tool").pressed?.()).toBe(true);
    expect(command("select_tool").pressed?.()).toBe(false);
    runCommand("place_tool");
    expect(balloonTools.tool).toBe("select");
  });

  it("zooms only while the drawing is shown", () => {
    openProject();
    expect(disabledReason("zoom_in")).toBeNull();
    view.setTab("measure");
    expect(disabledReason("zoom_in")).toBe("Only while the drawing is shown.");
  });

  it("opens and closes the cheat sheet with help", () => {
    expect(runCommand("help")).toBe(true);
    expect(view.shortcutsOpen).toBe(true);
    runCommand("help");
    expect(view.shortcutsOpen).toBe(false);
  });

  it("switches to the drawing when Place balloon is turned on from another view (T3.17)", () => {
    openProject();
    view.setTab("measure");
    expect(runCommand("place_tool")).toBe(true);
    expect(view.current).toBe("drawing");
    expect(balloonTools.tool).toBe("place");
  });

  it("opens and closes the setup pane with Set up drawing (T3.17)", () => {
    expect(disabledReason("setup_checklist")).toBe("Open or start a project first.");
    openProject();
    view.setTab("report");
    expect(setupStore.visible).toBe(false);
    expect(runCommand("setup_checklist")).toBe(true);
    expect(setupStore.visible).toBe(true);
    expect(command("setup_checklist").pressed?.()).toBe(true);
    expect(view.current).toBe("drawing");
    runCommand("setup_checklist");
    expect(setupStore.visible).toBe(false);
  });

  it("opens the zone grid and views sections of the pane and draws with them (T3.18)", () => {
    openProject();
    view.setTab("measure");
    view.showSidePanel = false;
    expect(runCommand("zone_grid")).toBe(true);
    expect(view.current).toBe("drawing");
    expect(view.showSidePanel).toBe(true);
    expect(view.paneTool).toBe("zones");
    expect(sheetRegions.tool).toBe("zone_frame");
    expect(command("zone_grid").pressed?.()).toBe(true);
    // The section stays after the frame is drawn (the tool stops by itself).
    sheetRegions.cancel();
    expect(command("zone_grid").pressed?.()).toBe(true);
    runCommand("draw_view");
    expect(view.paneTool).toBe("views");
    expect(sheetRegions.tool).toBe("view");
    runCommand("draw_view");
    expect(view.paneTool).toBeNull();
    expect(sheetRegions.tool).toBeNull();
  });

  it("opens File > Info for project info and a dialog for numbering options (T3.18)", () => {
    openProject();
    runCommand("project_info");
    expect(view.tab).toBe("file");
    expect(view.filePane).toBe("info");
    runCommand("numbering_options");
    expect(view.projectDialog).toBe("numbering");
    view.projectDialog = null;
  });
});
