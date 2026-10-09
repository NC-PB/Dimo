/**
 * Applies the color theme (D-51). `tokens.css` defines the light colors on `:root` and the dark
 * ones on `:root[data-theme="dark"]`; this module sets `data-theme` to the theme in effect,
 * following the operating system while the setting is `system`.
 */

import type { Theme } from "$lib/ipc/bindings";

export type EffectiveTheme = "light" | "dark";

/** The theme in effect for a setting, given whether the operating system prefers dark. */
export function effectiveTheme(theme: Theme, systemDark: boolean): EffectiveTheme {
  switch (theme) {
    case "light":
      return "light";
    case "dark":
      return "dark";
    case "system":
      return systemDark ? "dark" : "light";
  }
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

/** Keeps `data-theme` on the document root in line with the setting and the system. */
export class ThemeController {
  #theme: Theme = "system";
  #query: MediaQueryList | null = null;
  readonly #root: HTMLElement | null;

  /** `root` defaults to the document element, looked up when a theme is applied. */
  constructor(root: HTMLElement | null = null) {
    this.#root = root;
  }

  /** Applies `theme` now and whenever the system theme changes while it is `system`. */
  apply(theme: Theme): void {
    this.#theme = theme;
    if (this.#query === null && typeof window.matchMedia === "function") {
      this.#query = window.matchMedia(DARK_QUERY);
      this.#query.addEventListener("change", () => {
        this.#update();
      });
    }
    this.#update();
  }

  #update(): void {
    const root = this.#root ?? document.documentElement;
    root.dataset.theme = effectiveTheme(this.#theme, this.#query?.matches ?? false);
  }
}

export const themeController = new ThemeController();
