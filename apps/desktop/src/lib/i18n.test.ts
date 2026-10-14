import { describe, expect, it } from "vitest";
import type { RejectReason } from "$lib/ipc/bindings";
import { rejectReasonText } from "./i18n";
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
    expect(m.start_new_title({}, { locale: "en" })).toBe("New project from drawing");
    expect(m.start_new_title({}, { locale: "de" })).toBe("Neues Projekt aus Zeichnung");
  });

  it("translates the sample, date and result refusals added in T3.1", () => {
    const reasons: RejectReason[] = [
      "unknown_sample",
      "unknown_revision",
      "not_an_image",
      "invalid_image_edit",
      "empty_serial",
      "duplicate_serial",
      "too_many_samples",
      "invalid_sample_count",
      "invalid_date",
      "invalid_result",
      "result_type_conflict",
      "unknown_balloon",
      "unknown_sheet",
      "invalid_geometry",
      "invalid_style",
      "invalid_sheet_setting",
      "invalid_project_setting",
      "nothing_to_undo",
      "nothing_to_redo",
    ];
    for (const reason of reasons) {
      expect(rejectReasonText(reason), reason).toMatch(/\S/);
    }
    expect(m.refusal_invalid_date({}, { locale: "de" })).toContain("Datum");
  });
});
