/**
 * The keyboard shortcut map (D-52, NFR-UX-01). Every key combination of the app is defined here
 * and listed in the cheat sheet (`?`). Labels come from `shortcutLabel` in `i18n.ts`. Commands of
 * the registry (`commands.svelte.ts`) name the action they run, so their tooltips show these
 * keys and the window runs the command for them (T3.15); keys are never repeated there.
 */

export type ShortcutAction =
  | "new_project"
  | "open_project"
  | "save_project"
  | "save_project_as"
  | "undo"
  | "redo"
  | "zoom_in"
  | "zoom_out"
  | "fit"
  | "rotate_left"
  | "rotate_right"
  | "pan_left"
  | "pan_right"
  | "pan_up"
  | "pan_down"
  | "previous_sheet"
  | "next_sheet"
  | "nudge_left"
  | "nudge_right"
  | "nudge_up"
  | "nudge_down"
  | "select_tool"
  | "place_tool"
  | "edit_value"
  | "select_all"
  | "delete_selection"
  | "restyle"
  | "cancel"
  | "show_shortcuts"
  | "show_drawing"
  | "show_review"
  | "show_measure"
  | "show_export"
  | "show_options"
  | "table_up"
  | "table_down"
  | "table_left"
  | "table_right"
  | "table_first_column"
  | "table_last_column"
  | "table_first_row"
  | "table_last_row"
  | "table_extend_up"
  | "table_extend_down"
  | "table_select_all"
  | "table_toggle"
  | "table_edit"
  | "table_cancel"
  | "table_move_up"
  | "table_move_down"
  | "grid_up"
  | "grid_down"
  | "grid_left"
  | "grid_right"
  | "grid_next_row"
  | "grid_previous_row"
  | "grid_next_sample"
  | "grid_previous_sample"
  | "grid_first_sample"
  | "grid_last_sample"
  | "grid_first_row"
  | "grid_last_row"
  | "grid_edit"
  | "grid_clear"
  | "grid_cancel"
  | "grid_ok"
  | "grid_not_ok"
  | "grid_toggle"
  | "grid_paste";

/**
 * Where a shortcut acts (MF F8). An area of the window registers its scope while it is shown or
 * focused (`stores/keyboard.svelte.ts`); the window handler (`Shortcuts.svelte`) then runs the
 * shortcuts of the active scopes:
 *
 * - `app`: in every area (project, tabs, undo, the cheat sheet).
 * - `drawing`: while the drawing is shown.
 * - `table`, `grid`: while the characteristic table (T1.7) or the measurement grid (T3.5) has
 *   the focus. These areas handle their keys themselves; the window then runs only `app` keys.
 * - `imageEditor`, `review`: reserved for the image editor (R.20) and the review view.
 *
 * No shortcut acts while a dialog is open (`AppDialog` counts them).
 */
export type ShortcutScope = "app" | "drawing" | "table" | "grid" | "imageEditor" | "review";

/** Scopes whose area handles its own keys while it has the focus. */
export const FOCUS_SCOPES: readonly ShortcutScope[] = ["table", "grid"];

/**
 * One key combination. `key` is compared with `KeyboardEvent.key` (letters case insensitive).
 * `mod` is Cmd on macOS and Ctrl elsewhere. Shift is compared only when `shift` is given,
 * because symbols like `+` and `?` need Shift on some keyboard layouts and not on others.
 * `alt` is Option on macOS and Alt elsewhere; it must match exactly.
 */
