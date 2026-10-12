import { m } from "$lib/i18n";
import type { OverallVerdict, ResultType, Verdict } from "$lib/ipc/bindings";

/** Translated name of a verdict of a cell or a group (M3 decision 3). */
export function verdictLabel(verdict: Verdict | OverallVerdict): string {
  switch (verdict) {
    case "pass":
      return m.verdict_pass();
    case "fail":
      return m.verdict_fail();
    case "not_evaluated":
      return m.verdict_not_evaluated();
    case "missing":
      return m.verdict_missing();
    case "open":
      return m.verdict_open();
  }
}

/**
 * Translated name of a result type (FR-MEA-06). For `auto`, `effective` is the type Rust uses
 * now, shown in brackets.
 */
export function resultTypeLabel(type: ResultType, effective?: ResultType): string {
  switch (type) {
    case "variable":
      return m.result_type_variable();
    case "attribute":
      return m.result_type_attribute();
    case "auto":
      return effective === undefined || effective === "auto"
        ? m.result_type_auto()
        : m.result_type_auto_effective({ type: resultTypeLabel(effective) });
  }
}
