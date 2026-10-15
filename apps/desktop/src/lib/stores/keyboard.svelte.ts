/**
 * Which window shortcuts act right now (MF F8, D-52). Two things decide it:
 *
 * - Open dialogs. `AppDialog` counts itself in {@link KeyboardStore.modals} while it is open;
 *   no window shortcut acts then, the dialog has the keys (Escape closes it).
 * - Areas with a shortcut scope (`shortcuts.ts`). The drawing registers `drawing` while it is
 *   shown; the characteristic table and the measurement grid register `table` and `grid`, which
 *   count only while they have the focus. Later areas (image editor, review) register their own.
 *
 * `Shortcuts.svelte` asks {@link KeyboardStore.shortcutFor} on every key press and runs the
 * answer. View state only.
 */

import {
  FOCUS_SCOPES,
  isMacPlatform,
  matchShortcut,
  type KeyInput,
  type Shortcut,
  type ShortcutScope,
} from "$lib/shortcuts";

/** A scope an area of the window registers; `app` belongs to the window itself. */
export type AreaScope = Exclude<ShortcutScope, "app">;

/** An area of the window with its own shortcut scope. */
export interface ShortcutArea {
  scope: AreaScope;
  /** The area's element. Keys typed inside it belong to the area. */
  element: () => HTMLElement | null;
  /**
   * Whether the area is shown. A shown area's keys act also while nothing or another control
   * (a ribbon button, a pane) has the focus. Focus scopes (`table`, `grid`) ignore it.
   */
  shown?: () => boolean;
}

/** The scopes a key press acts in, and whether its area itself (or nothing) has the focus. */
export interface ActiveScopes {
  scopes: ShortcutScope[];
  focused: boolean;
}

/** Text entry and choice controls keep their own keys. */
function isEditable(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.closest("input, select, textarea") !== null)
  );
}

function isFocusScope(scope: ShortcutScope): boolean {
  return FOCUS_SCOPES.includes(scope);
}

export class KeyboardStore {
  /** Number of open dialogs. While any is open, no window shortcut acts. */
  modals = $state(0);
  #areas: ShortcutArea[] = [];

  /** Counts one open dialog until the returned function is called (once). */
  openModal(): () => void {
    this.modals += 1;
    let open = true;
    return () => {
      if (open) {
        open = false;
        this.modals -= 1;
      }
    };
  }

  /** Registers an area until the returned function is called. */
  register(area: ShortcutArea): () => void {
    this.#areas = [...this.#areas, area];
    return () => {
      this.#areas = this.#areas.filter((a) => a !== area);
    };
  }

  /** The registered area that holds `target`: the innermost one if areas are nested. */
  #areaOf(target: Node): ShortcutArea | undefined {
    const holding = this.#areas.flatMap((area) => {
      const element = area.element();
      return element?.contains(target) === true ? [{ area, element }] : [];
    });
    // The innermost area lies inside every other one that holds the target.
    return holding.find((a) => holding.every((b) => b.element.contains(a.element)))?.area;
  }

  /** The scopes a key typed at `target` (the focused element) acts in. */
  scopesAt(target: EventTarget | null): ActiveScopes {
    const node = target instanceof Node ? target : null;
    const nothing = node === null || node === document.body || node === document.documentElement;
    const area = nothing ? undefined : this.#areaOf(node);
    if (area !== undefined && isFocusScope(area.scope)) {
      // The table or grid handles its keys itself; only the window's own keys act from it.
      return { scopes: ["app"], focused: true };
    }
    const shown = this.#areas
      .filter((a) => !isFocusScope(a.scope) && (a.shown?.() ?? true))
      .map((a) => a.scope);
    return {
      scopes: ["app", ...shown],
      focused: nothing || (area !== undefined && shown.includes(area.scope)),
    };
  }

  /**
   * The window shortcut a key press at `target` triggers, or `null`: nothing while a dialog is
   * open, only shortcuts of the active scopes, text fields keep their keys (a Cmd/Ctrl key
   * without a text editing meaning still acts), and focus only keys need their area focused.
   */
  shortcutFor(
    input: KeyInput,
    target: EventTarget | null,
    mac: boolean = isMacPlatform(),
  ): Shortcut | null {
    if (this.modals > 0) {
      return null;
    }
    const { scopes, focused } = this.scopesAt(target);
    const shortcut = matchShortcut(input, mac, scopes);
    if (shortcut === null) {
      return null;
    }
    if (
      isEditable(target) &&
      (shortcut.textEditing === true || shortcut.keys.every((k) => k.mod !== true))
    ) {
      return null;
    }
    if (shortcut.focusOnly === true && !focused) {
      return null;
    }
    return shortcut;
  }
}

export const keyboard = new KeyboardStore();
