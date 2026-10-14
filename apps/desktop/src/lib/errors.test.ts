import { afterEach, describe, expect, it } from "vitest";
import bindings from "./ipc/bindings.ts?raw";
import type { CommandError } from "./ipc/bindings";
import { REASON_TEXTS, asCommandError, commandErrorMessage, errorDetail } from "./errors";
import { overwriteGetLocale, type Locale } from "./paraglide/runtime.js";

/**
 * MF F3: every error kind and reason Rust can send is translated in every UI language. The kinds
 * and reasons are read from the generated bindings, so a new Rust variant without a text fails
 * here even before anyone looks at the UI.
 */

/** The body of `export type <name> = ...;` in the bindings, without doc comments. */
function typeBody(name: string): string {
  const start = bindings.indexOf(`export type ${name} = `);
  expect(start, `type ${name} in bindings.ts`).toBeGreaterThanOrEqual(0);
  const end = bindings.indexOf(";\n\n", start);
  return bindings.slice(start, end).replace(/\/\*\*[\s\S]*?\*\//g, "");
}

/** The string literals of a union of strings such as `"a" | "b"`. */
function literals(name: string): string[] {
  return [...typeBody(name).matchAll(/"([a-z_]+)"/g)].map((match) => match[1] ?? "");
}

/** Every variant of `CommandError` as `[kind, reason type or null]`. */
function commandErrorVariants(): [string, string | null][] {
  return typeBody("CommandError")
    .split("} |")
    .map((variant) => {
      const kind = /kind: "([a-z_]+)"/.exec(variant)?.[1] ?? "";
      const reason = /reason: (\w+)/.exec(variant)?.[1] ?? null;
      return [kind, reason];
    });
}

const DETAIL = "RUST_DIAGNOSTIC_TEXT";

/** One error of each kind and reason, with the diagnostic marker as message. */
function everyError(): CommandError[] {
  const errors: CommandError[] = [];
  for (const [kind, reasonType] of commandErrorVariants()) {
    const reasons = reasonType === null ? [null] : literals(reasonType);
    for (const reason of reasons) {
      errors.push({
        kind,
        ...(reason === null ? {} : { reason }),
        message: DETAIL,
        user: null,
        found: 9,
        supported: 4,
      } as unknown as CommandError);
    }
  }
  return errors;
}

function inLocale<T>(locale: Locale, work: () => T): T {
  overwriteGetLocale(() => locale);
  return work();
}

afterEach(() => {
  overwriteGetLocale(() => "en");
});

describe("error translations (MF F3)", () => {
  it("has a text table for every reason type an error can carry", () => {
    // Errors reach the UI as `CommandError`, as the failed autosave state and as a refused
    // table import.
    const reasonTypes = new Set(
      ["CommandError", "Autosave", "TableImport"].flatMap((name) =>
        [...typeBody(name).matchAll(/reason: (\w+)/g)].map((match) => match[1] ?? ""),
      ),
    );
    expect(reasonTypes.size).toBe(10);
    expect(Object.keys(REASON_TEXTS).sort()).toEqual([...reasonTypes].sort());
  });

  it("translates exactly the reasons Rust defines, in English and German", () => {
    for (const [type, table] of Object.entries(REASON_TEXTS)) {
      const reasons = literals(type);
      expect(reasons.length, type).toBeGreaterThan(0);
      expect(Object.keys(table).sort(), type).toEqual([...reasons].sort());
      for (const reason of reasons) {
        const text = (table as Record<string, () => string>)[reason];
        const en = inLocale("en", () => text?.() ?? "");
        const de = inLocale("de", () => text?.() ?? "");
        expect(en, `${type}.${reason} en`).toMatch(/\S/);
        expect(de, `${type}.${reason} de`).toMatch(/\S/);
        expect(de, `${type}.${reason} is not translated`).not.toBe(en);
      }
    }
  });

  it("translates every error kind and reason without showing Rust's text", () => {
    const errors = everyError();
    const kinds = new Set(errors.map((e) => e.kind));
    expect(kinds.size).toBe(commandErrorVariants().length);
    for (const error of errors) {
      const label = `${error.kind}${"reason" in error ? `.${error.reason}` : ""}`;
      const en = inLocale("en", () => commandErrorMessage(error));
      const de = inLocale("de", () => commandErrorMessage(error));
      expect(en, label).toMatch(/\S/);
      expect(de, label).toMatch(/\S/);
      expect(en, label).not.toContain(DETAIL);
      expect(de, label).not.toContain(DETAIL);
      expect(de, `${label} is not translated`).not.toBe(en);
    }
  });

  it("keeps Rust's text as the diagnostic detail only", () => {
    const error: CommandError = { kind: "io", reason: "target_locked", message: "a.xlsx: busy" };
    expect(errorDetail(error)).toBe("a.xlsx: busy");
    expect(errorDetail({ kind: "no_project" })).toBeNull();
    expect(inLocale("de", () => commandErrorMessage(error))).toBe(
      "Die Datei ist in einem anderen Programm geöffnet. Schließe sie dort und versuche es noch einmal.",
    );
  });

  it("turns text errors from Tauri into command errors", () => {
    const text = "invalid args `command` for command `execute`: invalid decimal";
    expect(asCommandError(text)).toEqual({
      kind: "invalid_argument",
      reason: "unreadable",
      message: text,
    });
    expect(asCommandError(new Error("boom"))).toEqual({
      kind: "internal",
      reason: "bug",
      message: "Error: boom",
    });
    const rust: CommandError = { kind: "unsaved_changes" };
    expect(asCommandError(rust)).toBe(rust);
  });
});
