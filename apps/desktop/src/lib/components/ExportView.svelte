<script lang="ts">
  import { fieldClass } from "$lib/ui/fields";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import JobStatus from "$lib/components/JobStatus.svelte";
  import { LOCALE_NAMES, m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import type { ExportFormat, PdfBalloons, Language } from "$lib/ipc/bindings";
  import { EXPORT_FORMATS, exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { view } from "$lib/stores/view.svelte";

  /**
   * File > Export (T1.9, T3.19): the ballooned PDF and the characteristic list as CSV or Excel,
   * with their options. The check sheet and the inspection report are on the Report tab, with
   * the preview, the checks before issuing and Print.
   */
  const LANGUAGES: readonly { value: Language; label: string }[] = [
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
  // The ballooned PDF of a photo or scan follows with R.21; lists work for every drawing.
  const imageDrawing = $derived.by(() => {
    const p = projectStore.project;
    const revision = p?.revisions.find((r) => r.id === p.current_revision);
    return revision?.source.type === "image";
  });
  function unavailable(format: ExportFormat): boolean {
    return format === "ballooned_pdf" && imageDrawing;
  }

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

  function setPrefs(change: Partial<typeof prefs>): void {
    void settingsStore.update({ export: { ...prefs, ...change } });
  }

  const boxClass = "flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 shadow-sm";
  const inputClass = fieldClass();
  const buttonClass =
    "min-h-10 shrink-0 rounded-lg bg-accent-strong px-4 text-accent-ink hover:opacity-90 disabled:opacity-40";
  const linkClass = "text-sm font-medium text-accent-strong underline-offset-2 hover:underline";
</script>

<div class="flex max-w-3xl flex-col gap-5 text-sm text-text" data-testid="file-export">
  <div>
    <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.view_export()}</h1>
    <p class="mt-1 text-text-muted">
      {m.file_export_lead()}
      <button
        type="button"
        class={linkClass}
        onclick={() => {
          view.setTab("report");
        }}>{m.file_export_to_report()}</button
      >
    </p>
  </div>

  {#if !open}
    <EmptyState
      icon={icons.report}
      title={m.empty_no_project_title()}
      text={m.empty_report_no_project()}
      actions={[{ command: "new_project" }, { command: "open_project" }]}
    />
  {:else}
    {#if (projectStore.project?.characteristics.length ?? 0) === 0}
      <div class="rounded-xl border border-border bg-surface">
        <EmptyState
          size="inline"
          icon={icons.report}
          title={m.empty_report_no_chars_title()}
          text={m.empty_report_no_chars_text()}
          actions={[{ command: "place_tool" }]}
        />
      </div>
    {/if}
    <fieldset class={boxClass}>
      <legend class="px-1 font-heading text-base font-semibold">{m.export_options()}</legend>
      <label class="flex flex-wrap items-center gap-2">
        <span class="w-48">{m.export_language()}</span>
        <select
          class={inputClass}
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

    <h2 class="font-heading text-base font-semibold">{m.export_lists()}</h2>
    <ul class="flex flex-col gap-3">
      {#each EXPORT_FORMATS as format (format)}
        <li class={boxClass}>
          <div class="flex items-start gap-4">
            <div class="flex-1">
              <h3 class="font-semibold">{title(format)}</h3>
              <p class="text-text-muted">{description(format)}</p>
              {#if unavailable(format)}
                <p class="mt-1 text-text-muted" data-testid="export-image-note">
                  {m.export_pdf_image_note()}
                </p>
              {/if}
            </div>
            <button
              type="button"
              class={buttonClass}
              disabled={exportStore.busy(format) || unavailable(format)}
              onclick={() => void exportStore.start(format)}
            >
              {buttonLabel(format)}
            </button>
          </div>
          <JobStatus {format} />
        </li>
      {/each}
    </ul>
  {/if}
</div>
