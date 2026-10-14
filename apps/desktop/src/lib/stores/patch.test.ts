import { describe, expect, it } from "vitest";
import type {
  Change,
  DrawingSource,
  ImageEdit,
  IssuedReport,
  MeasurementResult,
  Sample,
} from "$lib/ipc/bindings";
import {
  DOC_HASH,
  REVISION_ID,
  SHEET_ID,
  addChanges,
  balloon,
  characteristic,
  emptyProject,
} from "./fixtures";
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

describe("applying patches of samples, results and report data (T3.1)", () => {
  const sample = (id: string, serial: string): Sample => ({
    id,
    serial,
    date: "",
    inspector: "",
    gauges: "",
    comment: "",
  });
  const result = (s: string, c: string, value: string): MeasurementResult => ({
    sample: s,
    characteristic: c,
    value: { type: "variable", value },
    source: { type: "manual" },
    comment: "",
  });
  const set = (r: MeasurementResult): Change => ({
    type: "result_changed",
    cell: { sample: r.sample, characteristic: r.characteristic },
    before: null,
    after: r,
  });

  it("keeps results in sample order, then placement order, as Rust does", () => {
    const project = applyChanges(emptyProject(), [
      ...addChanges("c1", 1, 0),
      ...addChanges("c2", 2, 1),
      { type: "sample_inserted", index: 0, sample: sample("s1", "1") },
      { type: "sample_inserted", index: 1, sample: sample("s2", "2") },
      set(result("s2", "c2", "1.0")),
      set(result("s1", "c2", "2.0")),
      set(result("s2", "c1", "3.0")),
      set(result("s1", "c1", "4.0")),
    ]);
    const cells = (p: typeof project) => p.results.map((r) => `${r.sample}/${r.characteristic}`);
    expect(cells(project)).toEqual(["s1/c1", "s1/c2", "s2/c1", "s2/c2"]);

    const moved = applyChanges(project, [
      { type: "sample_order_changed", before: ["s1", "s2"], after: ["s2", "s1"] },
      { type: "order_changed", before: ["c1", "c2"], after: ["c2", "c1"] },
    ]);
    expect(cells(moved)).toEqual(["s2/c2", "s2/c1", "s1/c2", "s1/c1"]);

    const changed = result("s1", "c1", "4.00");
    const next = applyChanges(project, [
      {
        type: "result_changed",
        cell: { sample: "s1", characteristic: "c1" },
        before: result("s1", "c1", "4.0"),
        after: changed,
      },
      {
        type: "result_changed",
        cell: { sample: "s2", characteristic: "c2" },
        before: result("s2", "c2", "1.0"),
        after: null,
      },
      { type: "sample_changed", before: sample("s2", "2"), after: sample("s2", "SN-2") },
    ]);
    expect(next.results[0]).toBe(changed);
    expect(cells(next)).toEqual(["s1/c1", "s1/c2", "s2/c1"]);
    expect(next.samples[1]?.serial).toBe("SN-2");
    expect(() => applyChanges(next, [set(result("s1", "c1", "5"))])).toThrow(PatchMismatch);
    const removed = applyChanges(next, [
      { type: "sample_removed", index: 0, sample: sample("s1", "1") },
    ]);
    expect(removed.samples.map((s) => s.id)).toEqual(["s2"]);
  });

  it("applies report info, revision label and issued reports", () => {
    const project = emptyProject();
    const report: IssuedReport = {
      kind: "inspection_report",
      template: { id: "dimo-default", version: 1 },
      revision: REVISION_ID,
      drawing_sha256: DOC_HASH,
      exported_at: "2026-10-12T09:30:00Z",
      output_sha256: "b".repeat(64),
      format: "pdf",
    };
    const next = applyChanges(project, [
      {
        type: "report_info_changed",
        before: project.report_info,
        after: { ...project.report_info, report_number: "R-1" },
      },
      { type: "revision_label_changed", revision: REVISION_ID, before: "", after: "B" },
      { type: "issued_report_inserted", index: 0, report },
    ]);
    expect(next.report_info.report_number).toBe("R-1");
    expect(next.revisions[0]?.label).toBe("B");
    expect(next.issued_reports).toEqual([report]);
    const undone = applyChanges(next, [{ type: "issued_report_removed", index: 0, report }]);
    expect(undone.issued_reports).toEqual([]);
    expect(() =>
      applyChanges(project, [
        { type: "revision_label_changed", revision: "x", before: "", after: "B" },
      ]),
    ).toThrow(PatchMismatch);
  });

  it("replaces the source of a drawing revision (MF F1)", () => {
    const project = emptyProject();
    const before: DrawingSource = { type: "pdf" };
    const after: DrawingSource = {
      type: "image",
      format: "png",
      pages: [{ ...noEdit, clean_up: true }],
    };
    const change = { type: "revision_changed", revision: REVISION_ID, before, after } as const;
    const next = applyChanges(project, [change]);
    expect(next.revisions[0]?.source).toEqual(after);
    expect(next.revisions[0]?.sheets).toEqual(project.revisions[0]?.sheets);
    const undone = applyChanges(next, [{ ...change, before: after, after: before }]);
    expect(undone).toEqual(project);
    expect(() => applyChanges(project, [{ ...change, revision: "x" }])).toThrow(PatchMismatch);
  });
});

const noEdit: ImageEdit = {
  rotation: "deg0",
  fine_angle: "0",
  corners: null,
  trim: null,
  paper: null,
  clean_up: false,
};
