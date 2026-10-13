// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { RecentProject } from "$lib/ipc/bindings";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { RecentStore } from "$lib/stores/recent.svelte";
import RecentProjects from "./RecentProjects.svelte";

const ok = <T>(data: T) => Promise.resolve({ status: "ok" as const, data });

function entry(name: string, exists = true): RecentProject {
  return { path: `/work/${name}`, name, folder: "/work", exists };
}

function setup(list: RecentProject[]) {
  const calls: string[] = [];
  let items = [...list];
  const api: ProjectCommands = {
    projectState: () => ok({ session: 1, snapshot: null, notice: null }),
    newProject: () => ok(true),
    openProject: () => ok(true),
    saveProject: () => ok(true),
    saveProjectAs: () => ok(true),
    confirmClose: () => ok(null),
    closeProject: () => ok(null),
    openRecentProject: (path, discard) => {
      calls.push(`open ${path} ${String(discard)}`);
      return ok(true);
    },
    execute: () => Promise.reject(new Error("not used")),
    undo: () => Promise.reject(new Error("not used")),
    redo: () => Promise.reject(new Error("not used")),
  };
  const project = new ProjectStore(api);
  const store = new RecentStore(
    {
      list: () => {
        calls.push("list");
        return Promise.resolve(items);
      },
      remove: (path) => {
        calls.push(`remove ${path}`);
        items = items.filter((i) => i.path !== path);
        return Promise.resolve(items);
      },
    },
    project,
  );
  return { calls, project, store };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

async function render(stores: ReturnType<typeof setup>, props: Record<string, unknown> = {}) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(RecentProjects, {
    target,
    props: { store: stores.store, project: stores.project, ...props },
  });
  flushSync();
  await tick();
  await Promise.resolve();
  flushSync();
}

const rowButton = (name: string) =>
  [...target.querySelectorAll<HTMLButtonElement>("li button")].find((b) =>
    b.textContent.includes(name),
  );

describe("recent projects (T3.16)", () => {
  it("opens a project in one click", async () => {
    const stores = setup([entry("bracket.dimo"), entry("shaft.dimo")]);
    await render(stores, { heading: "Recent projects" });
    expect(target.querySelectorAll("li")).toHaveLength(2);
    expect(target.querySelector("h2")?.textContent).toBe("Recent projects");
    rowButton("shaft.dimo")?.click();
    await tick();
    await Promise.resolve();
    expect(stores.calls).toContain("open /work/shaft.dimo false");
  });

  it("shows missing files greyed, does not open them, and removes them", async () => {
    const stores = setup([entry("gone.dimo", false), entry("kept.dimo")]);
    await render(stores);
    const gone = rowButton("gone.dimo");
    expect(gone?.getAttribute("aria-disabled")).toBe("true");
    expect(gone?.textContent).toContain("File not found");
    gone?.click();
    await tick();
    expect(stores.calls.some((c) => c.startsWith("open"))).toBe(false);
    target
      .querySelector<HTMLButtonElement>('button[aria-label="Remove gone.dimo from the list"]')
      ?.click();
    await tick();
    await Promise.resolve();
    flushSync();
    expect(stores.calls).toContain("remove /work/gone.dimo");
    expect(target.querySelectorAll("li")).toHaveLength(1);
  });

  it("limits a compact list and says when it is empty", async () => {
    const many = setup(["a", "b", "c", "d"].map((n) => entry(`${n}.dimo`)));
    await render(many, { compact: true, limit: 3 });
    expect(target.querySelectorAll("li")).toHaveLength(3);
    void unmount(component ?? {});
    component = null;
    target.remove();
    await render(setup([]));
    expect(target.querySelector("[data-testid='recent-empty']")).not.toBeNull();
  });
});
