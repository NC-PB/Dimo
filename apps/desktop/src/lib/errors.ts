import type {
  ArgumentReason,
  AutosaveReason,
  CommandError,
  DocumentReason,
  ExportReason,
  InternalReason,
  IoReason,
  ProjectReason,
  RecentReason,
  RejectReason,
  TableImportReason,
} from "./ipc/bindings";
import { m } from "./paraglide/messages.js";

/**
 * Translations of the errors Rust reports (MF F3). Rust decides which error happened and sends a
 * machine readable `kind` and `reason`; this module turns them into the text the user reads. The
 * English `message` of an error is diagnostic text only, shown collapsed under "Details"
 * (`ErrorDetails.svelte`), never as the main text.
 *
 * Each table is a `Record` over the generated reason type, so TypeScript refuses to compile when
 * Rust adds a reason without a text here; `errors.test.ts` checks every reason in both languages.
 */
type Texts<T extends string> = Record<T, () => string>;

const REJECT: Texts<RejectReason> = {
  unknown_characteristic: m.refusal_unknown_characteristic,
  unknown_balloon: m.refusal_unknown_balloon,
  unknown_sheet: m.refusal_unknown_sheet,
  numbering_locked: m.refusal_numbering_locked,
  invalid_geometry: m.refusal_invalid_geometry,
  invalid_style: m.refusal_invalid_style,
  invalid_sheet_setting: m.refusal_invalid_sheet_setting,
  invalid_project_setting: m.refusal_invalid_project_setting,
  invalid_move_target: m.refusal_invalid_move_target,
  zero_quantity: m.refusal_zero_quantity,
  unknown_sample: m.refusal_unknown_sample,
  unknown_revision: m.refusal_unknown_revision,
  not_an_image: m.refusal_not_an_image,
  invalid_image_edit: m.refusal_invalid_image_edit,
  empty_serial: m.refusal_empty_serial,
  duplicate_serial: m.refusal_duplicate_serial,
  too_many_samples: m.refusal_too_many_samples,
  invalid_sample_count: m.refusal_invalid_sample_count,
  invalid_date: m.refusal_invalid_date,
  invalid_result: m.refusal_invalid_result,
  result_type_conflict: m.refusal_result_type_conflict,
  nothing_to_undo: m.refusal_nothing_to_undo,
  nothing_to_redo: m.refusal_nothing_to_redo,
  internal: m.refusal_internal,
};

const IO: Texts<IoReason> = {
  not_found: m.error_io_not_found,
  permission_denied: m.error_io_permission_denied,
  target_locked: m.error_io_target_locked,
  disk_full: m.error_io_disk_full,
  other: m.error_io_other,
};

const DOCUMENT: Texts<DocumentReason> = {
  not_a_pdf: m.error_document_not_a_pdf,
  image_not_supported: m.error_document_image_not_supported,
  damaged: m.error_document_damaged,
  too_large: m.error_document_too_large,
  no_pdf_engine: m.error_document_no_pdf_engine,
};

const ARGUMENT: Texts<ArgumentReason> = {
  shorter_leg: m.error_argument_shorter_leg,
  region: m.error_argument_region,
  document_id: m.error_argument_document_id,
  unreadable: m.error_argument_unreadable,
  request: m.error_argument_request,
};

const PROJECT: Texts<ProjectReason> = {
  not_a_project: m.error_project_not_a_project,
  damaged_container: m.error_project_damaged_container,
  too_large: m.error_project_too_large,
  migration_failed: m.error_project_migration_failed,
  missing_drawing: m.error_project_missing_drawing,
  missing_table: m.error_project_missing_table,
  changed_table: m.error_project_changed_table,
  no_file: m.error_project_no_file,
};

const EXPORT: Texts<ExportReason> = {
  target_locked: m.error_export_target_locked,
  permission_denied: m.error_export_permission_denied,
  disk_full: m.error_export_disk_full,
  write_failed: m.error_export_write_failed,
  invalid_file_name: m.error_export_invalid_file_name,
  no_pdf_engine: m.error_export_no_pdf_engine,
  pdf_failed: m.error_export_pdf_failed,
  report_failed: m.error_export_report_failed,
  list_failed: m.error_export_list_failed,
  wrong_extension: m.error_export_wrong_extension,
  open_failed: m.error_export_open_failed,
};

