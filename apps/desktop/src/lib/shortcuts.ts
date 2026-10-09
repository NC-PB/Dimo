/**
 * The keyboard shortcut map (D-52, NFR-UX-01). Every shortcut of the app is defined here and
 * listed in the cheat sheet (`?`). Labels come from `shortcutLabel` in `i18n.ts`.
 */

export type ShortcutAction =
  | "open"
  | "zoom_in"
  | "zoom_out"
  | "fit"
  | "pan_left"
  | "pan_right"
  | "pan_up"
  | "pan_down"
  | "previous_sheet"
  | "next_sheet"
  | "show_shortcuts";

/**
 * One key combination. `key` is compared with `KeyboardEvent.key` (letters case insensitive).
 * `mod` is Cmd on macOS and Ctrl elsewhere. Shift is not compared, because symbols like `+` and
 * `?` need Shift on some keyboard layouts and not on others.
 */
export interface KeyCombo {
  key: string;
  mod?: boolean;
}

export interface Shortcut {
  action: ShortcutAction;
  /** The first combination is the one shown in the UI. */
  keys: readonly KeyCombo[];
  /** Arrow keys and similar act only when the drawing or nothing else has focus. */
  viewportOnly?: boolean;
}

export const SHORTCUTS: readonly Shortcut[] = [
  { action: "open", keys: [{ key: "o", mod: true }] },
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
];

/** The parts of a keyboard event the matcher needs. */
export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
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
};

/** Display text of a key combination, for example `⌘O` on macOS and `Ctrl+O` elsewhere. */
export function formatCombo(combo: KeyCombo, mac: boolean = isMacPlatform()): string {
  const key =
    KEY_NAMES[combo.key] ?? (combo.key.length === 1 ? combo.key.toUpperCase() : combo.key);
  if (!combo.mod) {
    return key;
  }
  return mac ? `⌘${key}` : `Ctrl+${key}`;
}

/** Display text of the main combination of an action. */
export function shortcutKeys(action: ShortcutAction, mac: boolean = isMacPlatform()): string {
  const combo = SHORTCUTS.find((s) => s.action === action)?.keys[0];
  return combo ? formatCombo(combo, mac) : "";
}
