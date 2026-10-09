import { commandErrorMessage, getLocale, m } from "$lib/i18n";
import type { CommandError, NumberingLock } from "$lib/ipc/bindings";

/** Prefix Tauri puts before the error of a command argument Rust could not read. */
const ARGUMENT_ERROR = /^invalid args `[^`]*` for command `[^`]*`: /;

/**
 * Why Rust refused a change, as shown next to the cell or the moved rows. Values Rust cannot
 * read (for example `1,5` as a decimal) come back as the argument error of the `execute` call;
 * the Tauri prefix is dropped so the message starts with Rust's own text.
 */
export function refusalDetail(error: CommandError): string {
  switch (error.kind) {
    case "rejected":
    case "invalid_argument":
    case "io":
      return error.message.replace(ARGUMENT_ERROR, "");
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
