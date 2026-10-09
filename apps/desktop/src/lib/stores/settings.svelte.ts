/**
 * Settings of the user (T1.9): theme (D-51), UI language (FR-SET-04), audit user name (D-27)
 * and the export options used last. Rust stores them in the app config directory; this store
 * mirrors them and sends every change through `set_app_settings`.
 */

import {
  commands,
  type AppSettings,
  type CommandError,
  type SettingsView,
} from "$lib/ipc/bindings";
import { getLocale, isLocale, localStorageKey, setLocale } from "$lib/paraglide/runtime.js";
import { themeController } from "$lib/theme/theme";

/** Result shape of the generated command functions. */
type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  locale: null,
  user_name: "",
  export: { language: "en", pdf_balloons: "page_content" },
};

/** The commands the store needs, replaceable in tests. */
export interface SettingsApi {
  get: () => Promise<SettingsView>;
  set: (settings: AppSettings) => Promise<Result<SettingsView>>;
}

const tauriApi: SettingsApi = {
  get: () => commands.appSettings(),
  set: (settings) => commands.setAppSettings(settings),
};

export class SettingsStore {
  current = $state.raw<AppSettings>(DEFAULT_SETTINGS);
  /** The operating system user name, used for the audit while `user_name` is empty. */
  osUserName = $state("");
  /** The last failed change, shown in the settings view. */
  error = $state.raw<CommandError | null>(null);
  readonly #api: SettingsApi;

  constructor(api: SettingsApi = tauriApi) {
    this.#api = api;
  }

  /** Reads the settings from Rust. Keeps the defaults outside Tauri. */
  async load(): Promise<void> {
    try {
      this.#show(await this.#api.get());
    } catch {
      // Outside Tauri, for example in a plain browser.
    }
  }

  /** Changes some settings. Shown at once; Rust's stored values replace them on success. */
  async update(change: Partial<AppSettings>): Promise<boolean> {
    const next = { ...this.current, ...change };
    this.current = next;
    themeController.apply(next.theme);
    try {
      const result = await this.#api.set(next);
      if (result.status === "error") {
        this.error = result.error;
        return false;
      }
      this.error = null;
      this.#show(result.data);
      return true;
    } catch (e) {
      this.error = { kind: "io", message: String(e) };
      return false;
    }
  }

  /**
   * Switches the UI language and reloads the window, so every message is rendered anew.
   * `null` follows the operating system.
   */
  async setLocale(locale: string | null): Promise<void> {
    if (!(await this.update({ locale }))) {
      return;
    }
    if (locale !== null && isLocale(locale)) {
      setLocale(locale);
    } else {
      forgetLocale();
      window.location.reload();
    }
  }

  #show(view: SettingsView): void {
    this.current = view.settings;
    this.osUserName = view.os_user_name;
  }
}

/** Drops Paraglide's remembered language, so the operating system language applies. */
function forgetLocale(): void {
  try {
    localStorage.removeItem(localStorageKey);
  } catch {
    // Storage may be unavailable; the language then stays as it is.
  }
}

/**
 * Applies the stored language and theme before the app is mounted, so the first render is
 * already in the right language and colors (startup, `main.ts`).
 */
export function applyStartupSettings(settings: AppSettings): void {
  const locale = settings.locale;
  if (locale !== null && isLocale(locale)) {
    if (locale !== getLocale()) {
      setLocale(locale, { reload: false });
    }
  } else if (locale === null) {
    forgetLocale();
  }
  themeController.apply(settings.theme);
}

export const settingsStore = new SettingsStore();
