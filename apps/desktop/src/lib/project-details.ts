/**
 * Helpers of the project and report details panel (T3.4). The values live in the project and
 * change only through Rust commands; Rust validates every value. The side panel and File > Info
 * (T3.16) share the edit helpers below.
 */

import type { Command, Project, ProjectInfo, ReportInfo, RevisionId } from "$lib/ipc/bindings";

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

/**
 * Sends a changed field. Afterwards the input shows what the project holds (`read`): Rust's
 * trimmed text, or the old value when Rust refused (the reason appears in the project
 * messages).
 */
async function send(
  input: HTMLInputElement | HTMLTextAreaElement,
  read: () => string | undefined,
  run: () => Promise<unknown>,
): Promise<void> {
  if (input.value === read()) {
    return;
  }
  await run();
  input.value = read() ?? "";
}

/** Sends a changed part field with `update_project_info` (one undo step). */
export function editProjectInfo(
  store: DetailsStore,
  input: HTMLInputElement,
  field: keyof ProjectInfo,
): Promise<void> {
  const project = store.project;
  if (!project) {
    return Promise.resolve();
  }
  const info = { ...project.info, [field]: input.value };
  return send(
    input,
    () => store.project?.info[field],
    () => store.execute({ type: "update_project_info", info }),
  );
}

/** Sends a changed report header field with `update_report_info` (one undo step). */
export function editReportInfo(
  store: DetailsStore,
  input: HTMLInputElement | HTMLTextAreaElement,
  field: keyof ReportInfo,
): Promise<void> {
  const project = store.project;
  if (!project) {
    return Promise.resolve();
  }
  const info = { ...project.report_info, [field]: input.value };
  return send(
    input,
    () => store.project?.report_info[field],
    () => store.execute({ type: "update_report_info", info }),
  );
}

/** Sends a changed label of drawing revision `id` with `update_revision_label`. */
export function editRevisionLabel(
  store: DetailsStore,
  input: HTMLInputElement,
  id: RevisionId,
): Promise<void> {
  const label = input.value;
  return send(
    input,
    () => store.project?.revisions.find((r) => r.id === id)?.label,
    () => store.execute({ type: "update_revision_label", revision: id, label }),
  );
}

/** Sets the report date to today (local calendar day). */
export function setReportDateToday(store: DetailsStore): void {
  const project = store.project;
  if (project) {
    const info = { ...project.report_info, report_date: localDate(new Date()) };
    void store.execute({ type: "update_report_info", info });
  }
}
