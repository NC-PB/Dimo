<script lang="ts">
  import { disabledReason, runCommand } from "$lib/commands.svelte";
  import JobStatus from "$lib/components/JobStatus.svelte";
  import { commandErrorMessage, m } from "$lib/i18n";
  import type { ReportField } from "$lib/ipc/bindings";
  import { measureUi } from "$lib/measure/measure-ui.svelte";
  import Pane from "$lib/pane/Pane.svelte";
  import { exportStore as appExport, type ExportStore } from "$lib/stores/export.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { reportView as appReportView, type ReportViewStore } from "./report-view.svelte";

  interface Props {
    /** The Report tab state; the app's unless a test passes its own. */
    store?: ReportViewStore;
    /** The export jobs; the app's unless a test passes its own. */
    exports?: ExportStore;
  }

  let { store = appReportView, exports = appExport }: Props = $props();

  /**
   * "Before you issue the report" (T3.19, mockup "Report tab", M3 decisions 15 to 17): what Rust
   * found open in the chosen document, with a way to fix each, then Issue report and Save copy
   * with the state of the export. Nothing here blocks issuing; the warnings are confirmed once
   * more when Issue report is pressed.
   */
  const checks = $derived(store.checks);
  const kind = $derived(exports.document);
  const format = $derived(exports.reportFormat(kind));
  const pending = $derived(exports.pending?.kind === kind ? exports.pending : null);
  const issueReason = $derived(disabledReason("issue_report"));
  const copyReason = $derived(disabledReason("save_copy"));

  interface Check {
    id: string;
    ok: boolean;
    text: string;
    action?: { label: string; run: () => void };
  }

  function fieldName(field: ReportField): string {
    switch (field) {
      case "part_number":
        return m.field_part_number();
      case "report_number":
        return m.field_report_number();
      case "company_name":
        return m.field_company_name();
      case "inspected_by":
        return m.field_inspected_by();
    }
  }

  const goMeasure = {
    label: m.report_go_measure(),
    run: () => {
      measureUi.onlyMissing = true;
      view.set("measure");
    },
  };
  const goInfo = {
    label: m.report_go_info(),
    run: () => {
      view.openFile("info");
    },
  };

  const items = $derived.by((): Check[] => {
    const c = checks;
    if (c === null) {
      return [];
    }
    const list: Check[] = [];
    if (kind === "check_sheet") {
      list.push({ id: "blank", ok: true, text: m.report_check_blank_sheet() });
    } else if (c.samples === 0) {
      list.push({
        id: "values",
        ok: false,
        text: m.report_check_no_parts(),
        action: { ...goMeasure, run: () => view.set("measure") },
      });
    } else if (c.missing_values > 0) {
      list.push({
        id: "values",
        ok: false,
        text: m.report_check_missing({ count: String(c.missing_values) }),
        action: goMeasure,
      });
    } else {
      list.push({ id: "values", ok: true, text: m.report_check_complete() });
    }
    if (c.not_evaluated > 0) {
      list.push({
        id: "not-evaluated",
        ok: false,
        text: m.report_check_not_evaluated({ count: String(c.not_evaluated) }),
      });
    }
    if (c.draft_rows > 0) {
      list.push({
        id: "draft",
        ok: false,
        text: m.report_check_draft({ count: String(c.draft_rows) }),
      });
    }
    if (c.changed_rows > 0) {
      list.push({
        id: "changed",
        ok: false,
        text: m.report_check_changed({ count: String(c.changed_rows) }),
      });
    }
    list.push(
      c.missing_fields.length > 0
        ? {
            id: "fields",
            ok: false,
            text: m.report_check_fields_missing({
              fields: c.missing_fields.map(fieldName).join(", "),
            }),
            action: goInfo,
          }
        : { id: "fields", ok: true, text: m.report_check_fields_ok() },
    );
    return list;
  });

  const primaryClass =
    "min-h-10 rounded-lg bg-accent-strong px-4 font-semibold text-accent-ink hover:opacity-90 aria-disabled:opacity-45";
  const secondaryClass =
    "min-h-10 rounded-lg border border-border-strong bg-surface px-4 text-text hover:bg-hover aria-disabled:opacity-45";
  // A link inside the text, still at least 32 px high (M3 decision 28).
  const linkClass =
    "inline-flex min-h-8 items-center px-1 font-medium text-accent-strong underline-offset-2 hover:underline";
</script>

