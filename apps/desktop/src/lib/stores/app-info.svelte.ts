import { commands, type AppInfo } from "$lib/ipc/bindings";

/** Facts about the running app from the `app_info` command. Read once at startup. */
export class AppInfoStore {
  current = $state<AppInfo | null>(null);

  /** Asks Rust for the app info. Stays `null` outside Tauri, for example in a plain browser. */
  async load(fetchInfo: () => Promise<AppInfo> = commands.appInfo): Promise<void> {
    try {
      this.current = await fetchInfo();
    } catch {
      this.current = null;
    }
  }
}

export const appInfo = new AppInfoStore();