export interface KeyCombo {
  key: string;
  mod?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export interface Shortcut {
  action: ShortcutAction;
  /** The first combination is the one shown in the UI. */
  keys: readonly KeyCombo[];
  /** Where the shortcut acts. */
  scope: ShortcutScope;
  /**
   * Acts only while its area itself or nothing has the focus. Arrow keys, Enter and Delete
   * belong to a focused button, list or pane otherwise.
   */
  focusOnly?: boolean;
  /** In a text field the key keeps its text editing meaning (undo typing, not the project). */
  textEditing?: boolean;
}

export const SHORTCUTS: readonly Shortcut[] = [
  { action: "new_project", keys: [{ key: "n", mod: true }], scope: "app" },
  { action: "open_project", keys: [{ key: "o", mod: true }], scope: "app" },
  { action: "save_project", keys: [{ key: "s", mod: true, shift: false }], scope: "app" },
  { action: "save_project_as", keys: [{ key: "s", mod: true, shift: true }], scope: "app" },
  {
    action: "undo",
    keys: [{ key: "z", mod: true, shift: false }],
    textEditing: true,
    scope: "app",
  },
  {
    action: "redo",
    keys: [
      { key: "z", mod: true, shift: true },
      { key: "y", mod: true, shift: false },
    ],
    textEditing: true,
    scope: "app",
  },
  // Tabs (ADR 0007): drawing (Home), review, measure, report, and File > Options.
  { action: "show_drawing", keys: [{ key: "1", mod: true }], scope: "app" },
  { action: "show_review", keys: [{ key: "2", mod: true }], scope: "app" },
  { action: "show_measure", keys: [{ key: "3", mod: true }], scope: "app" },
  { action: "show_export", keys: [{ key: "e", mod: true }], scope: "app" },
  { action: "show_options", keys: [{ key: ",", mod: true }], scope: "app" },
  { action: "zoom_in", keys: [{ key: "+" }, { key: "=" }], scope: "drawing" },
  { action: "zoom_out", keys: [{ key: "-" }, { key: "_" }], scope: "drawing" },
  { action: "fit", keys: [{ key: "0" }], scope: "drawing" },
  // Shift tells the direction, so the keys work on every keyboard layout (FR-DOC-05).
  { action: "rotate_right", keys: [{ key: "r", shift: false }], scope: "drawing" },
  { action: "rotate_left", keys: [{ key: "r", shift: true }], scope: "drawing" },
  {
    action: "pan_left",
    keys: [{ key: "ArrowLeft", shift: false }],
    scope: "drawing",
    focusOnly: true,
  },
  {
    action: "pan_right",
    keys: [{ key: "ArrowRight", shift: false }],
    scope: "drawing",
    focusOnly: true,
  },
  { action: "pan_up", keys: [{ key: "ArrowUp", shift: false }], scope: "drawing", focusOnly: true },
  {
    action: "pan_down",
    keys: [{ key: "ArrowDown", shift: false }],
    scope: "drawing",
    focusOnly: true,
  },
  { action: "previous_sheet", keys: [{ key: "PageUp" }], scope: "drawing" },
  { action: "next_sheet", keys: [{ key: "PageDown" }], scope: "drawing" },
  {
    action: "nudge_left",
    keys: [{ key: "ArrowLeft", shift: true }],
    scope: "drawing",
    focusOnly: true,
  },
  {
    action: "nudge_right",
    keys: [{ key: "ArrowRight", shift: true }],
    scope: "drawing",
    focusOnly: true,
  },
  {
    action: "nudge_up",
    keys: [{ key: "ArrowUp", shift: true }],
    scope: "drawing",
    focusOnly: true,
  },
  {
    action: "nudge_down",
    keys: [{ key: "ArrowDown", shift: true }],
    scope: "drawing",
    focusOnly: true,
  },
  { action: "select_tool", keys: [{ key: "v" }], scope: "drawing" },
  { action: "place_tool", keys: [{ key: "b" }], scope: "drawing" },
  { action: "edit_value", keys: [{ key: "Enter" }], scope: "drawing", focusOnly: true },
  { action: "select_all", keys: [{ key: "a", mod: true }], scope: "drawing", textEditing: true },
  {
    action: "delete_selection",
    keys: [{ key: "Delete" }, { key: "Backspace" }],
    scope: "drawing",
    focusOnly: true,
  },
  { action: "restyle", keys: [{ key: "s", shift: false }], scope: "drawing" },
  { action: "cancel", keys: [{ key: "Escape" }], scope: "drawing" },
  { action: "show_shortcuts", keys: [{ key: "?" }], scope: "app" },
  // Characteristic table (T1.7), active while the table has focus.
  { action: "table_up", keys: [{ key: "ArrowUp", shift: false }], scope: "table" },
  { action: "table_down", keys: [{ key: "ArrowDown", shift: false }], scope: "table" },
  { action: "table_left", keys: [{ key: "ArrowLeft" }], scope: "table" },
  { action: "table_right", keys: [{ key: "ArrowRight" }], scope: "table" },
  { action: "table_first_column", keys: [{ key: "Home" }], scope: "table" },
  { action: "table_last_column", keys: [{ key: "End" }], scope: "table" },
  {
    action: "table_first_row",
    keys: [
      { key: "ArrowUp", mod: true },
      { key: "Home", mod: true },
    ],
    scope: "table",
  },
  {
    action: "table_last_row",
    keys: [
      { key: "ArrowDown", mod: true },
      { key: "End", mod: true },
    ],
    scope: "table",
  },
  { action: "table_extend_up", keys: [{ key: "ArrowUp", shift: true }], scope: "table" },
  { action: "table_extend_down", keys: [{ key: "ArrowDown", shift: true }], scope: "table" },
  { action: "table_select_all", keys: [{ key: "a", mod: true }], scope: "table" },
  { action: "table_toggle", keys: [{ key: " " }], scope: "table" },
  { action: "table_edit", keys: [{ key: "Enter" }, { key: "F2" }], scope: "table" },
  { action: "table_cancel", keys: [{ key: "Escape" }], scope: "table" },
  { action: "table_move_up", keys: [{ key: "ArrowUp", alt: true }], scope: "table" },
  { action: "table_move_down", keys: [{ key: "ArrowDown", alt: true }], scope: "table" },
  // Measurement grid (T3.5, M3 decision 7), active while the grid has focus. Enter, Tab and
  // their Shift variants also confirm a value being typed.
  { action: "grid_up", keys: [{ key: "ArrowUp" }], scope: "grid" },
  { action: "grid_down", keys: [{ key: "ArrowDown" }], scope: "grid" },
  { action: "grid_left", keys: [{ key: "ArrowLeft" }], scope: "grid" },
  { action: "grid_right", keys: [{ key: "ArrowRight" }], scope: "grid" },
  { action: "grid_next_row", keys: [{ key: "Enter", shift: false }], scope: "grid" },
  { action: "grid_previous_row", keys: [{ key: "Enter", shift: true }], scope: "grid" },
  { action: "grid_next_sample", keys: [{ key: "Tab", shift: false }], scope: "grid" },
  { action: "grid_previous_sample", keys: [{ key: "Tab", shift: true }], scope: "grid" },
  { action: "grid_first_sample", keys: [{ key: "Home" }], scope: "grid" },
  { action: "grid_last_sample", keys: [{ key: "End" }], scope: "grid" },
  {
    action: "grid_first_row",
    keys: [
      { key: "ArrowUp", mod: true },
      { key: "Home", mod: true },
    ],
    scope: "grid",
  },
  {
    action: "grid_last_row",
    keys: [
      { key: "ArrowDown", mod: true },
      { key: "End", mod: true },
    ],
    scope: "grid",
  },
  { action: "grid_edit", keys: [{ key: "F2" }], scope: "grid" },
  { action: "grid_clear", keys: [{ key: "Delete" }, { key: "Backspace" }], scope: "grid" },
  { action: "grid_cancel", keys: [{ key: "Escape" }], scope: "grid" },
  { action: "grid_ok", keys: [{ key: "o" }, { key: "+" }], scope: "grid" },
  { action: "grid_not_ok", keys: [{ key: "n" }, { key: "x" }], scope: "grid" },
  { action: "grid_toggle", keys: [{ key: " " }], scope: "grid" },
  { action: "grid_paste", keys: [{ key: "v", mod: true }], scope: "grid" },
];

/** The parts of a keyboard event the matcher needs. */
export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export function isMacPlatform(): boolean {
  return typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);
}

