// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { commandForShortcut, runCommand } from "$lib/commands.svelte";
import type { AppSettings, SettingsView } from "$lib/ipc/bindings";
import { appInfo } from "$lib/stores/app-info.svelte";
import { DEFAULT_SETTINGS, SettingsStore } from "$lib/stores/settings.svelte";
import { view } from "$lib/stores/view.svelte";
import FilePage from "./FilePage.svelte";
import OptionsPane from "./OptionsPane.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement = document.createElement("div");

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  view.setTab("home");
  view.filePane = "info";
  appInfo.current = null;
});

function render(c: typeof FilePage | typeof OptionsPane, props: Record<string, unknown> = {}) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(c, { target, props });
  flushSync();
}

describe("File page (T3.16)", () => {
  it("opens Options with the shortcut of the former settings page", () => {
    expect(commandForShortcut("show_options")).toBe("options");
    view.setTab("measure");
    runCommand("options");
    expect([view.tab, view.filePane]).toEqual(["file", "options"]);
    runCommand("about");
    expect(view.filePane).toBe("about");
  });

  it("switches panes from the rail and goes back to the tab before File", () => {
    view.setTab("drawing");
    view.openFile("info");
    render(FilePage);
    expect(target.querySelector("[data-testid='file-info']")).not.toBeNull();
    target.querySelector<HTMLButtonElement>("[data-pane='open']")?.click();
    flushSync();
    expect(target.querySelector("[data-testid='file-open']")).not.toBeNull();
    expect(target.querySelector("[data-pane='open']")?.getAttribute("aria-current")).toBe("page");
    target.querySelector<HTMLButtonElement>("[data-command='options']")?.click();
    flushSync();
    expect(target.querySelector("[data-testid='options-pane']")).not.toBeNull();
    target.querySelector<HTMLButtonElement>("[data-file='back']")?.click();
    expect(view.tab).toBe("drawing");
  });

  it("shows the app version below the rail (R.11)", async () => {
    await appInfo.load(() =>
      Promise.resolve({ version: "0.1.0", build_profile: "release", pdfium_available: true }),
    );
    render(FilePage);
    expect(target.querySelector("[data-testid='file-version']")?.textContent.trim()).toBe(
      "Version 0.1.0",
    );
  });

  it("says Print is not there yet instead of hiding it", () => {
    render(FilePage);
    const print = target.querySelector("[data-command='print']");
    expect(print?.getAttribute("aria-disabled")).toBe("true");
  });

  it("without a project, Info offers New and Open", () => {
    view.openFile("info");
    render(FilePage);
    const info = target.querySelector("[data-testid='file-info']");
    expect(info?.textContent).toContain("No project is open");
    expect(info?.querySelector("[data-command='new_project']")).not.toBeNull();
    expect(info?.querySelector("[data-command='open_project']")).not.toBeNull();
  });
});

describe("Options (M3 decision 21)", () => {
  it("holds only personal settings and stores them as settings of the user", async () => {
    const sent: AppSettings[] = [];
    const store = new SettingsStore({
      get: () => Promise.resolve({ settings: DEFAULT_SETTINGS, os_user_name: "anna" }),
      set: (settings) => {
        sent.push(settings);
        const data: SettingsView = { settings, os_user_name: "anna" };
        return Promise.resolve({ status: "ok", data });
      },
    });
    render(OptionsPane, { settingsStore: store });
    const company = target.querySelector<HTMLInputElement>("#options-company");
    if (company) {
      company.value = "Muster AG";
      company.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await tick();
    expect(sent.at(-1)?.company_name).toBe("Muster AG");
    // Every field is a setting of the user; nothing that is saved in a project.
    const labels = [...target.querySelectorAll("label")].map((l) => l.textContent.trim());
    expect(labels).toEqual([
      "Theme",
      "Language",
      "User name",
      "Company name",
      "Language",
      "Check sheet",
      "Inspection report",
      "Blank result columns",
      "Balloons in the PDF",
      "Open the file after exporting",
    ]);
  });
});
