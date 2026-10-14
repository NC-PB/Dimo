<script lang="ts">
  /**
   * File > Options (ADR 0007, M3 decision 21, T3.16): the personal settings of the user, stored
   * by Rust in the app config directory and never in a project. Theme (D-51), language
   * (FR-SET-04), user name for the audit log (D-27), company name for new projects (M3 decision
   * 15) and the export defaults (D-32, D-33, M3 decision 12). Project settings live on the
   * Drawing tab and in File > Info.
   */
  import { LOCALE_NAMES, commandErrorMessage, locales, m, themeLabel } from "$lib/i18n";
  import type {
    ExportPreferences,
    PdfBalloons,
    ReportFormat,
    Language,
    Theme,
  } from "$lib/ipc/bindings";
  import { baseLocale, extractLocaleFromNavigator } from "$lib/paraglide/runtime.js";
  import { settingsStore as appSettings, type SettingsStore } from "$lib/stores/settings.svelte";

  interface Props {
    settingsStore?: SettingsStore;
  }

  let { settingsStore = appSettings }: Props = $props();

  const THEMES: readonly Theme[] = ["system", "light", "dark"];
  /** Select value for "follow the system language". */
  const SYSTEM = "system";
  /** The language the operating system asks for, as the app would use it. */
  const systemLocale = extractLocaleFromNavigator() ?? baseLocale;
  const REPORT_LANGUAGES: readonly { value: Language; label: string }[] = [
    { value: "en", label: LOCALE_NAMES.en },
    { value: "de", label: LOCALE_NAMES.de },
  ];
  const PDF_MODES: readonly { value: PdfBalloons; label: () => string }[] = [
    { value: "page_content", label: () => m.export_pdf_page_content() },
    { value: "annotations", label: () => m.export_pdf_annotations() },
  ];
  const FORMATS: readonly { value: ReportFormat; label: () => string }[] = [
    { value: "pdf", label: () => m.export_format_pdf() },
    { value: "xlsx", label: () => m.export_format_xlsx() },
  ];
  /** Bounds of the blank check sheet columns (M3 decision 12); Rust clamps them too. */
  const MIN_BLANK = 1;
  const MAX_BLANK = 10;

  const settings = $derived(settingsStore.current);
  const prefs = $derived(settings.export);
  const chosenLocale = $derived(settings.locale ?? SYSTEM);

  function setPrefs(change: Partial<ExportPreferences>): void {
    void settingsStore.update({ export: { ...prefs, ...change } });
  }

  function setBlankColumns(text: string): void {
    const value = Number(text);
    if (Number.isInteger(value)) {
      setPrefs({ blank_columns: Math.min(MAX_BLANK, Math.max(MIN_BLANK, value)) });
    }
  }

  const fieldClass =
    "min-h-8 w-full max-w-72 rounded-md border border-border-strong bg-surface px-2 py-1 text-text";
  const boxClass = "flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 shadow-sm";
  const gridClass =
    "grid grid-cols-[minmax(8rem,12rem)_minmax(0,1fr)] items-center gap-x-3 gap-y-2";
  const headingClass = "font-heading text-base font-semibold";
</script>

