// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import { RecentStore } from "$lib/stores/recent.svelte";
import { ok, unused } from "$lib/test/fakes";
import ProjectMessages from "./ProjectMessages.svelte";
import RecentProjects from "./RecentProjects.svelte";

const components: ReturnType<typeof mount>[] = [];
let target: HTMLElement;

afterEach(() => {
  for (const c of components.splice(0)) {
    void unmount(c);
  }
  target.remove();
});

describe("project messages", () => {
  it("NFR-REL-04: opening a project of a newer Dimo shows the newer version message", async () => {
    const api: ProjectCommands = {
      projectState: () => ok({ session: 1, snapshot: null, notice: null }),
      newProject: unused,
      openProject: unused,
      saveProject: unused,
      saveProjectAs: unused,
      confirmClose: unused,
      closeProject: unused,
      openRecentProject: () =>
        Promise.resolve({
          status: "error",
          error: { kind: "newer_version", found: 99, supported: 4 },
        }),
      execute: unused,
      undo: unused,
      redo: unused,
    };
    const project = new ProjectStore(api);
    const recent = new RecentStore(
      {
        list: () =>
          Promise.resolve([
            { path: "/work/future.dimo", name: "future.dimo", folder: "/work", exists: true },
          ]),
        remove: () => Promise.resolve([]),
      },
      project,
    );
    target = document.createElement("div");
    document.body.append(target);
    components.push(mount(RecentProjects, { target, props: { store: recent, project } }));
    components.push(mount(ProjectMessages, { target, props: { projectStore: project } }));
    flushSync();
    await tick();
    await Promise.resolve();
    flushSync();

    target.querySelector<HTMLButtonElement>('li button[title="/work/future.dimo"]')?.click();
    await tick();
    await Promise.resolve();
    await Promise.resolve();
    flushSync();

    const alert = target.querySelector("[role='alert']");
    expect(alert?.querySelector("[data-testid='project-error']")?.textContent).toBe(
      "The project was written by a newer Dimo version (file format 99, this version reads up to 4). Update Dimo to open it.",
    );
  });
});
