<script lang="ts">
  import { LOCALE_NAMES, commandErrorMessage, m } from "$lib/i18n";
  import type {
    ExportFormat,
    PdfBalloons,
    ReportFormat,
    ReportKind,
    ReportLanguage,
  } from "$lib/ipc/bindings";
  import { EXPORT_FORMATS, REPORT_KINDS, exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";

  const LANGUAGES: readonly { value: ReportLanguage; label: string }[] = [
    { value: "en", label: LOCALE_NAMES.en },
    { value: "de", label: LOCALE_NAMES.de },
  ];
  const PDF_MODES: readonly { value: PdfBalloons; label: () => string }[] = [
    { value: "page_content", label: () => m.export_pdf_page_content() },
    { value: "annotations", label: () => m.export_pdf_annotations() },
  ];
  const REPORT_FILE_FORMATS: readonly { value: ReportFormat; label: () => string }[] = [
    { value: "pdf", label: () => m.export_format_pdf() },
    { value: "xlsx", label: () => m.export_format_xlsx() },
  ];
  /** Bounds of the blank check sheet columns (M3 decision 12); Rust clamps them too. */
  const MIN_BLANK = 1;
  const MAX_BLANK = 10;

  const prefs = $derived(settingsStore.current.export);
  const open = $derived(projectStore.project !== null);
  const locked = $derived(projectStore.project?.numbering.lock != null);

  function title(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_title();
      case "csv":
        return m.export_csv_title();
      default:
        return m.export_xlsx_title();
    }
  }

  function description(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_description();
      case "csv":
        return m.export_csv_description();
      default:
        return m.export_xlsx_description();
    }
  }

  function buttonLabel(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_button();
      case "csv":
        return m.export_csv_button();
      default:
        return m.export_xlsx_button();
    }
  }

  function reportTitle(kind: ReportKind): string {
    return kind === "check_sheet"
      ? m.export_check_sheet_title()
      : m.export_inspection_report_title();
  }

  function reportDescription(kind: ReportKind): string {
    return kind === "check_sheet"
      ? m.export_check_sheet_description()
      : m.export_inspection_report_description();
  }

  function reportButton(kind: ReportKind): string {
    return kind === "check_sheet"
      ? m.export_check_sheet_button()
      : m.export_inspection_report_button();
  }

  function reportFileFormat(kind: ReportKind): ReportFormat {
    return kind === "check_sheet" ? prefs.check_sheet_format : prefs.inspection_report_format;
  }

  function setReportFileFormat(kind: ReportKind, format: ReportFormat): void {
    setPrefs(
      kind === "check_sheet"
        ? { check_sheet_format: format }
        : { inspection_report_format: format },
    );
  }

  function setBlankColumns(text: string): void {
    const value = Math.round(Number(text));
    if (Number.isFinite(value)) {
      setPrefs({ blank_columns: Math.min(MAX_BLANK, Math.max(MIN_BLANK, value)) });
    }
  }

  function setPrefs(change: Partial<typeof prefs>): void {
    void settingsStore.update({ export: { ...prefs, ...change } });
  }

  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-text";
  const buttonClass =
    "shrink-0 rounded bg-accent px-3 py-1.5 text-accent-text hover:opacity-90 disabled:opacity-40";
  const secondaryClass =
    "shrink-0 rounded border border-border px-3 py-1.5 text-text hover:bg-bg disabled:opacity-40";
</script>

