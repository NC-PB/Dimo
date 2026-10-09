import { describe, expect, it } from "vitest";
import type { DocumentInfo } from "$lib/ipc/bindings";
import { DocumentStore } from "./document.svelte";

const INFO: DocumentInfo = {
  doc: "a".repeat(64),
  name: "part.pdf",
  sheets: [
    { width: 841.89, height: 595.276 },
    { width: 1190.55, height: 841.89 },
  ],
};

describe("document store (T0.8, T1.5)", () => {
  it("is empty until a drawing is shown", () => {
    const store = new DocumentStore();
    expect(store.current).toBeNull();
    expect(store.sheetCount).toBe(0);
    expect(store.sheetSize).toBeNull();
  });

  it("starts a shown drawing at its first sheet", () => {
    const store = new DocumentStore();
    store.show(INFO);
    store.nextSheet();
    store.show(INFO);
    expect(store.sheet).toBe(0);
    expect(store.sheetSize).toEqual({ width: 841.89, height: 595.276 });
    store.show(null);
    expect(store.current).toBeNull();
  });

  it("switches sheets within the drawing", () => {
    const store = new DocumentStore();
    store.show(INFO);
    store.nextSheet();
    expect(store.sheet).toBe(1);
    store.nextSheet();
    expect(store.sheet).toBe(1);
    store.previousSheet();
    store.previousSheet();
    expect(store.sheet).toBe(0);
    store.setSheet(1.5);
    expect(store.sheet).toBe(0);
  });
});