function matches(combo: KeyCombo, input: KeyInput, mac: boolean): boolean {
  const modPressed = mac ? input.metaKey : input.ctrlKey;
  const otherModPressed = mac ? input.ctrlKey : input.metaKey;
  if (
    input.altKey !== (combo.alt ?? false) ||
    otherModPressed ||
    modPressed !== (combo.mod ?? false)
  ) {
    return false;
  }
  if (combo.shift !== undefined && input.shiftKey !== combo.shift) {
    return false;
  }
  return combo.key.length === 1
    ? input.key.toLowerCase() === combo.key.toLowerCase()
    : input.key === combo.key;
}

/** Scopes of the window while the drawing is shown and nothing special has the focus. */
const WINDOW_SCOPES: readonly ShortcutScope[] = ["app", "drawing"];

/** The shortcut of one of `scopes` a key press triggers, or `null`. */
export function matchShortcut(
  input: KeyInput,
  mac: boolean = isMacPlatform(),
  scopes: ShortcutScope | readonly ShortcutScope[] = WINDOW_SCOPES,
): Shortcut | null {
  const active: readonly ShortcutScope[] = typeof scopes === "string" ? [scopes] : scopes;
  return (
    SHORTCUTS.find(
      (s) => active.includes(s.scope) && s.keys.some((combo) => matches(combo, input, mac)),
    ) ?? null
  );
}

const KEY_NAMES: Record<string, string> = {
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  PageUp: "PgUp",
  PageDown: "PgDn",
  " ": "Space",
  Enter: "↵",
  Delete: "Del",
  Tab: "⇥",
  Backspace: "⌫",
  Escape: "Esc",
};

/** Display text of a key combination, for example `⌘O` on macOS and `Ctrl+O` elsewhere. */
export function formatCombo(combo: KeyCombo, mac: boolean = isMacPlatform()): string {
  const key =
    KEY_NAMES[combo.key] ?? (combo.key.length === 1 ? combo.key.toUpperCase() : combo.key);
  if (mac) {
    return `${combo.alt ? "⌥" : ""}${combo.shift ? "⇧" : ""}${combo.mod ? "⌘" : ""}${key}`;
  }
  const parts = [combo.mod ? "Ctrl" : "", combo.alt ? "Alt" : "", combo.shift ? "Shift" : ""];
  return [...parts.filter((p) => p !== ""), key].join("+");
}

/** Display text of the main combination of an action. */
export function shortcutKeys(action: ShortcutAction, mac: boolean = isMacPlatform()): string {
  const combo = SHORTCUTS.find((s) => s.action === action)?.keys[0];
  return combo ? formatCombo(combo, mac) : "";
}
