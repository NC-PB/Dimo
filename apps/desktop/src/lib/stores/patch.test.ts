import { describe, expect, it } from "vitest";
import type { Change } from "$lib/ipc/bindings";
import { SHEET_ID, addChanges, balloon, characteristic, emptyProject } from "./fixtures";
import { PatchMismatch, applyChanges } from "./patch";

describe("applying patches (T1.5)", () => {
  it("inserts and removes characteristics and balloons at their index", () => {
    const empty = emptyProject();
    const one = applyChanges(empty, addChanges("c1", 1, 0));
    const two = applyChanges(one, addChanges("c0", 1, 0));
    expect(two.characteristics.map((c) => c.id)).toEqual(["c0", "c1"]);
    expect(two.balloons.map((b) => b.id)).toEqual(["b-c0", "b-c1"]);
    // The input is never changed.
    expect(empty.characteristics).toEqual([]);
    expect(one.characteristics).toHaveLength(1);
  });

  it("keeps unchanged items and lists by identity", () => {
    const project = applyChanges(emptyProject(), [
      ...addChanges("c1", 1, 0),
      ...addChanges("c2", 2, 1),
    ]);
    const renumbered = { ...characteristic("c2", 2), comment: "check" };
    const next = applyChanges(project, [
      { type: "characteristic_changed", before: characteristic("c2", 2), after: renumbered },
    ]);
    expect(next.characteristics[0]).toBe(project.characteristics[0]);
    expect(next.characteristics[1]).toBe(renumbered);
    expect(next.balloons).toBe(project.balloons);
    expect(next.revisions).toBe(project.revisions);
  });

  it("reorders, replaces sheets and project wide values", () => {
    const project = applyChanges(emptyProject(), [
      ...addChanges("c1", 1, 0),
      ...addChanges("c2", 2, 1),
    ]);
    const sheet = project.revisions[0]?.sheets[0];
    if (!sheet) {
      throw new Error("fixture has a sheet");
    }
    const changes: Change[] = [
      { type: "order_changed", before: ["c1", "c2"], after: ["c2", "c1"] },
      { type: "sheet_changed", before: sheet, after: { ...sheet, rotation: "deg90" } },
      {
        type: "info_changed",
        before: project.info,
        after: { ...project.info, part_number: "P-1" },
      },
      {
        type: "balloon_changed",
        before: balloon("b-c1", "c1"),
        after: { ...balloon("b-c1", "c1"), position: { x: 1, y: 2 } },
      },
    ];
    const next = applyChanges(project, changes);
    expect(next.characteristics.map((c) => c.id)).toEqual(["c2", "c1"]);
    expect(next.revisions[0]?.sheets[0]?.rotation).toBe("deg90");
    expect(next.revisions[0]?.sheets[0]?.id).toBe(SHEET_ID);
    expect(next.info.part_number).toBe("P-1");
    expect(next.balloons[0]?.position).toEqual({ x: 1, y: 2 });
  });

  it("refuses changes that do not fit", () => {
    const project = emptyProject();
    expect(() =>
      applyChanges(project, [
        { type: "characteristic_removed", index: 0, characteristic: characteristic("x", 1) },
      ]),
    ).toThrow(PatchMismatch);
    expect(() => applyChanges(project, addChanges("c1", 1, 3))).toThrow(PatchMismatch);
    expect(() =>
      applyChanges(project, [{ type: "order_changed", before: [], after: ["x"] }]),
    ).toThrow(PatchMismatch);
  });
});
