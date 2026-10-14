<script lang="ts">
  import { LOCALE_NAMES, m } from "$lib/i18n";
  import type { ReportFormat, Language } from "$lib/ipc/bindings";
  import { exportStore } from "$lib/stores/export.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import CommandButton from "./CommandButton.svelte";
  import { FIELD_CLASS, LABEL_CLASS, fieldTitle } from "./field";
  import Ribbon from "./Ribbon.svelte";

  /**
   * Report tab (ADR 0007, mockup "Report tab"): the document, its options, the output and the
   * other exports. The options are the remembered export options of the user (T1.9); the work
   * area below shows the preview of what Save copy writes, Print opens it in the PDF viewer
   * (T3.19). Blank result columns apply to a check sheet of a project without parts.
   */
  /** Bounds of the blank check sheet columns (M3 decision 12); Rust refuses others. */
  const MIN_BLANK = 1;
  const MAX_BLANK = 10;
  const FORMATS: readonly { value: ReportFormat; label: () => string }[] = [
    { value: "pdf", label: () => m.export_format_pdf() },
    { value: "xlsx", label: () => m.export_format_xlsx() },
  ];
  const LANGUAGES: readonly Language[] = ["en", "de"];

  const prefs = $derived(settingsStore.current.export);
  const format = $derived(
    exportStore.document === "check_sheet"
      ? prefs.check_sheet_format
      : prefs.inspection_report_format,
  );

  const blankReason = $derived(
    exportStore.document === "check_sheet" ? null : m.reason_check_sheet_only(),
  );

  function setFormat(value: ReportFormat): void {
    void settingsStore.update({
      export: {
        ...prefs,
        ...(exportStore.document === "check_sheet"
          ? { check_sheet_format: value }
          : { inspection_report_format: value }),
      },
    });
  }
</script>

{#snippet documentGroup()}
  <CommandButton id="check_sheet" size="large" />
  <CommandButton id="inspection_report" size="large" />
{/snippet}

{#snippet options()}
  <div class="flex gap-2">
    <label class={LABEL_CLASS} title={m.field_report_format_desc()}>
      {m.export_format()}
      <select
        class="{FIELD_CLASS} min-w-32"
        data-field="report-format"
        value={format}
        onchange={(e) => {
          const value = FORMATS.find((f) => f.value === e.currentTarget.value)?.value;
          if (value) {
            setFormat(value);
          }
        }}
      >
        {#each FORMATS as option (option.value)}
          <option value={option.value}>{option.label()}</option>
        {/each}
      </select>
    </label>
    <label class={LABEL_CLASS} title={m.field_report_language_desc()}>
      {m.export_language()}
      <select
        class="{FIELD_CLASS} min-w-28"
        data-field="report-language"
        value={prefs.language}
        onchange={(e) => {
          const language = LANGUAGES.find((l) => l === e.currentTarget.value);
          if (language) {
            void settingsStore.update({ export: { ...prefs, language } });
          }
        }}
      >
        {#each LANGUAGES as language (language)}
          <option value={language}>{LOCALE_NAMES[language]}</option>
        {/each}
      </select>
    </label>
  </div>
  <label class={LABEL_CLASS} title={fieldTitle(m.export_blank_columns_hint(), blankReason)}>
    {m.export_blank_columns()}
    <input
      type="number"
      min={MIN_BLANK}
      max={MAX_BLANK}
      step="1"
      class="{FIELD_CLASS} w-16"
      data-field="blank-columns"
      disabled={blankReason !== null}
      value={prefs.blank_columns}
      onchange={(e) => {
        const text = e.currentTarget.value.trim();
        const value = Math.round(Number(text));
        if (text !== "" && Number.isFinite(value)) {
          const blank_columns = Math.min(MAX_BLANK, Math.max(MIN_BLANK, value));
          void settingsStore.update({ export: { ...prefs, blank_columns } });
        }
        e.currentTarget.value = String(prefs.blank_columns);
      }}
    />
  </label>
  <div class="flex flex-col gap-0.5 self-end">
    <CommandButton id="open_after_export" />
  </div>
{/snippet}

{#snippet output()}
  <CommandButton id="save_copy" size="large" />
  <CommandButton id="print" size="large" />
  <CommandButton id="issue_report" size="large" />
{/snippet}

{#snippet more()}
  <CommandButton id="more_exports" size="large" />
{/snippet}

<Ribbon
  groups={[
    { id: "document", label: m.ribbon_group_document(), content: documentGroup },
    { id: "options", label: m.ribbon_group_options(), content: options },
    { id: "output", label: m.ribbon_group_output(), content: output },
    { id: "more", label: m.ribbon_group_more(), content: more },
  ]}
/>