<Pane title={m.report_pane_title()} label={m.report_pane_title()} testid="report-pane">
  <!-- Fills the scrolling body of the pane, so the actions sit at its foot. -->
  <div class="flex min-h-full flex-col">
    <ul class="flex flex-col" data-testid="report-checks">
      {#each items as item (item.id)}
        <li
          class="flex gap-2.5 border-b border-border px-3.5 py-3 text-sm leading-snug text-text"
          data-check={item.id}
          data-ok={item.ok}
        >
          {#if item.ok}
            <!-- Check mark: done (shape plus color, NFR-UX-05). -->
            <svg
              class="mt-0.5 size-4 shrink-0 text-success"
              viewBox="0 0 16 16"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M2.5 8.5 6.5 12.5 13.5 3.5"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {:else}
            <!-- Triangle with "!": look at this before issuing. -->
            <svg
              class="mt-0.5 size-4 shrink-0 text-warning"
              viewBox="0 0 16 16"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M8 1.5 15 14H1L8 1.5Z"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linejoin="round"
              />
              <path
                d="M8 6.5v3.5"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
              <circle cx="8" cy="12" r="0.9" fill="currentColor" />
            </svg>
          {/if}
          <span>
            {item.text}
            {#if item.action}
              <button type="button" class={linkClass} onclick={item.action.run}
                >{item.action.label}</button
              >
            {/if}
          </span>
        </li>
      {/each}
    </ul>
    {#if store.checksError}
      <p class="px-4 py-3 text-sm text-danger" role="alert">
        {commandErrorMessage(store.checksError)}
      </p>
    {/if}

    <div class="mt-auto flex flex-col gap-2 border-t border-border p-4">
      {#if pending}
        <div
          class="flex flex-col gap-2 rounded-lg border border-warning bg-bg p-3 text-sm"
          role="alertdialog"
          aria-labelledby="report-warnings-title"
        >
          <p id="report-warnings-title" class="font-semibold">
            <span aria-hidden="true">⚠</span>
            {m.export_warnings_title()}
          </p>
          <ul class="list-disc pl-5">
            {#if pending.warnings.draft_limits}
              <li>{m.export_warning_draft()}</li>
            {/if}
            {#if pending.warnings.open_results}
              <li>{m.export_warning_open()}</li>
            {/if}
            {#if pending.warnings.limits_changed}
              <li>{m.export_warning_changed()}</li>
            {/if}
          </ul>
          <div class="flex flex-wrap gap-2">
            <button
              type="button"
              class={primaryClass}
              onclick={() => void exports.confirmPending()}
              {@attach (el: HTMLButtonElement) => {
                el.focus();
              }}>{m.export_warnings_confirm()}</button
            >
            <button
              type="button"
              class={secondaryClass}
              onclick={() => {
                exports.dismissPending();
              }}>{m.export_warnings_back()}</button
            >
          </div>
        </div>
      {:else if exports.job(format)?.state === "running"}
        <button type="button" class={secondaryClass} onclick={() => void exports.cancel(format)}
          >{m.export_cancel()}</button
        >
      {:else}
        <button
          type="button"
          class={primaryClass}
          title={issueReason ?? m.cmd_issue_report_desc()}
          aria-disabled={issueReason !== null ? "true" : undefined}
          data-action="issue_report"
          onclick={() => {
            runCommand("issue_report");
          }}>{m.report_issue_button()}</button
        >
        <button
          type="button"
          class={secondaryClass}
          title={copyReason ?? m.cmd_save_copy_desc()}
          aria-disabled={copyReason !== null ? "true" : undefined}
          data-action="save_copy"
          onclick={() => {
            runCommand("save_copy");
          }}>{m.report_save_copy_button()}</button
        >
      {/if}
      <JobStatus {format} store={exports} />
      {#if store.print.state === "running"}
        <p class="text-sm text-text-muted" aria-live="polite">{m.report_print_running()}</p>
      {:else if store.print.state === "done"}
        <p class="text-sm text-success" aria-live="polite">
          <span aria-hidden="true">✓</span>
          {m.report_print_done({ file: store.print.fileName })}
        </p>
      {:else if store.print.state === "failed"}
        <p class="text-sm text-danger" role="alert">
          <span aria-hidden="true">⚠</span>
          {m.report_print_failed({ message: commandErrorMessage(store.print.error) })}
        </p>
      {/if}
      <p class="text-xs text-text-muted">{m.report_issue_hint()}</p>
    </div>
  </div>
</Pane>