<div class="flex max-w-3xl flex-col gap-5" data-testid="options-pane">
  <div>
    <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.options_title()}</h1>
    <p class="mt-1 text-text-muted">{m.options_lead()}</p>
  </div>

  {#if settingsStore.error}
    <p class="text-danger" role="alert">
      <span aria-hidden="true">⚠</span>
      {commandErrorMessage(settingsStore.error)}
    </p>
  {/if}

  <section class={boxClass} aria-labelledby="options-appearance">
    <h2 id="options-appearance" class={headingClass}>{m.settings_appearance()}</h2>
    <div class={gridClass}>
      <label for="options-theme" class="text-text-muted">{m.theme_label()}</label>
      <select
        id="options-theme"
        class={fieldClass}
        value={settings.theme}
        onchange={(e) => {
          const theme = THEMES.find((t) => t === e.currentTarget.value);
          if (theme) {
            void settingsStore.update({ theme });
          }
        }}
      >
        {#each THEMES as theme (theme)}
          <option value={theme}>{themeLabel(theme)}</option>
        {/each}
      </select>
      <label for="options-language" class="text-text-muted">{m.language_label()}</label>
      <div class="flex flex-col gap-1">
        <select
          id="options-language"
          class={fieldClass}
          aria-describedby="options-language-hint"
          value={chosenLocale}
          onchange={(e) => {
            const value = e.currentTarget.value;
            void settingsStore.setLocale(value === SYSTEM ? null : value);
          }}
        >
          <option value={SYSTEM}>{m.language_system()} ({LOCALE_NAMES[systemLocale]})</option>
          {#each locales as l (l)}
            <option value={l}>{LOCALE_NAMES[l]}</option>
          {/each}
        </select>
        <span id="options-language-hint" class="text-xs text-text-muted">{m.language_hint()}</span>
      </div>
    </div>
  </section>

  <section class={boxClass} aria-labelledby="options-you">
    <h2 id="options-you" class={headingClass}>{m.options_you()}</h2>
    <div class={gridClass}>
      <label for="options-user" class="text-text-muted">{m.user_name_label()}</label>
      <div class="flex flex-col gap-1">
        <input
          id="options-user"
          class={fieldClass}
          type="text"
          autocomplete="off"
          spellcheck="false"
          aria-describedby="options-user-hint"
          placeholder={settingsStore.osUserName}
          value={settings.user_name}
          onchange={(e) => void settingsStore.update({ user_name: e.currentTarget.value })}
        />
        <span id="options-user-hint" class="text-xs text-text-muted">
          {m.user_name_hint({ name: settingsStore.osUserName })}
        </span>
      </div>
      <label for="options-company" class="text-text-muted">{m.company_name_label()}</label>
      <div class="flex flex-col gap-1">
        <input
          id="options-company"
          class={fieldClass}
          type="text"
          autocomplete="off"
          spellcheck="false"
          aria-describedby="options-company-hint"
          value={settings.company_name}
          onchange={(e) => void settingsStore.update({ company_name: e.currentTarget.value })}
        />
        <span id="options-company-hint" class="text-xs text-text-muted"
          >{m.company_name_hint()}</span
        >
      </div>
    </div>
  </section>

  <section class={boxClass} aria-labelledby="options-export">
    <h2 id="options-export" class={headingClass}>{m.options_export()}</h2>
    <p class="text-xs text-text-muted">{m.options_export_hint()}</p>
    <div class={gridClass}>
      <label for="options-report-language" class="text-text-muted">{m.export_language()}</label>
      <select
        id="options-report-language"
        class={fieldClass}
        value={prefs.language}
        onchange={(e) => {
          const value = REPORT_LANGUAGES.find((l) => l.value === e.currentTarget.value)?.value;
          if (value) {
            setPrefs({ language: value });
          }
        }}
      >
        {#each REPORT_LANGUAGES as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <label for="options-check-sheet" class="text-text-muted">{m.export_check_sheet_title()}</label
      >
      <select
        id="options-check-sheet"
        class={fieldClass}
        value={prefs.check_sheet_format}
        onchange={(e) => {
          const value = FORMATS.find((f) => f.value === e.currentTarget.value)?.value;
          if (value) {
            setPrefs({ check_sheet_format: value });
          }
        }}
      >
        {#each FORMATS as option (option.value)}
          <option value={option.value}>{option.label()}</option>
        {/each}
      </select>
      <label for="options-report" class="text-text-muted"
        >{m.export_inspection_report_title()}</label
      >
      <select
        id="options-report"
        class={fieldClass}
        value={prefs.inspection_report_format}
        onchange={(e) => {
          const value = FORMATS.find((f) => f.value === e.currentTarget.value)?.value;
          if (value) {
            setPrefs({ inspection_report_format: value });
          }
        }}
      >
        {#each FORMATS as option (option.value)}
          <option value={option.value}>{option.label()}</option>
        {/each}
      </select>
      <label for="options-blank" class="text-text-muted">{m.export_blank_columns()}</label>
      <input
        id="options-blank"
        type="number"
        class="{fieldClass} max-w-24 font-mono"
        min={MIN_BLANK}
        max={MAX_BLANK}
        step="1"
        value={prefs.blank_columns}
        onchange={(e) => {
          setBlankColumns(e.currentTarget.value);
          e.currentTarget.value = String(prefs.blank_columns);
        }}
      />
      <label for="options-pdf" class="text-text-muted">{m.export_pdf_balloons()}</label>
      <select
        id="options-pdf"
        class={fieldClass}
        value={prefs.pdf_balloons}
        onchange={(e) => {
          const value = PDF_MODES.find((p) => p.value === e.currentTarget.value)?.value;
          if (value) {
            setPrefs({ pdf_balloons: value });
          }
        }}
      >
        {#each PDF_MODES as option (option.value)}
          <option value={option.value}>{option.label()}</option>
        {/each}
      </select>
    </div>
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
  </section>

  <p class="text-xs text-text-muted">{m.options_not_in_project()}</p>
</div>
