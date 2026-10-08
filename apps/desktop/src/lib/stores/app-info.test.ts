import { describe, expect, it } from "vitest";
import type { AppInfo } from "$lib/ipc/bindings";
import { AppInfoStore } from "./app-info.svelte";

const INFO: AppInfo = { version: "1.2.3", build_profile: "release", pdfium_available: false };

describe("app info store (T0.3)", () => {
  it("is empty until loaded", () => {
    expect(new AppInfoStore().current).toBeNull();
  });

  it("keeps what app_info returns", async () => {
    const store = new AppInfoStore();
    await store.load(() => Promise.resolve(INFO));
    expect(store.current).toEqual(INFO);
  });

  it("stays empty when the command fails, for example outside Tauri", async () => {
    const store = new AppInfoStore();
    await store.load(() => Promise.reject(new Error("no Tauri runtime")));
    expect(store.current).toBeNull();
  });
});