const RECENT: Texts<RecentReason> = {
  not_listed: m.error_recent_not_listed,
  missing: m.error_recent_missing,
};

const INTERNAL: Texts<InternalReason> = {
  shipped_tables: m.error_internal_shipped_tables,
  report_template: m.error_internal_report_template,
  window: m.error_internal_window,
  bug: m.error_internal_bug,
};

const AUTOSAVE: Texts<AutosaveReason> = {
  no_folder: m.autosave_failed_no_folder,
  disk_full: m.autosave_failed_disk_full,
  permission_denied: m.autosave_failed_permission_denied,
  write_failed: m.autosave_failed_write_failed,
};

const TABLE_IMPORT: Texts<TableImportReason> = {
  too_large: m.table_import_too_large,
  unreadable: m.table_import_unreadable,
  not_text: m.table_import_not_text,
  syntax: m.table_import_syntax,
  invalid_content: m.table_import_invalid_content,
  not_company_table: m.table_import_not_company_table,
  duplicate_id: m.table_import_duplicate_id,
  settings_conflict: m.table_import_settings_conflict,
};

/** Every reason table by the name of its generated type, for the translation test. */
export const REASON_TEXTS = {
  RejectReason: REJECT,
  IoReason: IO,
  DocumentReason: DOCUMENT,
  ArgumentReason: ARGUMENT,
  ProjectReason: PROJECT,
  ExportReason: EXPORT,
  RecentReason: RECENT,
  InternalReason: INTERNAL,
  AutosaveReason: AUTOSAVE,
  TableImportReason: TABLE_IMPORT,
} as const satisfies Record<string, Texts<string>>;

/** Translated reason of a refused document command. */
export function rejectReasonText(reason: RejectReason): string {
  return REJECT[reason]();
}

/** Translated reason why the autosave journal is not written (NFR-REL-01). */
export function autosaveFailedText(reason: AutosaveReason): string {
  return AUTOSAVE[reason]();
}

/** Translated reason why a company table file was refused (FR-TOL-07). */
export function tableImportText(reason: TableImportReason): string {
  return TABLE_IMPORT[reason]();
}

/** The one translation of an IPC error into the text the user reads. Never Rust's text. */
export function commandErrorMessage(error: CommandError): string {
  switch (error.kind) {
    case "io":
      return IO[error.reason]();
    case "invalid_document":
      return DOCUMENT[error.reason]();
    case "invalid_argument":
      return ARGUMENT[error.reason]();
    case "rejected":
      return m.error_rejected({ message: REJECT[error.reason]() });
    case "no_project":
      return m.error_no_project();
    case "unsaved_changes":
      return m.error_unsaved_changes();
    case "in_use":
      return error.user === null ? m.error_in_use() : m.error_in_use_by({ user: error.user });
    case "newer_version":
      return m.error_newer_version({
        found: String(error.found),
        supported: String(error.supported),
      });
    case "project":
      return PROJECT[error.reason]();
    case "export":
      return EXPORT[error.reason]();
    case "recent":
      return RECENT[error.reason]();
    case "task_failed":
      return m.error_task_failed();
    case "internal":
      return INTERNAL[error.reason]();
  }
}

/** The diagnostic text of an error for the "Details" line, `null` if it has none. */
export function errorDetail(error: CommandError): string | null {
  return "message" in error && error.message !== "" ? error.message : null;
}

/** Prefix Tauri puts before the error of a command argument Rust could not read. */
export const ARGUMENT_ERROR = /^invalid args `[^`]*` for command `[^`]*`: /;

/**
 * An IPC error as a {@link CommandError}. Commands answer with a `CommandError` object; Tauri
 * itself reports arguments it could not read as plain text (reason `unreadable`), and anything
 * else thrown on the way is a bug.
 */
export function asCommandError(error: unknown): CommandError {
  if (typeof error === "object" && error !== null && "kind" in error) {
    return error as CommandError;
  }
  const message = String(error);
  return ARGUMENT_ERROR.test(message)
    ? { kind: "invalid_argument", reason: "unreadable", message }
    : { kind: "internal", reason: "bug", message };
}
