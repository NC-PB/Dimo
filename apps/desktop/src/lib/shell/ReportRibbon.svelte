<script lang="ts">
  import { LOCALE_NAMES, m } from "$lib/i18n";
  import type { ReportFormat, ReportLanguage } from "$lib/ipc/bindings";
  import { exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import CommandButton from "./CommandButton.svelte";
  import { FIELD_CLASS, LABEL_CLASS, fieldTitle } from "./field";
  import Ribbon from "./Ribbon.svelte";

  /**
   * Report tab (ADR 0007, mockup "Report tab"): the document, its options and the output. The
   * options are the remembered export options of the user (T1.9); Save copy and Issue report
   * run the same exports as the cards of the report page below. Preview and Print follow in
   * T3.19.
   */
  const FORMATS: readonly { value: ReportFormat; label: () => string }[] = [
    { value: "pdf", label: () => m.export_format_pdf() },
    { value: "xlsx", label: () => m.export_format_xlsx() },
  ];
  const LANGUAGES: readonly ReportLanguage[] = ["en", "de"];

  const prefs = $derived(settingsStore.current.export);
  const reason = $derived(projectStore.isOpen ? null : m.reason_no_project());
  const format = $derived(
    exportStore.document === "check_sheet"
      ? prefs.check_sheet_format
      : prefs.inspection_report_format,
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
    <label class={LABEL_CLASS} title={fieldTitle(m.field_report_format_desc(), reason)}>
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
    <label class={LABEL_CLASS} title={fieldTitle(m.field_report_language_desc(), reason)}>
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
  <div class="flex flex-col gap-0.5 self-end">
    <CommandButton id="open_after_export" />
  </div>
{/snippet}

{#snippet output()}
  <CommandButton id="save_copy" size="large" />
  <CommandButton id="issue_report" size="large" />
{/snippet}

<Ribbon
  groups={[
    { id: "document", label: m.ribbon_group_document(), content: documentGroup },
    { id: "options", label: m.ribbon_group_options(), content: options },
    { id: "output", label: m.ribbon_group_output(), content: output },
  ]}
/>
