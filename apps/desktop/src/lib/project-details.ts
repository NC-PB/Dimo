/**
 * Helpers of the project and report details panel (T3.4). The values live in the project and
 * change only through Rust commands; Rust validates every value.
 */

/** The calendar date of `date` in the local time zone as `YYYY-MM-DD` (the report date form). */
export function localDate(date: Date): string {
  const year = String(date.getFullYear()).padStart(4, "0");
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}
