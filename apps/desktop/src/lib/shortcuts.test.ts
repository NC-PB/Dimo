import { describe, expect, it } from "vitest";
import { SHORTCUTS, formatCombo, matchShortcut, shortcutKeys } from "./shortcuts";

const key = (
  k: string,
  mods: Partial<{ ctrl: boolean; meta: boolean; alt: boolean; shift: boolean }> = {},
) => ({
  key: k,
  ctrlKey: mods.ctrl ?? false,
  metaKey: mods.meta ?? false,
  altKey: mods.alt ?? false,
  shiftKey: mods.shift ?? false,
});

describe("shortcut map (D-52)", () => {
  it("defines every action once", () => {
    const actions = SHORTCUTS.map((s) => s.action);
    expect(new Set(actions).size).toBe(actions.length);
  });

  it("never assigns one key combination to two actions", () => {
    const combos = SHORTCUTS.flatMap((s) =>
      s.keys.map(
        (k) =>
          `${s.scope ?? "global"}:${k.mod ? "mod+" : ""}${k.alt ? "alt+" : ""}${k.shift === undefined ? "" : `shift=${String(k.shift)}+`}${k.key.toLowerCase()}`,
      ),
    );
    expect(new Set(combos).size).toBe(combos.length);
  });

  it("uses Cmd on macOS and Ctrl elsewhere", () => {
    expect(matchShortcut(key("o", { meta: true }), true)?.action).toBe("open_project");
    expect(matchShortcut(key("O", { ctrl: true }), false)?.action).toBe("open_project");
    expect(matchShortcut(key("o", { ctrl: true }), true)).toBeNull();
    expect(matchShortcut(key("o"), true)).toBeNull();
  });

  it("matches symbols whatever the keyboard layout needs for them", () => {
    expect(matchShortcut(key("+"), true)?.action).toBe("zoom_in");
    expect(matchShortcut(key("="), true)?.action).toBe("zoom_in");
    expect(matchShortcut(key("-"), false)?.action).toBe("zoom_out");
    expect(matchShortcut(key("0"), false)?.action).toBe("fit");
    expect(matchShortcut(key("?"), false)?.action).toBe("show_shortcuts");
    expect(matchShortcut(key("+", { alt: true }), false)).toBeNull();
  });

  it("tells undo from redo and save from save as by Shift", () => {
    expect(matchShortcut(key("z", { meta: true }), true)?.action).toBe("undo");
    expect(matchShortcut(key("Z", { meta: true, shift: true }), true)?.action).toBe("redo");
    expect(matchShortcut(key("y", { ctrl: true }), false)?.action).toBe("redo");
    expect(matchShortcut(key("s", { ctrl: true }), false)?.action).toBe("save_project");
    expect(matchShortcut(key("S", { ctrl: true, shift: true }), false)?.action).toBe(
      "save_project_as",
    );
    expect(matchShortcut(key("n", { meta: true }), true)?.action).toBe("new_project");
    // Shift stays free for symbols that need it on some layouts.
    expect(matchShortcut(key("?", { shift: true }), true)?.action).toBe("show_shortcuts");
  });

  it("formats keys for the platform", () => {
    expect(formatCombo({ key: "o", mod: true }, true)).toBe("⌘O");
    expect(formatCombo({ key: "o", mod: true }, false)).toBe("Ctrl+O");
    expect(shortcutKeys("pan_left", false)).toBe("←");
    expect(shortcutKeys("next_sheet", false)).toBe("PgDn");
    expect(shortcutKeys("redo", true)).toBe("⇧⌘Z");
    expect(shortcutKeys("save_project_as", false)).toBe("Ctrl+Shift+S");
  });

  it("keeps table keys inside the table", () => {
    expect(matchShortcut(key("ArrowUp"), true)?.action).toBe("pan_up");
    expect(matchShortcut(key("ArrowUp"), true, "table")?.action).toBe("table_up");
    expect(matchShortcut(key("ArrowUp", { shift: true }), true, "table")?.action).toBe(
      "table_extend_up",
    );
    expect(matchShortcut(key("ArrowDown", { alt: true }), true, "table")?.action).toBe(
      "table_move_down",
    );
    expect(matchShortcut(key("ArrowDown", { meta: true }), true, "table")?.action).toBe(
      "table_last_row",
    );
    expect(matchShortcut(key("a", { ctrl: true }), false, "table")?.action).toBe(
      "table_select_all",
    );
    expect(matchShortcut(key("Enter"), true)).toBeNull();
    expect(matchShortcut(key("Enter"), true, "table")?.action).toBe("table_edit");
    expect(matchShortcut(key("+"), true, "table")).toBeNull();
  });

  it("formats Alt and Shift without a modifier", () => {
    expect(shortcutKeys("table_move_up", true)).toBe("⌥↑");
    expect(shortcutKeys("table_move_up", false)).toBe("Alt+↑");
    expect(shortcutKeys("table_extend_down", true)).toBe("⇧↓");
    expect(shortcutKeys("table_extend_down", false)).toBe("Shift+↓");
    expect(shortcutKeys("table_toggle", false)).toBe("Space");
  });
});
