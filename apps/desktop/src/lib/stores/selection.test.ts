import { describe, expect, it } from "vitest";
import { SelectionStore } from "./selection.svelte";

describe("selection store (FR-BAL-12)", () => {
  it("starts empty", () => {
    const s = new SelectionStore();
    expect(s.isEmpty).toBe(true);
    expect(s.primary).toBeNull();
    expect(s.focusRequest).toBeNull();
  });

  it("replaces the selection and takes the last ID as primary", () => {
    const s = new SelectionStore();
    s.select(["a", "b"]);
    expect([...s.ids]).toEqual(["a", "b"]);
    expect(s.primary).toBe("b");
    s.select(["c"], null);
    expect([...s.ids]).toEqual(["c"]);
    expect(s.primary).toBeNull();
  });

  it("adds, removes and toggles", () => {
    const s = new SelectionStore();
    s.add(["a"]);
    s.add(["b", "a"]);
    expect([...s.ids]).toEqual(["a", "b"]);
    expect(s.primary).toBe("a");
    s.toggle("a");
    expect([...s.ids]).toEqual(["b"]);
    expect(s.primary).toBe("b");
    s.toggle("c");
    expect(s.has("c")).toBe(true);
    expect(s.primary).toBe("c");
    s.remove(["b", "c", "x"]);
    expect(s.isEmpty).toBe(true);
    expect(s.primary).toBeNull();
  });

  it("replaces the set object on change and keeps it otherwise", () => {
    const s = new SelectionStore();
    const empty = s.ids;
    s.remove(["a"]);
    expect(s.ids).toBe(empty);
    s.add(["a"]);
    expect(s.ids).not.toBe(empty);
  });

  it("focus selects one ID and raises a new request every time", () => {
    const s = new SelectionStore();
    s.select(["a", "b"]);
    s.focus("b", "table");
    expect([...s.ids]).toEqual(["b"]);
    const first = s.focusRequest;
    s.focus("b", "viewport");
    expect(s.focusRequest?.id).toBe("b");
    expect(s.focusRequest?.from).toBe("viewport");
    expect(s.focusRequest?.seq).toBeGreaterThan(first?.seq ?? 0);
  });

  it("retain drops deleted IDs and a stale focus request", () => {
    const s = new SelectionStore();
    s.select(["a", "b", "c"]);
    s.focus("c", "viewport");
    s.add(["a"]);
    s.retain((id) => id === "a");
    expect([...s.ids]).toEqual(["a"]);
    expect(s.primary).toBe("a");
    expect(s.focusRequest).toBeNull();
  });
});
