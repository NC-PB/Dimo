/**
 * Recent projects of the user (T3.16, ADR 0007): the list Rust keeps in the app config directory
 * next to the settings. Rust adds an entry after every open and "save as"; this store mirrors the
 * list, removes entries and opens one through the project store, which asks about unsaved changes
 * first. Rust opens only paths that are on its list.
 */

import { commands, type RecentProject } from "$lib/ipc/bindings";
import { projectStore as appProject, type ProjectStore } from "./project.svelte";

/** The commands the store needs, replaceable in tests. */
export interface RecentApi {
  list: () => Promise<RecentProject[]>;
  remove: (path: string) => Promise<RecentProject[]>;
}

const tauriApi: RecentApi = {
  list: () => commands.recentProjects(),
  remove: (path) => commands.removeRecentProject(path),
};

export class RecentStore {
  items = $state.raw<readonly RecentProject[]>([]);
  readonly #api: RecentApi;
  readonly #project: ProjectStore;

  constructor(api: RecentApi = tauriApi, project: ProjectStore = appProject) {
    this.#api = api;
    this.#project = project;
  }

  /** Reads the list from Rust. Keeps it empty outside Tauri. */
  async load(): Promise<void> {
    try {
      this.items = await this.#api.list();
    } catch {
      // Outside Tauri, for example in a plain browser.
    }
  }

  /** Removes an entry from the list; the file is not touched. */
  async remove(path: string): Promise<void> {
    try {
      this.items = await this.#api.remove(path);
    } catch {
      // Outside Tauri.
    }
  }

  /** Opens a recent project, then reads the list again (it moved to the top, or is gone). */
  async open(path: string): Promise<void> {
    await this.#project.openRecent(path);
    await this.load();
  }
}

export const recentStore = new RecentStore();
