import { describe, expect, it } from "vitest";
import { m } from "./paraglide/messages.js";
import { locales } from "./paraglide/runtime.js";

describe("i18n (FR-SET-04)", () => {
  it("has English and German", () => {
    expect(locales).toEqual(["en", "de"]);
  });

  it("translates the sample message", () => {
    expect(m.sample_greeting({}, { locale: "en" })).toBe("Open a drawing to start.");
    expect(m.sample_greeting({}, { locale: "de" })).toBe("Öffne eine Zeichnung, um zu beginnen.");
  });
});
