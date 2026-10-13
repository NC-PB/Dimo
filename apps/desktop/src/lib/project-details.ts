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
  executeWith(build: (project: Project) => Command | null): Promise<unknown>;
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

/**
 * Sends a changed part field with `update_project_info` (one undo step). The whole info is
 * built when the command runs, from the project after every earlier edit, so a fast second
 * edit of another field never sends this field's old value (T3.18).
 */
export function editProjectInfo(
  store: DetailsStore,
  input: HTMLInputElement,
  field: keyof ProjectInfo,
): Promise<void> {
  const value = input.value;
  return send(
    input,
    () => store.project?.info[field],
    () =>
      store.executeWith((project) =>
        project.info[field] === value
          ? null
          : { type: "update_project_info", info: { ...project.info, [field]: value } },
      ),
  );
}

/** Sends a changed report header field with `update_report_info`, built like the part info. */
export function editReportInfo(
  store: DetailsStore,
  input: HTMLInputElement | HTMLTextAreaElement,
  field: keyof ReportInfo,
): Promise<void> {
  const value = input.value;
  return send(
    input,
    () => store.project?.report_info[field],
    () => store.executeWith((project) => reportInfoCommand(project, field, value)),
  );
}

/** `update_report_info` with one field changed, or `null` when it already has that value. */
function reportInfoCommand(
  project: Project,
  field: keyof ReportInfo,
  value: string,
): Command | null {
  return project.report_info[field] === value
    ? null
    : { type: "update_report_info", info: { ...project.report_info, [field]: value } };
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
  const label = input.value;
  return send(
    input,
    () => store.project?.revisions.find((r) => r.id === id)?.label,
    () => store.execute({ type: "update_revision_label", revision: id, label }),
  );
}

/** Sets the report date to today (local calendar day). */
export function setReportDateToday(store: DetailsStore): void {
  const today = localDate(new Date());
  void store.executeWith((project) => reportInfoCommand(project, "report_date", today));
}