{#snippet jobStatus(format: ExportFormat)}
  {@const job = exportStore.job(format)}
  {@const startError = exportStore.startError[format]}
  <div aria-live="polite">
    {#if exportStore.asking[format]}
      <p class="text-text-muted">{m.export_waiting()}</p>
    {:else if startError}
      <p class="text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {commandErrorMessage(startError)}
      </p>
    {:else if job?.state === "running"}
      {@const percent = String(Math.round(job.fraction * 100))}
      <div class="flex items-center gap-2">
        <progress
          class="h-2 flex-1"
          max="1"
          value={job.fraction}
          aria-label={m.export_progress_label()}
        ></progress>
        <span class="text-text-muted">{m.export_running({ percent })}</span>
      </div>
    {:else if job?.state === "done"}
      <p class="text-success">
        <span aria-hidden="true">✓</span>
        {m.export_done({ file: job.fileName })}
      </p>
      {#if job.openError}
        <p class="text-danger" role="alert">
          <span aria-hidden="true">⚠</span>
          {m.export_open_failed({ message: commandErrorMessage(job.openError) })}
        </p>
      {/if}
      {#if job.issued?.state === "recorded"}
        <p class="text-success">
          <span aria-hidden="true">✓</span>
          {m.export_issued_recorded()}
        </p>
      {:else if job.issued?.state === "project_changed"}
        <p class="text-danger" role="alert">
          <span aria-hidden="true">⚠</span>
          {m.export_issued_project_changed()}
        </p>
      {:else if job.issued?.state === "refused"}
        <p class="text-danger" role="alert">
          <span aria-hidden="true">⚠</span>
          {m.export_issued_refused({ message: commandErrorMessage(job.issued.error) })}
        </p>
      {/if}
    {:else if job?.state === "cancelled"}
      <p class="text-text-muted">
        <span aria-hidden="true">⊘</span>
        {m.export_cancelled()}
      </p>
    {:else if job?.state === "failed"}
      <p class="text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {commandErrorMessage(job.error)}
      </p>
    {/if}
  </div>
{/snippet}

<section
  class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto bg-bg p-6 text-sm text-text"
  aria-labelledby="export-heading"
>
  <h1 id="export-heading" class="text-lg font-semibold">{m.view_export()}</h1>

  {#if !open}
    <p class="text-text-muted">{m.export_no_project()}</p>
  {:else}
    <fieldset class="flex max-w-2xl flex-col gap-3 rounded border border-border bg-surface p-4">
      <legend class="px-1 font-semibold">{m.export_options()}</legend>
      <label class="flex flex-wrap items-center gap-2">
        <span class="w-48">{m.export_language()}</span>
        <select
          class={fieldClass}
          aria-describedby="export-language-hint"
          value={prefs.language}
          onchange={(e) => {
            const value = LANGUAGES.find((l) => l.value === e.currentTarget.value)?.value;
            if (value) {
              setPrefs({ language: value });
            }
          }}
        >
          {#each LANGUAGES as option (option.value)}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
        <span id="export-language-hint" class="text-xs text-text-muted">
          {m.export_language_hint()}
        </span>
      </label>
      <fieldset class="flex flex-wrap items-start gap-2">
        <legend class="float-left w-48">{m.export_pdf_balloons()}</legend>
        <div class="flex flex-col gap-1">
          {#each PDF_MODES as mode (mode.value)}
            <label class="flex items-center gap-2">
              <input
                type="radio"
                name="pdf-balloons"
                value={mode.value}
                checked={prefs.pdf_balloons === mode.value}
                onchange={() => {
                  setPrefs({ pdf_balloons: mode.value });
                }}
              />
              {mode.label()}
            </label>
          {/each}
        </div>
      </fieldset>
      <label class="flex flex-wrap items-center gap-2">
        <span class="w-48">{m.export_blank_columns()}</span>
        <input
          type="number"
          class="{fieldClass} w-20"
          min={MIN_BLANK}
          max={MAX_BLANK}
          step="1"
          aria-describedby="export-blank-hint"
          value={prefs.blank_columns}
          onchange={(e) => {
            setBlankColumns(e.currentTarget.value);
            e.currentTarget.value = String(prefs.blank_columns);
          }}
        />
        <span id="export-blank-hint" class="text-xs text-text-muted">
          {m.export_blank_columns_hint()}
        </span>
      </label>
      <label class="flex items-center gap-2">
        <input
          type="checkbox"
          checked={prefs.open_after_export}
          onchange={(e) => {
            setPrefs({ open_after_export: e.currentTarget.checked });
          }}
        />
        {m.export_open_after()}
      </label>
      <div class="flex flex-col gap-1">
        <label class="flex items-center gap-2">
          <input type="checkbox" bind:checked={exportStore.issued} />
          {m.export_issued()}
        </label>
        <p class="pl-6 text-xs text-text-muted">
          {locked ? m.export_locked() : m.export_issued_hint()}
        </p>
      </div>
    </fieldset>

    <h2 class="font-semibold">{m.export_reports()}</h2>
    <ul class="flex max-w-2xl flex-col gap-3">
      {#each REPORT_KINDS as kind (kind)}
        {@const format = exportStore.reportFormat(kind)}
        {@const pending = exportStore.pending?.kind === kind ? exportStore.pending : null}
        <li class="flex flex-col gap-2 rounded border border-border bg-surface p-4">
          <div class="flex items-start gap-4">
            <div class="flex flex-1 flex-col gap-2">
              <div>
                <h3 class="font-semibold">{reportTitle(kind)}</h3>
                <p class="text-text-muted">{reportDescription(kind)}</p>
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <span aria-hidden="true">{m.export_format()}</span>
                <select
                  class={fieldClass}
                  aria-label="{reportTitle(kind)}: {m.export_format()}"
                  value={reportFileFormat(kind)}
                  disabled={exportStore.busy(format)}
                  onchange={(e) => {
                    const value = REPORT_FILE_FORMATS.find(
                      (f) => f.value === e.currentTarget.value,
                    )?.value;
                    if (value) {
                      setReportFileFormat(kind, value);
                    }
                  }}
                >
                  {#each REPORT_FILE_FORMATS as option (option.value)}
                    <option value={option.value}>{option.label()}</option>
                  {/each}
                </select>
              </div>
            </div>
            {#if exportStore.job(format)?.state === "running"}
              <button
                type="button"
                class={secondaryClass}
                onclick={() => void exportStore.cancel(format)}
              >
                {m.export_cancel()}
              </button>
            {:else}
              <button
                type="button"
                class={buttonClass}
                disabled={exportStore.busy(format) || pending !== null}
                onclick={() => void exportStore.startReport(kind)}
              >
                {reportButton(kind)}
              </button>
            {/if}
          </div>
          {#if pending}
            <div
              class="flex flex-col gap-2 rounded border border-warning bg-bg p-3"
              role="alertdialog"
              aria-labelledby="warnings-{kind}"
            >
              <p id="warnings-{kind}" class="font-semibold">
                <span aria-hidden="true">⚠</span>
                {m.export_warnings_title()}
              </p>
              <ul class="list-disc pl-6">
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
              <div class="flex gap-2">
                <button
                  type="button"
                  class={buttonClass}
                  onclick={() => void exportStore.confirmPending()}
                  {@attach (el: HTMLButtonElement) => {
                    el.focus();
                  }}
                >
                  {m.export_warnings_confirm()}
                </button>
                <button
                  type="button"
                  class={secondaryClass}
                  onclick={() => {
                    exportStore.dismissPending();
                  }}
                >
                  {m.export_warnings_back()}
                </button>
              </div>
            </div>
          {/if}
          {@render jobStatus(format)}
        </li>
      {/each}
    </ul>

    <h2 class="font-semibold">{m.export_lists()}</h2>
    <ul class="flex max-w-2xl flex-col gap-3">
      {#each EXPORT_FORMATS as format (format)}
        <li class="flex flex-col gap-2 rounded border border-border bg-surface p-4">
          <div class="flex items-start gap-4">
            <div class="flex-1">
              <h3 class="font-semibold">{title(format)}</h3>
              <p class="text-text-muted">{description(format)}</p>
            </div>
            <button
              type="button"
              class={buttonClass}
              disabled={exportStore.busy(format)}
              onclick={() => void exportStore.start(format)}
            >
              {buttonLabel(format)}
            </button>
          </div>
          {@render jobStatus(format)}
        </li>
      {/each}
    </ul>
  {/if}
</section>
