<script lang="ts">
  import { LOCALE_NAMES, commandErrorMessage, m } from "$lib/i18n";
  import type { ExportFormat, PdfBalloons, ReportLanguage } from "$lib/ipc/bindings";
  import { EXPORT_FORMATS, exportStore } from "$lib/stores/export.svelte";
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

  const prefs = $derived(settingsStore.current.export);
  const open = $derived(projectStore.project !== null);
  const locked = $derived(projectStore.project?.numbering.lock != null);

  function title(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_title();
      case "csv":
        return m.export_csv_title();
      case "xlsx":
        return m.export_xlsx_title();
    }
  }

  function description(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_description();
      case "csv":
        return m.export_csv_description();
      case "xlsx":
        return m.export_xlsx_description();
    }
  }

  function buttonLabel(format: ExportFormat): string {
    switch (format) {
      case "ballooned_pdf":
        return m.export_pdf_button();
      case "csv":
        return m.export_csv_button();
      case "xlsx":
        return m.export_xlsx_button();
    }
  }

  function setPrefs(change: Partial<typeof prefs>): void {
    void settingsStore.update({ export: { ...prefs, ...change } });
  }

  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-text";
</script>

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
      <div class="flex flex-col gap-1">
        <label class="flex items-center gap-2">
          <input type="checkbox" bind:checked={exportStore.issued} disabled={locked} />
          {m.export_issued()}
        </label>
        <p class="pl-6 text-xs text-text-muted">
          {locked ? m.export_locked() : m.export_issued_hint()}
        </p>
      </div>
    </fieldset>

    <ul class="flex max-w-2xl flex-col gap-3">
      {#each EXPORT_FORMATS as format (format)}
        {@const job = exportStore.job(format)}
        {@const startError = exportStore.startError[format]}
        <li class="flex flex-col gap-2 rounded border border-border bg-surface p-4">
          <div class="flex items-start gap-4">
            <div class="flex-1">
              <h2 class="font-semibold">{title(format)}</h2>
              <p class="text-text-muted">{description(format)}</p>
            </div>
            <button
              type="button"
              class="shrink-0 rounded bg-accent px-3 py-1.5 text-accent-text hover:opacity-90 disabled:opacity-40"
              disabled={exportStore.busy(format)}
              onclick={() => void exportStore.start(format)}
            >
              {buttonLabel(format)}
            </button>
          </div>
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
            {:else if job?.state === "failed"}
              <p class="text-danger" role="alert">
                <span aria-hidden="true">⚠</span>
                {commandErrorMessage(job.error)}
              </p>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
