/**
 * The keyboard shortcut reference of the user guide (`docs/user/<lang>/shortcuts.md`, NFR-UX-06)
 * must list every shortcut of `shortcuts.ts` with its keys and its translated name. The tables
 * between the `<!-- shortcuts:<group> -->` and `<!-- /shortcuts -->` markers are generated from
 * `SHORTCUTS`; when this test fails, it prints the tables to paste.
 */

import { describe, expect, it } from "vitest";
import { shortcutLabel } from "./i18n";
import { locales, overwriteGetLocale } from "./paraglide/runtime.js";
import { SHORTCUTS, formatCombo, type Shortcut } from "./shortcuts";

type Locale = (typeof locales)[number];

const pages = import.meta.glob<string>("../../../../docs/user/*/shortcuts.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** Shortcut groups of the reference, in page order. */
const GROUPS = {
  anywhere: (s: Shortcut) => (s.scope ?? "global") === "global" && s.anyView === true,
  drawing: (s: Shortcut) => (s.scope ?? "global") === "global" && s.anyView !== true,
  table: (s: Shortcut) => s.scope === "table",
} as const;

type Group = keyof typeof GROUPS;

/** Table header cells and the word between alternative keys, per language of the guide. */
const WORDS: Record<string, { header: [string, string, string]; or: string }> = {
  en: { header: ["Action", "macOS", "Windows and Linux"], or: "or" },
  de: { header: ["Aktion", "macOS", "Windows und Linux"], or: "oder" },
};

/** Languages that have a page, from the file names. */
const LANGUAGES = Object.keys(pages)
  .map((p) => /user\/([^/]+)\/shortcuts\.md$/.exec(p)?.[1] ?? "")
  .sort();

function keys(shortcut: Shortcut, mac: boolean, or: string): string {
  return shortcut.keys.map((combo) => `\`${formatCombo(combo, mac)}\``).join(` ${or} `);
}

/** The generated table of one group in one language. */
function table(group: Group, locale: string): string {
  const words = WORDS[locale];
  if (words === undefined) {
    throw new Error(`Add the table header words of "${locale}" to WORDS in shortcuts-docs.test.ts`);
  }
  overwriteGetLocale(() => locale as Locale);
  try {
    const [action, mac, other] = words.header;
    const or = words.or;
    const rows = SHORTCUTS.filter(GROUPS[group]).map(
      (s) => `| ${shortcutLabel(s.action)} | ${keys(s, true, or)} | ${keys(s, false, or)} |`,
    );
    return [`| ${action} | ${mac} | ${other} |`, "|---|---|---|", ...rows].join("\n");
  } finally {
    overwriteGetLocale(() => "en");
  }
}

/** The text between the markers of `group`, or `null` if the page has none. */
function block(page: string, group: Group): string | null {
  const start = `<!-- shortcuts:${group} -->\n`;
  const from = page.indexOf(start);
  const to = page.indexOf("\n<!-- /shortcuts -->", from);
  return from < 0 || to < 0 ? null : page.slice(from + start.length, to);
}

describe("keyboard shortcut reference of the user guide (NFR-UX-06)", () => {
  it("has a page in English and German, and only for languages of the app", () => {
    expect(LANGUAGES).toEqual(expect.arrayContaining(["de", "en"]));
    for (const language of LANGUAGES) {
      expect(locales as readonly string[]).toContain(language);
    }
  });

  it("groups cover every shortcut exactly once", () => {
    const covered = (Object.keys(GROUPS) as Group[]).flatMap((g) =>
      SHORTCUTS.filter(GROUPS[g]).map((s) => s.action),
    );
    expect(covered.sort()).toEqual(SHORTCUTS.map((s) => s.action).sort());
  });

  for (const locale of LANGUAGES) {
    for (const group of Object.keys(GROUPS) as Group[]) {
      it(`lists the ${group} shortcuts in ${locale} as in shortcuts.ts`, () => {
        const page = Object.entries(pages).find(([p]) => p.includes(`/${locale}/`))?.[1] ?? "";
        const expected = table(group, locale);
        expect(
          block(page, group),
          `docs/user/${locale}/shortcuts.md, group "${group}". Replace the text between the markers with:\n\n${expected}\n`,
        ).toBe(expected);
      });
    }
  }
});
