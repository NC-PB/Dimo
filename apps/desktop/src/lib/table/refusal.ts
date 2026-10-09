import { commandErrorMessage, getLocale, m, rejectReasonText } from "$lib/i18n";
import type { CommandError, NumberingLock } from "$lib/ipc/bindings";

/** Prefix Tauri puts before the error of a command argument Rust could not read. */
const ARGUMENT_ERROR = /^invalid args `[^`]*` for command `[^`]*`: /;

/** What the refused change was, for errors Rust reports only as an argument it could not read. */
export interface RefusalContext {
  /** The kind of value that was typed: an exact decimal or a quantity. */
  input?: "decimal" | "quantity";
  /** The numbering lock of the project, to explain a refused move. */
  lock?: NumberingLock | null;
}

/** Whether Rust refused because numbering is locked (`NumberingLocked`, D-23). */
export function isNumberingLocked(error: CommandError): boolean {
  return error.kind === "rejected" && error.reason === "numbering_locked";
}

/**
 * Why Rust refused a change, as shown next to the cell or the moved rows, in the UI language.
 *
 * - Refused commands carry a machine readable `reason`; known ones are translated, the others
 *   show Rust's English message.
 * - Values Rust cannot read (for example `1,5` as a decimal, or `1.5` as a quantity) come back
 *   as the argument error of the `execute` call, without a kind. The typed value's kind
 *   (`context.input`) tells which text applies; without it, Rust's own text is shown with the
 *   Tauri prefix dropped.
 */
export function refusalDetail(error: CommandError, context: RefusalContext = {}): string {
  switch (error.kind) {
    case "rejected":
      if (isNumberingLocked(error) && context.lock) {
        return lockExplanation(context.lock);
      }
      return rejectReasonText(error.reason) ?? error.message;
    case "invalid_argument": {
      if (ARGUMENT_ERROR.test(error.message)) {
        if (context.input === "decimal") {
          return m.refusal_invalid_decimal();
        }
        if (context.input === "quantity") {
          return m.refusal_invalid_quantity();
        }
      }
      return error.message.replace(ARGUMENT_ERROR, "");
    }
    case "io":
      return error.message;
    default:
      return commandErrorMessage(error);
  }
}

/** Why characteristics cannot be moved while numbering is locked (D-23). */
export function lockExplanation(lock: NumberingLock): string {
  const at = new Date(lock.locked_at);
  const date = Number.isNaN(at.getTime())
    ? lock.locked_at
    : new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium" }).format(at);
  const values = { user: lock.locked_by, date };
  return lock.reason === "issued_report"
    ? m.table_locked_issued(values)
    : m.table_locked_manual(values);
}
