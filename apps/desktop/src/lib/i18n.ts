import type {
  Autosave,
  BuildProfile,
  CharacteristicKind,
  Classification,
  CommandError,
  OpenNotice,
  RejectReason,
  Theme,
  Unit,
} from "./ipc/bindings";
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

/** Translated name of a theme setting (D-51). */
export function themeLabel(theme: Theme): string {
  switch (theme) {
    case "system":
      return m.theme_system();
    case "light":
      return m.theme_light();
    case "dark":
      return m.theme_dark();
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
    case "rotate_left":
      return m.rotate_left();
    case "rotate_right":
      return m.rotate_right();
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
    case "nudge_left":
      return m.nudge_left();
    case "nudge_right":
      return m.nudge_right();
    case "nudge_up":
      return m.nudge_up();
    case "nudge_down":
      return m.nudge_down();
    case "select_tool":
      return m.select_tool();
    case "place_tool":
      return m.place_tool();
    case "edit_value":
      return m.edit_value();
    case "select_all":
      return m.select_all();
    case "delete_selection":
      return m.delete_selection();
    case "restyle":
      return m.balloon_style();
    case "cancel":
      return m.cancel_action();
    case "show_shortcuts":
      return m.show_shortcuts();
    case "show_drawing":
      return m.show_drawing();
    case "show_export":
      return m.show_export();
    case "show_settings":
      return m.show_settings();
    case "table_up":
      return m.table_up();
    case "table_down":
      return m.table_down();
    case "table_left":
      return m.table_left();
    case "table_right":
      return m.table_right();
    case "table_first_column":
      return m.table_first_column();
    case "table_last_column":
      return m.table_last_column();
    case "table_first_row":
      return m.table_first_row();
    case "table_last_row":
      return m.table_last_row();
    case "table_extend_up":
      return m.table_extend_up();
    case "table_extend_down":
      return m.table_extend_down();
    case "table_select_all":
      return m.table_select_all();
    case "table_toggle":
      return m.table_toggle();
    case "table_edit":
      return m.table_edit();
    case "table_cancel":
      return m.table_cancel();
    case "table_move_up":
      return m.table_move_up();
    case "table_move_down":
      return m.table_move_down();
  }
}

/** Translated name of a characteristic kind (FR-CHR-01). */
export function kindLabel(kind: CharacteristicKind): string {
  switch (kind) {
    case "linear":
      return m.kind_linear();
    case "diameter":
      return m.kind_diameter();
    case "radius":
      return m.kind_radius();
    case "spherical_radius":
      return m.kind_spherical_radius();
    case "angle":
      return m.kind_angle();
    case "chamfer":
      return m.kind_chamfer();
    case "thread":
      return m.kind_thread();
    case "counterbore":
      return m.kind_counterbore();
    case "countersink":
      return m.kind_countersink();
    case "depth":
      return m.kind_depth();
    case "surface_texture":
      return m.kind_surface_texture();
    case "geometric":
      return m.kind_geometric();
    case "note":
      return m.kind_note();
    case "flag_note":
      return m.kind_flag_note();
    case "material_process":
      return m.kind_material_process();
    case "other":
      return m.kind_other();
  }
}

/** Translated name of a classification (FR-CHR-02, D-26). */
export function classificationLabel(classification: Classification): string {
  switch (classification) {
    case "critical":
      return m.class_critical();
    case "major":
      return m.class_major();
    case "minor":
      return m.class_minor();
    case "key":
      return m.class_key();
    case "none":
      return m.class_none();
  }
}

/** Display text of a unit; `null` is "none". */
export function unitLabel(unit: Unit | null): string {
  switch (unit) {
    case "mm":
      return m.unit_mm();
    case "in":
      return m.unit_in();
    case "deg":
      return m.unit_deg();
    case null:
      return m.unit_none();
  }
}

/**
 * Translated text for the reasons of a refused command that need one, `null` for the others
 * (the caller then shows Rust's English message).
 */
export function rejectReasonText(reason: RejectReason): string | null {
  switch (reason) {
    case "numbering_locked":
      return m.refusal_numbering_locked();
    case "zero_quantity":
      return m.refusal_zero_quantity();
    case "invalid_move_target":
      return m.refusal_invalid_move_target();
    case "unknown_characteristic":
      return m.refusal_unknown_characteristic();
    case "unknown_balloon":
    case "unknown_sheet":
    case "invalid_geometry":
    case "invalid_style":
    case "invalid_sheet_setting":
    case "invalid_project_setting":
    case "nothing_to_undo":
    case "nothing_to_redo":
    case "internal":
      return null;
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
      return m.error_rejected({ message: error.message });
    case "rejected":
      return m.error_rejected({ message: rejectReasonText(error.reason) ?? error.message });
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
    case "export":
      return m.error_export({ message: error.message });
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
