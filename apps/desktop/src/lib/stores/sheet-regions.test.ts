import { describe, expect, it } from "vitest";
import type { Command, Patch, Project, Sheet } from "$lib/ipc/bindings";
import { emptyProject } from "./fixtures";
import { SheetRegionsStore, type RegionProject } from "./sheet-regions.svelte";

function fakeProject(edit: (sheet: Sheet) => void = () => undefined) {
  const project: Project = emptyProject();
  const sheet = project.revisions[0]?.sheets[0];
  if (sheet) {
    edit(sheet);
  }
  const commands: Command[] = [];
  const state: RegionProject = {
    project,
    sheets: project.revisions[0]?.sheets ?? [],
    execute: (command: Command): Promise<Patch | undefined> => {
      commands.push(command);
      return Promise.resolve({ changes: [] });
    },
  };
  return { state, commands, sheet };
}

describe("rectangle tools of the sheet properties (T2.7, M2 decision 1)", () => {
  it("draws a zone frame with the default labels when the sheet has no grid", async () => {
    const rust = fakeProject();
    const store = new SheetRegionsStore(rust.state, () => 0);
    store.toggle("zone_frame");
    store.begin({ x: 400, y: 300 });
    store.moveTo({ x: 200, y: 250 });
    expect(store.drag).toEqual({ from: { x: 400, y: 300 }, to: { x: 200, y: 250 } });
    await store.finish({ x: 20, y: 10 });
    expect(rust.commands).toEqual([
      {
        type: "set_zone_grid",
        sheet: rust.sheet?.id,
        grid: {
          frame: { origin: { x: 20, y: 10 }, size: { width: 380, height: 290 } },
          column_labels: ["1", "2", "3", "4", "5", "6", "7", "8"],
          row_labels: ["A", "B", "C", "D", "E", "F"],
        },
      },
    ]);
    expect(store.tool).toBeNull();
    expect(store.drag).toBeNull();
  });

  it("keeps the labels of an existing grid when the frame is drawn again", async () => {
    const rust = fakeProject((s) => {
      s.zone_grid = {
        frame: { origin: { x: 0, y: 0 }, size: { width: 10, height: 10 } },
        column_labels: ["4", "3", "2", "1"],
        row_labels: ["A", "B"],
      };
    });
    const store = new SheetRegionsStore(rust.state, () => 0);
    store.toggle("zone_frame");
    store.begin({ x: 10, y: 10 });
    await store.finish({ x: 110, y: 60 });
    const command = rust.commands[0];
    expect(command?.type === "set_zone_grid" && command.grid?.column_labels).toEqual([
      "4",
      "3",
      "2",
      "1",
    ]);
  });

  it("adds a view after the others", async () => {
    const rust = fakeProject((s) => {
      s.views = [
        { label: "A-A", rect: { origin: { x: 0, y: 0 }, size: { width: 50, height: 50 } } },
      ];
    });
    const store = new SheetRegionsStore(rust.state, () => 0);
    store.toggle("view");
    store.begin({ x: 100, y: 100 });
    await store.finish({ x: 300, y: 200 });
    expect(rust.commands).toEqual([
      {
        type: "set_views",
        sheet: rust.sheet?.id,
        views: [
          { label: "A-A", rect: { origin: { x: 0, y: 0 }, size: { width: 50, height: 50 } } },
          { label: "", rect: { origin: { x: 100, y: 100 }, size: { width: 200, height: 100 } } },
        ],
      },
    ]);
  });

  it("ignores a click without a rectangle, and stops on cancel or a second toggle", async () => {
    const rust = fakeProject();
    const store = new SheetRegionsStore(rust.state, () => 0);
    store.toggle("view");
    store.begin({ x: 100, y: 100 });
    await store.finish({ x: 102, y: 103 });
    expect(rust.commands).toEqual([]);
    store.toggle("view");
    expect(store.tool).toBe("view");
    store.toggle("view");
    expect(store.tool).toBeNull();
    store.toggle("zone_frame");
    store.cancel();
    expect(store.tool).toBeNull();
    // Without a tool a press draws nothing.
    store.begin({ x: 1, y: 1 });
    expect(store.drag).toBeNull();
  });
});
