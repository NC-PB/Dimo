// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { COMMAND_IDS } from "$lib/commands.svelte";
import { TABS, view } from "$lib/stores/view.svelte";
import FilePage from "./FilePage.svelte";
import TabBar from "./TabBar.svelte";
import TitleBar from "./TitleBar.svelte";

const mounted: ReturnType<typeof mount>[] = [];
let target: HTMLElement | null = null;

afterEach(() => {
  for (const c of mounted.splice(0)) {
    void unmount(c);
  }
  target?.remove();
  view.setTab("home");
});

describe("every command is reachable (T3.15 acceptance)", () => {
  it("has a focusable button on the title bar, a ribbon tab or File", () => {
    const host = document.createElement("div");
    target = host;
    document.body.append(host);
    mounted.push(mount(TitleBar, { target: host }), mount(TabBar, { target: host }));
    mounted.push(mount(FilePage, { target: host }));
    const found = new Set<string>();
    for (const tab of TABS) {
      view.setTab(tab);
      flushSync();
      for (const el of host.querySelectorAll<HTMLElement>("button[data-command]")) {
        expect(el.tabIndex, el.dataset.command).toBeGreaterThanOrEqual(0);
        found.add(el.dataset.command ?? "");
      }
    }
    expect([...found].sort()).toEqual([...COMMAND_IDS].sort());
  });
});
