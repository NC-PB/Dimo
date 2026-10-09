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
          `${k.mod ? "mod+" : ""}${k.shift === undefined ? "" : `shift=${String(k.shift)}+`}${k.key.toLowerCase()}`,
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

  it("rotates right with R and left with Shift+R (FR-DOC-05)", () => {
    expect(matchShortcut(key("r"), true)?.action).toBe("rotate_right");
    expect(matchShortcut(key("R", { shift: true }), false)?.action).toBe("rotate_left");
    expect(matchShortcut(key("r", { meta: true }), true)).toBeNull();
    expect(shortcutKeys("rotate_left", false)).toBe("Shift+R");
    expect(shortcutKeys("rotate_left", true)).toBe("⇧R");
    expect(shortcutKeys("rotate_right", false)).toBe("R");
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
});
