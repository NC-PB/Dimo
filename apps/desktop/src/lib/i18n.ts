import type {
  Autosave,
  BuildProfile,
  ChangeSource,
  CharacteristicField,
  CharacteristicKind,
  Classification,
  DerivationHint,
  DerivationRule,
  InterpreterNote,
  NumberingStrategy,
  OpenNotice,
  Theme,
  Unit,
} from "./ipc/bindings";
import { autosaveFailedText } from "./errors";
import { m } from "./paraglide/messages.js";
import { getLocale, locales, setLocale, type Locale } from "./paraglide/runtime.js";
import type { ShortcutAction } from "./shortcuts";
import type { Tab, View } from "./stores/view.svelte";

export { getLocale, locales, m, setLocale, type Locale };
export {
  autosaveFailedText,
  commandErrorMessage,
  errorDetail,
  rejectReasonText,
  tableImportText,
} from "./errors";

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
    case "file":
      return m.tab_file();
  }
}

/** Translated label of a ribbon tab (ADR 0007). */
export function tabLabel(tab: Tab): string {
  switch (tab) {
    case "file":
      return m.tab_file();
    case "home":
      return m.tab_home();
    case "drawing":
      return m.tab_drawing();
    case "review":
      return m.tab_review();
    case "measure":
      return m.tab_measure();
    case "report":
      return m.tab_report();
    case "view":
      return m.tab_view();
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
    case "show_review":
      return m.show_review();
    case "show_measure":
      return m.show_measure();
    case "show_export":
      return m.show_export();
    case "show_options":
      return m.show_options();
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
    case "grid_up":
      return m.grid_up();
    case "grid_down":
      return m.grid_down();
    case "grid_left":
      return m.grid_left();
    case "grid_right":
      return m.grid_right();
    case "grid_next_row":
      return m.grid_next_row();
    case "grid_previous_row":
      return m.grid_previous_row();
    case "grid_next_sample":
      return m.grid_next_sample();
    case "grid_previous_sample":
      return m.grid_previous_sample();
    case "grid_first_sample":
      return m.grid_first_sample();
    case "grid_last_sample":
      return m.grid_last_sample();
    case "grid_first_row":
      return m.grid_first_row();
    case "grid_last_row":
      return m.grid_last_row();
    case "grid_edit":
      return m.grid_edit();
    case "grid_clear":
      return m.grid_clear();
    case "grid_cancel":
      return m.grid_cancel();
    case "grid_ok":
      return m.grid_ok();
    case "grid_not_ok":
      return m.grid_not_ok();
    case "grid_toggle":
      return m.grid_toggle();
    case "grid_paste":
      return m.grid_paste();
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

/** Translated name of the rule that produced limits (FR-TOL-08); `null` is "no rule". */
export function ruleLabel(rule: DerivationRule | null): string {
  switch (rule?.rule) {
    case "explicit":
      return m.rule_explicit();
    case "fit":
      return m.rule_fit();
    case "drawing_rule":
      return m.rule_drawing_rule();
    case "general":
      return m.rule_general();
    case "decimal_rule":
      return m.rule_decimal_rule();
    case "custom_table":
      return m.rule_custom_table();
    case "no_tolerance_defined":
      return m.rule_no_tolerance_defined();
    case "manual":
      return m.rule_manual();
    case undefined:
      return m.rule_none();
  }
}

/**
 * Translated text of a derivation hint that the hint marker lists (T2.8). Reference and basic
 * dimensions have their own badges and return `null`.
 */
export function hintText(hint: DerivationHint): string | null {
  switch (hint.hint) {
    case "fit_deviations_differ":
      return m.hint_fit_deviations_differ({ fit: hint.fit });
    case "unknown_fit":
      return m.hint_unknown_fit({ fit: hint.fit });
    case "size_outside_table":
      return m.hint_size_outside_table({ table: hint.table.id });
    case "reference_dimension":
    case "basic_dimension":
      return null;
  }
}

/** Translated source of a change in the history (FR-CHR-10). */
export function changeSourceLabel(source: ChangeSource): string {
  switch (source) {
    case "manual":
      return m.history_source_manual();
    case "rule":
      return m.history_source_rule();
    case "recognition":
      return m.history_source_recognition();
  }
}

/** Translated name of a characteristic field in the history (FR-CHR-10). */
export function fieldLabel(field: CharacteristicField): string {
  switch (field) {
    case "number":
      return m.col_number();
    case "kind":
      return m.col_kind();
    case "requirement_text":
      return m.col_requirement_text();
    case "nominal":
      return m.col_nominal();
    case "unit":
      return m.col_unit();
    case "upper_dev":
      return m.col_upper_dev();
    case "lower_dev":
      return m.col_lower_dev();
    case "upper_limit":
      return m.col_upper_limit();
    case "lower_limit":
      return m.col_lower_limit();
    case "fit":
      return m.col_fit();
    case "derivation":
      return m.col_rule();
    case "quantity":
      return m.col_quantity();
    case "classification":
      return m.col_classification();
    case "inspection":
      return m.field_inspection();
    case "inspect":
      return m.col_inspect();
    case "status":
      return m.field_status();
    case "origin":
      return m.field_origin();
    case "sources":
      return m.field_sources();
    case "comment":
      return m.col_comment();
    case "shorter_leg":
      return m.field_shorter_leg();
    case "result_type":
      return m.field_result_type();
  }
}

/** Translated text of a tolerance engine note on a proposal (T2.6). */
export function noteText(note: InterpreterNote): string {
  switch (note) {
    case "shorter_leg_unknown":
      return m.note_shorter_leg_unknown();
    case "fit_pair":
      return m.note_fit_pair();
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

/** Translated autosave state (NFR-REL-01). */
export function autosaveLabel(autosave: Autosave, hasFile: boolean): string {
  switch (autosave.state) {
    case "clean":
      return hasFile ? m.autosave_clean() : m.autosave_not_saved();
    case "journaled":
      return m.autosave_journaled();
    case "failed":
      return autosaveFailedText(autosave.reason);
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

/** Translated name of a numbering strategy (T2.7). */
export function strategyLabel(strategy: NumberingStrategy): string {
  switch (strategy) {
    case "sheet_zone":
      return m.numbering_strategy_sheet_zone();
    case "view":
      return m.numbering_strategy_view();
    case "view_clockwise":
      return m.numbering_strategy_view_clockwise();
    case "kind":
      return m.numbering_strategy_kind();
    case "manual":
      return m.numbering_strategy_manual();
  }
}

/** One sentence on what a numbering strategy does (T2.7). */
export function strategyHint(strategy: NumberingStrategy): string {
  switch (strategy) {
    case "sheet_zone":
      return m.numbering_hint_sheet_zone();
    case "view":
      return m.numbering_hint_view();
    case "view_clockwise":
      return m.numbering_hint_view_clockwise();
    case "kind":
      return m.numbering_hint_kind();
    case "manual":
      return m.numbering_hint_manual();
  }
}
