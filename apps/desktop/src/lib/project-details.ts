/**
 * Helpers of the project and report details panel (T3.4). The values live in the project and
 * change only through Rust commands; Rust validates every value. The side panel and File > Info
 * (T3.16) share the edit helpers below, built on `commitField` (MF F8): after the command the
 * field shows what the project holds, Rust's trimmed text or the old value when Rust refused
 * (the reason appears in the project messages). Each command carries only the changed field, and Rust
 * applies it to the project as it is when the command runs, so two fast edits of two fields
 * both survive (MF F4).
 */

import type { Command, Project, ProjectInfo, ReportInfo, RevisionId } from "$lib/ipc/bindings";
import { commitField } from "$lib/ui/fields";

/** The calendar date of `date` in the local time zone as `YYYY-MM-DD` (the report date form). */
export function localDate(date: Date): string {
  const year = String(date.getFullYear()).padStart(4, "0");
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** The project store as the edit helpers need it (the app store, or a fake in tests). */
interface DetailsStore {
  readonly project: Project | null;
  execute(command: Command): Promise<unknown>;
}

/** Sends a changed part field with `update_project_info` (one undo step). */
export function editProjectInfo(
  store: DetailsStore,
  input: HTMLInputElement,
  field: keyof ProjectInfo,
): Promise<void> {
  return commitField(
    input,
    () => store.project?.info[field],
    (value) => store.execute({ type: "update_project_info", values: [{ field, value }] }),
  );
}

/** Sends a changed report header field with `update_report_info` (one undo step). */
export function editReportInfo(
  store: DetailsStore,
  input: HTMLInputElement | HTMLTextAreaElement,
  field: keyof ReportInfo,
): Promise<void> {
  return commitField(
    input,
    () => store.project?.report_info[field],
    (value) => store.execute({ type: "update_report_info", values: [{ field, value }] }),
  );
}

/**
 * Sends a changed label of drawing revision `id` with `update_revision_label`. The command
 * carries only the label, so it cannot overwrite another field.
 */
export function editRevisionLabel(
  store: DetailsStore,
  input: HTMLInputElement,
  id: RevisionId,
): Promise<void> {
  return commitField(
    input,
    () => store.project?.revisions.find((r) => r.id === id)?.label,
    (label) => store.execute({ type: "update_revision_label", revision: id, label }),
  );
}

/** Sets the report date to today (local calendar day). */
export function setReportDateToday(store: DetailsStore): void {
  const value = localDate(new Date());
  void store.execute({ type: "update_report_info", values: [{ field: "report_date", value }] });
}
