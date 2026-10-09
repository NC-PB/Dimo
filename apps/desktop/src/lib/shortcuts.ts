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
  | "rotate_left"
  | "rotate_right"
  | "pan_left"
  | "pan_right"
  | "pan_up"
  | "pan_down"
  | "previous_sheet"
  | "next_sheet"
  | "select_tool"
  | "place_tool"
  | "edit_value"
  | "select_all"
  | "delete_selection"
  | "restyle"
  | "cancel"
  | "show_shortcuts";

/**
 * One key combination. `key` is compared with `KeyboardEvent.key` (letters case insensitive).
 * `mod` is Cmd on macOS and Ctrl elsewhere. Shift is compared only when `shift` is given,
 * because symbols like `+` and `?` need Shift on some keyboard layouts and not on others.
 */
export interface KeyCombo {
  key: string;
  mod?: boolean;
  shift?: boolean;
}

export interface Shortcut {
  action: ShortcutAction;
  /** The first combination is the one shown in the UI. */
  keys: readonly KeyCombo[];
  /** Arrow keys and similar act only when the drawing or nothing else has focus. */
  viewportOnly?: boolean;
  /** In a text field the key keeps its text editing meaning (undo typing, not the project). */
  textEditing?: boolean;
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
  // Shift tells the direction, so the keys work on every keyboard layout (FR-DOC-05).
  { action: "rotate_right", keys: [{ key: "r", shift: false }] },
  { action: "rotate_left", keys: [{ key: "r", shift: true }] },
  { action: "pan_left", keys: [{ key: "ArrowLeft" }], viewportOnly: true },
  { action: "pan_right", keys: [{ key: "ArrowRight" }], viewportOnly: true },
  { action: "pan_up", keys: [{ key: "ArrowUp" }], viewportOnly: true },
  { action: "pan_down", keys: [{ key: "ArrowDown" }], viewportOnly: true },
  { action: "previous_sheet", keys: [{ key: "PageUp" }] },
  { action: "next_sheet", keys: [{ key: "PageDown" }] },
  { action: "select_tool", keys: [{ key: "v" }] },
  { action: "place_tool", keys: [{ key: "b" }] },
  { action: "edit_value", keys: [{ key: "Enter" }], viewportOnly: true },
  { action: "select_all", keys: [{ key: "a", mod: true }], textEditing: true },
  {
    action: "delete_selection",
    keys: [{ key: "Delete" }, { key: "Backspace" }],
    viewportOnly: true,
  },
  { action: "restyle", keys: [{ key: "s", shift: false }] },
  { action: "cancel", keys: [{ key: "Escape" }] },
  { action: "show_shortcuts", keys: [{ key: "?" }] },
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
  if (input.altKey || otherModPressed || modPressed !== (combo.mod ?? false)) {
    return false;
  }
  if (combo.shift !== undefined && input.shiftKey !== combo.shift) {
    return false;
  }
  return combo.key.length === 1
    ? input.key.toLowerCase() === combo.key.toLowerCase()
    : input.key === combo.key;
}

/** The shortcut a key press triggers, or `null`. */
export function matchShortcut(input: KeyInput, mac: boolean = isMacPlatform()): Shortcut | null {
  return SHORTCUTS.find((s) => s.keys.some((combo) => matches(combo, input, mac))) ?? null;
}

const KEY_NAMES: Record<string, string> = {
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  PageUp: "PgUp",
  PageDown: "PgDn",
  Enter: "↵",
  Delete: "Del",
  Backspace: "⌫",
  Escape: "Esc",
};

/** Display text of a key combination, for example `⌘O` on macOS and `Ctrl+O` elsewhere. */
export function formatCombo(combo: KeyCombo, mac: boolean = isMacPlatform()): string {
  const key =
    KEY_NAMES[combo.key] ?? (combo.key.length === 1 ? combo.key.toUpperCase() : combo.key);
  if (!combo.mod) {
    return combo.shift ? (mac ? `⇧${key}` : `Shift+${key}`) : key;
  }
  if (combo.shift) {
    return mac ? `⇧⌘${key}` : `Ctrl+Shift+${key}`;
  }
  return mac ? `⌘${key}` : `Ctrl+${key}`;
}

/** Display text of the main combination of an action. */
export function shortcutKeys(action: ShortcutAction, mac: boolean = isMacPlatform()): string {
  const combo = SHORTCUTS.find((s) => s.action === action)?.keys[0];
  return combo ? formatCombo(combo, mac) : "";
}
