import { describe, expect, it } from "vitest";
import { m } from "./paraglide/messages.js";
import { locales } from "./paraglide/runtime.js";

describe("i18n (FR-SET-04)", () => {
  it("has English and German", () => {
    expect(locales).toEqual(["en", "de"]);
  });

  it("has the same message keys in every language (docs/dev/translations.md)", () => {
    const files = import.meta.glob<Record<string, string>>("../../messages/*.json", {
      eager: true,
      import: "default",
    });
    const keys = (messages: Record<string, string>) =>
      Object.keys(messages)
        .filter((k) => k !== "$schema")
        .sort();
    const english = files["../../messages/en.json"];
    expect(english).toBeDefined();
    expect(Object.keys(files)).toHaveLength(locales.length);
    for (const [file, messages] of Object.entries(files)) {
      expect(keys(messages), file).toEqual(keys(english ?? {}));
    }
  });

  it("translates the sample message", () => {
    expect(m.sample_greeting({}, { locale: "en" })).toBe(
      "Create a project from a drawing, or open a project.",
    );
    expect(m.sample_greeting({}, { locale: "de" })).toBe(
      "Lege ein Projekt aus einer Zeichnung an oder öffne ein Projekt.",
    );
  });
});
