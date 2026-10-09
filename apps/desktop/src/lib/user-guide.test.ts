/**
 * Structure of the user guide (NFR-UX-06): every page exists in English and German with the same
 * headings and tables, links between pages resolve, and the index lists every page.
 */

import { describe, expect, it } from "vitest";

const files = import.meta.glob<string>("../../../../docs/user/*/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
});

/** Pages by language, then by file name. */
const guide: Record<string, Record<string, string>> = {};
for (const [path, text] of Object.entries(files)) {
  const match = /user\/([^/]+)\/([^/]+\.md)$/.exec(path);
  if (match?.[1] !== undefined && match[2] !== undefined) {
    (guide[match[1]] ??= {})[match[2]] = text;
  }
}

const en = guide["en"] ?? {};
const de = guide["de"] ?? {};

/** Number of lines of each kind that must match between the languages. */
function shape(text: string): { headings: number; tableRows: number; codeBlocks: number } {
  const lines = text.split("\n");
  return {
    headings: lines.filter((l) => /^#{1,6} /.test(l)).length,
    tableRows: lines.filter((l) => l.startsWith("|")).length,
    codeBlocks: lines.filter((l) => l.startsWith("```")).length,
  };
}

describe("user guide (NFR-UX-06)", () => {
  it("has the same pages in English and German", () => {
    expect(Object.keys(de).sort()).toEqual(Object.keys(en).sort());
    expect(Object.keys(en)).toContain("README.md");
    expect(Object.keys(en)).toContain("shortcuts.md");
  });

  it("keeps headings, table rows and code blocks in step between the languages", () => {
    for (const [name, text] of Object.entries(en)) {
      expect(shape(de[name] ?? ""), name).toEqual(shape(text));
    }
  });

  it("links only to pages that exist", () => {
    for (const [language, pages] of Object.entries(guide)) {
      for (const [name, text] of Object.entries(pages)) {
        for (const match of text.matchAll(/\]\(([^)#]+\.md)(?:#[^)]*)?\)/g)) {
          expect(Object.keys(pages), `${language}/${name} links to ${match[1] ?? ""}`).toContain(
            match[1],
          );
        }
      }
    }
  });

  it("lists every page in the index", () => {
    for (const [language, pages] of Object.entries(guide)) {
      const index = pages["README.md"] ?? "";
      for (const name of Object.keys(pages)) {
        if (name !== "README.md") {
          expect(index, `${language}/README.md links to ${name}`).toContain(`](${name})`);
        }
      }
    }
  });

  it("uses no dashes as sentence breaks (AGENTS.md style of written text)", () => {
    for (const [language, pages] of Object.entries(guide)) {
      for (const [name, text] of Object.entries(pages)) {
        expect(text, `${language}/${name}`).not.toMatch(/[–—]/);
      }
    }
  });
});
