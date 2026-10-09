// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { ThemeController, effectiveTheme } from "./theme";

describe("theme (D-51)", () => {
  it("follows the system only for the system setting", () => {
    expect(effectiveTheme("system", true)).toBe("dark");
    expect(effectiveTheme("system", false)).toBe("light");
    expect(effectiveTheme("light", true)).toBe("light");
    expect(effectiveTheme("dark", false)).toBe("dark");
  });

  it("sets data-theme on the root element", () => {
    const root = document.createElement("div");
    const controller = new ThemeController(root);
    controller.apply("dark");
    expect(root.dataset.theme).toBe("dark");
    controller.apply("light");
    expect(root.dataset.theme).toBe("light");
    controller.apply("system");
    expect(["light", "dark"]).toContain(root.dataset.theme);
  });
});
