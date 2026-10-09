import type { Autosave, BuildProfile, CommandError, OpenNotice } from "./ipc/bindings";
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
    case "new_project":
      return m.new_project();
    case "open_project":
      return m.open_project();
    case "save_project":
      return m.save_project();
    case "save_project_as":
      return m.save_project_as();
    case "undo":
      return m.undo();
    case "redo":
      return m.redo();
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
  switch (error.kind) {
    case "pdfium_unavailable":
      return m.pdfium_missing();
    case "io":
      return m.error_io({ message: error.message });
    case "invalid_document":
      return m.open_error({ message: error.message });
    case "invalid_argument":
    case "rejected":
      return m.error_rejected({ message: error.message });
    case "no_project":
      return m.error_no_project();
    case "unsaved_changes":
      return m.error_unsaved_changes();
    case "in_use":
      return error.user === null ? m.error_in_use() : m.error_in_use_by({ user: error.user });
    case "newer_version":
      return m.error_newer_version({
        found: String(error.found),
        supported: String(error.supported),
      });
    case "project":
      return m.error_project({ message: error.message });
  }
}

/** Translated autosave state (NFR-REL-01). */
export function autosaveLabel(autosave: Autosave, hasFile: boolean): string {
  switch (autosave.state) {
    case "clean":
      return hasFile ? m.autosave_clean() : m.autosave_not_saved();
    case "journaled":
      return m.autosave_journaled();
    case "failed":
      return m.autosave_failed({ message: autosave.message });
  }
}

/** Translated lines of an open notice, in a fixed order. */
export function noticeLines(notice: OpenNotice): string[] {
  const lines: string[] = [];
  if (notice.restored_unsaved) {
    lines.push(m.notice_restored_unsaved());
  }
  if (notice.recovered_changes > 0) {
    lines.push(m.notice_recovered({ count: String(notice.recovered_changes) }));
  }
  if (notice.dropped_incomplete_change) {
    lines.push(m.notice_dropped());
  }
  if (notice.set_aside_journal !== null) {
    lines.push(m.notice_set_aside({ path: notice.set_aside_journal }));
  }
  if (notice.migrated_from !== null) {
    lines.push(m.notice_migrated({ version: String(notice.migrated_from) }));
  }
  return lines;
}
