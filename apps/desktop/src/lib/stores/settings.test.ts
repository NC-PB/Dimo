// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import type { AppSettings, SettingsView } from "$lib/ipc/bindings";
import { DEFAULT_SETTINGS, SettingsStore, type SettingsApi } from "./settings.svelte";

function view(settings: AppSettings): SettingsView {
  return { settings, os_user_name: "anna" };
}

describe("settings store (T1.9)", () => {
  it("keeps the defaults outside Tauri", async () => {
    const store = new SettingsStore({
      get: () => Promise.reject(new Error("no Tauri runtime")),
      set: () => Promise.reject(new Error("no Tauri runtime")),
    });
    await store.load();
    expect(store.current).toEqual(DEFAULT_SETTINGS);
  });

  it("loads the stored settings and the system user name", async () => {
    const stored: AppSettings = { ...DEFAULT_SETTINGS, theme: "dark", user_name: "Anna" };
    const store = new SettingsStore({
      get: () => Promise.resolve(view(stored)),
      set: (s) => Promise.resolve({ status: "ok", data: view(s) }),
    });
    await store.load();
    expect(store.current.theme).toBe("dark");
    expect(store.osUserName).toBe("anna");
  });

  it("sends changes to Rust and shows what Rust stored", async () => {
    const sent: AppSettings[] = [];
    const api: SettingsApi = {
      get: () => Promise.resolve(view(DEFAULT_SETTINGS)),
      set: (s) => {
        sent.push(s);
        // Rust trims the user name.
        return Promise.resolve({
          status: "ok",
          data: view({ ...s, user_name: s.user_name.trim() }),
        });
      },
    };
    const store = new SettingsStore(api);
    expect(await store.update({ user_name: "  Anna  ", theme: "light" })).toBe(true);
    expect(sent).toEqual([{ ...DEFAULT_SETTINGS, user_name: "  Anna  ", theme: "light" }]);
    expect(store.current.user_name).toBe("Anna");
    expect(store.current.theme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("keeps the change for this run and reports a failed write", async () => {
    const store = new SettingsStore({
      get: () => Promise.resolve(view(DEFAULT_SETTINGS)),
      set: () => Promise.resolve({ status: "error", error: { kind: "io", message: "read only" } }),
    });
    expect(await store.update({ theme: "dark" })).toBe(false);
    expect(store.current.theme).toBe("dark");
    expect(store.error).toEqual({ kind: "io", message: "read only" });
  });
});
