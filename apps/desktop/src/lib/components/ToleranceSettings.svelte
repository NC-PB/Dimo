<script lang="ts">
  /**
   * Tolerance settings of the open project (T2.8, M2 decision 2, FR-TOL-06, FR-TOL-07,
   * FR-TOL-09): general standard and class, drawing rule, decimal place rules, rounding after
   * unit conversion, and custom tables. Rust lists the tables and classes, checks every value
   * and every imported file; each change is one undoable `set_tolerance_settings` command.
   */
  import { m } from "$lib/i18n";
  import type { DecimalPlaceRule, TableClass, ToleranceTableInfo } from "$lib/ipc/bindings";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";

  interface Props {
    project?: ProjectStore;
    tolerance?: ToleranceStore;
  }

  let { project = appProject, tolerance = appTolerance }: Props = $props();

  const settings = $derived(tolerance.settings);
  /** Select value for "no table". */
  const NONE = "";

  // The table list follows the project: an import, undo or another project.
  $effect(() => {
    void project.session;
    void project.project?.settings.tolerance;
    void tolerance.refreshTables();
  });

  function tableLabel(t: ToleranceTableInfo): string {
    return `${t.title} (${t.table.id} v${String(t.table.version)})`;
  }

  function classesOf(choice: TableClass | null): string[] {
    if (choice === null) {
      return [];
    }
    return tolerance.tables.find((t) => t.table.id === choice.table.id)?.classes ?? [];
  }

  function isDraft(choice: TableClass): boolean {
    return tolerance.tables.some((t) => t.table.id === choice.table.id && t.draft);
  }

  /** The setting for table `id` with its first class, `null` for {@link NONE}. */
  function pickTable(id: string, current: TableClass | null): TableClass | null {
    const table = tolerance.tables.find((t) => t.table.id === id);
    if (table === undefined) {
      return null;
    }
    const keep = current?.table.id === id && table.classes.includes(current.class);
    return {
      table: table.table,
      class: keep && current ? current.class : (table.classes[0] ?? ""),
    };
  }

  /** Whole numbers only; anything else is `null` and not sent (Rust checks the range). */
  function places(text: string): number | null {
    return /^\d+$/.test(text.trim()) ? Number(text.trim()) : null;
  }

  /** True while the last typed number of places is not a whole number. */
  let placesInvalid = $state(false);
  let newPlaces = $state("");
  let newTolerance = $state("");

  async function addRule() {
    if (settings === null || newPlaces.trim() === "" || newTolerance.trim() === "") {
      return;
    }
    const count = places(newPlaces);
    placesInvalid = count === null;
    if (count === null) {
      return;
    }
    const rule: DecimalPlaceRule = { places: count, tolerance: newTolerance.trim() };
    // Rust wants the rules in order of places; it refuses duplicates.
    const rules = [...settings.decimal_rules, rule].sort((a, b) => a.places - b.places);
    if (await tolerance.setSettings({ decimal_rules: rules })) {
      newPlaces = "";
      newTolerance = "";
    }
  }

  function setRule(index: number, tolerance_: string) {
    if (settings === null) {
      return;
    }
    const rules = settings.decimal_rules.map((r, i) =>
      i === index ? { ...r, tolerance: tolerance_.trim() } : r,
    );
    void tolerance.setSettings({ decimal_rules: rules });
  }

  function removeRule(index: number) {
    if (settings !== null) {
      void tolerance.setSettings({
        decimal_rules: settings.decimal_rules.filter((_, i) => i !== index),
      });
    }
  }

  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-text";
  const sectionClass = "flex max-w-2xl flex-col gap-3 rounded border border-border bg-surface p-4";
  const buttonClass =
    "rounded border border-border px-2 py-1 text-xs hover:bg-surface-raised disabled:opacity-50";
</script>

