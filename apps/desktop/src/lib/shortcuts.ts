/**
 * The keyboard shortcut map (D-52, NFR-UX-01). Every shortcut of the app is defined here and
 * listed in the cheat sheet (`?`). Labels come from `shortcutLabel` in `i18n.ts`.
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
  | "pan_left"
  | "pan_right"
  | "pan_up"
  | "pan_down"
  | "previous_sheet"
  | "next_sheet"
  | "show_shortcuts"
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
  | "table_move_down";

/**
 * Where a shortcut acts. `global` shortcuts are handled by the window (`Shortcuts.svelte`),
 * `table` shortcuts only while the characteristic table has focus (T1.7).
 */
export type ShortcutScope = "global" | "table";

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
  /** Arrow keys and similar act only when the drawing or nothing else has focus. */
  viewportOnly?: boolean;
  /** In a text field the key keeps its text editing meaning (undo typing, not the project). */
  textEditing?: boolean;
  /** Where the shortcut acts; `global` when not given. */
  scope?: ShortcutScope;
}

export const SHORTCUTS: readonly Shortcut[] = [
  { action: "new_project", keys: [{ key: "n", mod: true }] },
  { action: "open_project", keys: [{ key: "o", mod: true }] },
  { action: "save_project", keys: [{ key: "s", mod: true, shift: false }] },
  { action: "save_project_as", keys: [{ key: "s", mod: true, shift: true }] },
  { action: "undo", keys: [{ key: "z", mod: true, shift: false }], textEditing: true },
  {
    action: "redo",
    keys: [
      { key: "z", mod: true, shift: true },
      { key: "y", mod: true, shift: false },
    ],
    textEditing: true,
  },
  { action: "zoom_in", keys: [{ key: "+" }, { key: "=" }] },
  { action: "zoom_out", keys: [{ key: "-" }, { key: "_" }] },
  { action: "fit", keys: [{ key: "0" }] },
  { action: "pan_left", keys: [{ key: "ArrowLeft" }], viewportOnly: true },
  { action: "pan_right", keys: [{ key: "ArrowRight" }], viewportOnly: true },
  { action: "pan_up", keys: [{ key: "ArrowUp" }], viewportOnly: true },
  { action: "pan_down", keys: [{ key: "ArrowDown" }], viewportOnly: true },
  { action: "previous_sheet", keys: [{ key: "PageUp" }] },
  { action: "next_sheet", keys: [{ key: "PageDown" }] },
  { action: "show_shortcuts", keys: [{ key: "?" }] },
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

/** The shortcut of `scope` a key press triggers, or `null`. */
export function matchShortcut(
  input: KeyInput,
  mac: boolean = isMacPlatform(),
  scope: ShortcutScope = "global",
): Shortcut | null {
  return (
    SHORTCUTS.find(
      (s) => (s.scope ?? "global") === scope && s.keys.some((combo) => matches(combo, input, mac)),
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
