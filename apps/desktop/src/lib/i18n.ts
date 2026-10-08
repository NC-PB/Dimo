import type { BuildProfile } from "./ipc/bindings";
import { m } from "./paraglide/messages.js";
import { getLocale, locales, setLocale, type Locale } from "./paraglide/runtime.js";
import type { View } from "./stores/view.svelte";

export { getLocale, locales, m, setLocale, type Locale };

/**
 * Display names of the UI languages. Deliberately not translated: each language is named in
 * itself so users can find their language whatever the current UI language is.
 */
export const LOCALE_NAMES: Record<Locale, string> = { en: "English", de: "Deutsch" };

/** Translated label of a view. */
export function viewLabel(view: View): string {
  switch (view) {
    case "drawing":
      return m.view_drawing();
    case "review":
      return m.view_review();
    case "measure":
      return m.view_measure();
    case "export":
      return m.view_export();
    case "settings":
      return m.view_settings();
  }
}

/** Translated label of a build profile reported by `app_info`. */
export function buildProfileLabel(profile: BuildProfile): string {
  switch (profile) {
    case "debug":
      return m.build_profile_debug();
    case "release":
      return m.build_profile_release();
  }
}