<fieldset class={sectionClass} data-testid="tolerance-settings">
  <legend class="px-1 font-semibold">{m.settings_tolerance()}</legend>
  {#if settings === null}
    <p class="text-text-muted">{m.settings_no_project()}</p>
  {:else}
    <p class="text-xs text-text-muted">{m.tolerance_settings_hint()}</p>

    {#each [{ key: "general", label: m.tolerance_general(), choice: settings.general, options: tolerance.generalTables }, { key: "drawing_rule", label: m.tolerance_drawing_rule(), choice: settings.drawing_rule, options: tolerance.customTables }] as row (row.key)}
      <div class="flex flex-wrap items-center gap-2">
        <label class="flex flex-wrap items-center gap-2">
          <span class="w-48">{row.label}</span>
          <select
            class={fieldClass}
            data-setting={row.key}
            value={row.choice?.table.id ?? NONE}
            onchange={(e) => {
              const next = pickTable(e.currentTarget.value, row.choice);
              void tolerance.setSettings(
                row.key === "general" ? { general: next } : { drawing_rule: next },
              );
            }}
          >
            <option value={NONE}>{m.tolerance_none()}</option>
            {#each row.options as t (t.table.id)}
              <option value={t.table.id}>{tableLabel(t)}</option>
            {/each}
          </select>
        </label>
        {#if row.choice}
          <label class="flex items-center gap-2">
            <span>{m.tolerance_class()}</span>
            <select
              class={fieldClass}
              data-setting="{row.key}_class"
              value={row.choice.class}
              onchange={(e) => {
                const next = row.choice ? { ...row.choice, class: e.currentTarget.value } : null;
                void tolerance.setSettings(
                  row.key === "general" ? { general: next } : { drawing_rule: next },
                );
              }}
            >
              {#each classesOf(row.choice) as cls (cls)}
                <option value={cls}>{cls}</option>
              {/each}
            </select>
          </label>
          {#if isDraft(row.choice)}
            <span class="draft text-warning" title={m.badge_draft_title()}>{m.badge_draft()}</span>
          {/if}
        {/if}
      </div>
    {/each}

    <fieldset class="flex flex-col gap-2">
      <legend class="font-semibold">{m.tolerance_decimal_rules()}</legend>
      <p class="text-xs text-text-muted">{m.tolerance_decimal_rules_hint()}</p>
      {#each settings.decimal_rules as rule, i (rule.places)}
        <div class="flex flex-wrap items-center gap-2">
          <span class="w-48 tabular-nums">{m.tolerance_places()}: {rule.places}</span>
          <label class="flex items-center gap-2">
            <span>{m.tolerance_value()}</span>
            <input
              class="{fieldClass} w-24 tabular-nums"
              type="text"
              spellcheck="false"
              value={rule.tolerance}
              onchange={(e) => {
                setRule(i, e.currentTarget.value);
              }}
            />
          </label>
          <button
            type="button"
            class={buttonClass}
            aria-label={m.tolerance_remove_rule({ places: String(rule.places) })}
            onclick={() => {
              removeRule(i);
            }}>✕</button
          >
        </div>
      {/each}
      <form
        class="flex flex-wrap items-center gap-2"
        onsubmit={(e) => {
          e.preventDefault();
          void addRule();
        }}
      >
        <label class="flex items-center gap-2">
          <span class="w-48">{m.tolerance_places()}</span>
          <input
            class="{fieldClass} w-16 tabular-nums"
            type="text"
            inputmode="numeric"
            bind:value={newPlaces}
          />
        </label>
        <label class="flex items-center gap-2">
          <span>{m.tolerance_value()}</span>
          <input
            class="{fieldClass} w-24 tabular-nums"
            type="text"
            spellcheck="false"
            bind:value={newTolerance}
          />
        </label>
        <button type="submit" class={buttonClass}>{m.tolerance_add_rule()}</button>
      </form>
    </fieldset>

    <fieldset class="flex flex-col gap-2">
      <legend class="font-semibold">{m.tolerance_rounding()}</legend>
      {#each [{ key: "mm_places", label: m.tolerance_mm_places() }, { key: "inch_places", label: m.tolerance_inch_places() }] as const as row (row.key)}
        <label class="flex flex-wrap items-center gap-2">
          <span class="w-48">{row.label}</span>
          <input
            class="{fieldClass} w-16 tabular-nums"
            type="text"
            inputmode="numeric"
            value={String(settings.unit_rounding[row.key])}
            onchange={(e) => {
              const count = places(e.currentTarget.value);
              placesInvalid = count === null;
              if (count === null) {
                return;
              }
              void tolerance.setSettings({
                unit_rounding: { ...settings.unit_rounding, [row.key]: count },
              });
            }}
          />
        </label>
      {/each}
    </fieldset>
    {#if placesInvalid}
      <p class="text-xs text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {m.tolerance_places_invalid()}
      </p>
    {/if}

    <fieldset class="flex flex-col gap-2">
      <legend class="font-semibold">{m.tolerance_custom_tables()}</legend>
      {#if tolerance.customTables.length === 0}
        <p class="text-xs text-text-muted">{m.tolerance_no_custom()}</p>
      {:else}
        <ul class="flex flex-col gap-1">
          {#each tolerance.customTables as t (t.table.id)}
            <li class="flex items-center gap-2">
              <span>{tableLabel(t)}</span>
              {#if t.draft}
                <span class="draft text-warning" title={m.badge_draft_title()}
                  >{m.badge_draft()}</span
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <div>
        <button
          type="button"
          class={buttonClass}
          disabled={tolerance.busy}
          onclick={() => void tolerance.importTable()}>{m.tolerance_import()}</button
        >
      </div>
      {#if tolerance.importRefusal}
        {@const refusal = tolerance.importRefusal}
        <p class="text-xs text-danger" role="alert" data-testid="import-refusal">
          <span aria-hidden="true">⚠</span>
          {refusal.line === null
            ? m.tolerance_import_refused({ message: refusal.message })
            : m.tolerance_import_refused_line({
                line: String(refusal.line),
                message: refusal.message,
              })}
        </p>
      {:else if tolerance.imported}
        <p class="text-xs" role="status">✓ {m.tolerance_imported({ table: tolerance.imported })}</p>
      {/if}
    </fieldset>
  {/if}
</fieldset>

<style>
  .draft {
    border: 1px dashed currentColor;
    border-radius: 3px;
    padding: 0 0.25rem;
    font-size: 0.6875rem;
    font-weight: 600;
  }
</style>
