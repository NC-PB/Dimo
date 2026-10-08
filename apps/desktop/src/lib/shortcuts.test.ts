import { describe, expect, it } from "vitest";
import { SHORTCUTS, formatCombo, matchShortcut, shortcutKeys } from "./shortcuts";

const key = (k: string, mods: Partial<{ ctrl: boolean; meta: boolean; alt: boolean }> = {}) => ({
  key: k,
  ctrlKey: mods.ctrl ?? false,
  metaKey: mods.meta ?? false,
  altKey: mods.alt ?? false,
});

describe("shortcut map (D-52)", () => {
  it("defines every action once", () => {
    const actions = SHORTCUTS.map((s) => s.action);
    expect(new Set(actions).size).toBe(actions.length);
  });

  it("never assigns one key combination to two actions", () => {
    const combos = SHORTCUTS.flatMap((s) =>
      s.keys.map((k) => `${k.mod ? "mod+" : ""}${k.key.toLowerCase()}`),
    );
    expect(new Set(combos).size).toBe(combos.length);
  });

  it("uses Cmd on macOS and Ctrl elsewhere", () => {
    expect(matchShortcut(key("o", { meta: true }), true)?.action).toBe("open");
    expect(matchShortcut(key("O", { ctrl: true }), false)?.action).toBe("open");
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

  it("formats keys for the platform", () => {
    expect(formatCombo({ key: "o", mod: true }, true)).toBe("⌘O");
    expect(formatCombo({ key: "o", mod: true }, false)).toBe("Ctrl+O");
    expect(shortcutKeys("pan_left", false)).toBe("←");
    expect(shortcutKeys("next_sheet", false)).toBe("PgDn");
  });
});
