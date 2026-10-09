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

describe("document store (T0.8)", () => {
  it("is empty until a document is shown", () => {
    const store = new DocumentStore();
    expect(store.current).toBeNull();
    expect(store.sheetCount).toBe(0);
    expect(store.sheetSize).toBeNull();
  });

  it("opens through the dialog command and starts at the first sheet", async () => {
    const store = new DocumentStore();
    await store.openWithDialog(() => Promise.resolve({ status: "ok", data: INFO }));
    expect(store.current).toEqual(INFO);
    expect(store.sheet).toBe(0);
    expect(store.sheetSize).toEqual({ width: 841.89, height: 595.276 });
  });

  it("keeps the current document when the dialog is cancelled", async () => {
    const store = new DocumentStore();
    store.show(INFO);
    await store.openWithDialog(() => Promise.resolve({ status: "ok", data: null }));
    expect(store.current).toEqual(INFO);
    expect(store.error).toBeNull();
  });

  it("keeps the error of a failed open", async () => {
    const store = new DocumentStore();
    await store.openWithDialog(() =>
      Promise.resolve({ status: "error", error: { kind: "invalid_document", message: "x" } }),
    );
    expect(store.error).toEqual({ kind: "invalid_document", message: "x" });
    expect(store.busy).toBe(false);
  });

  it("switches sheets within the document", () => {
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
