import type { BuildProfile, CommandError } from "./ipc/bindings";
import { m } from "./paraglide/messages.js";
import { getLocale, locales, setLocale, type Locale } from "./paraglide/runtime.js";
import type { ShortcutAction } from "./shortcuts";
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

/** Translated description of a shortcut action (D-52). */
export function shortcutLabel(action: ShortcutAction): string {
  switch (action) {
    case "open":
      return m.open_drawing();
    case "zoom_in":
      return m.zoom_in();
    case "zoom_out":
      return m.zoom_out();
    case "fit":
      return m.fit_sheet();
    case "pan_left":
      return m.pan_left();
    case "pan_right":
      return m.pan_right();
    case "pan_up":
      return m.pan_up();
    case "pan_down":
      return m.pan_down();
    case "previous_sheet":
      return m.previous_sheet();
    case "next_sheet":
      return m.next_sheet();
    case "show_shortcuts":
      return m.show_shortcuts();
  }
}

/** Translated message for a failed command. */
export function commandErrorMessage(error: CommandError): string {
  return error.kind === "pdfium_unavailable"
    ? m.pdfium_missing()
    : m.open_error({ message: error.message });
}
