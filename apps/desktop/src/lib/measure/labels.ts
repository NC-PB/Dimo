import { m } from "$lib/i18n";
import type { LimitPosition, OverallVerdict, ResultType, Verdict } from "$lib/ipc/bindings";

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

/**
 * The verdict of one cell in words for the Measure bar (T3.19), for example "pass, on the lower
 * limit". Both the verdict and where the value lies against the limits come from Rust
 * (`measurement_state`); nothing is compared here (rule 2).
 */
export function cellVerdictText(verdict: Verdict, position: LimitPosition | null): string {
  switch (verdict) {
    case "pass":
      return position === "on_lower"
        ? m.measure_verdict_pass_lower()
        : position === "on_upper"
          ? m.measure_verdict_pass_upper()
          : m.measure_verdict_pass();
    case "fail":
      return position === "below"
        ? m.measure_verdict_fail_below()
        : position === "above"
          ? m.measure_verdict_fail_above()
          : m.measure_verdict_fail();
    case "not_evaluated":
      return m.measure_verdict_not_evaluated();
    case "missing":
      return m.measure_verdict_missing();
  }
}
