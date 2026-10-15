<script lang="ts">
  /**
   * Tolerance rules of the open project (T2.8, M2 decision 2, FR-TOL-06, FR-TOL-07, FR-TOL-09),
   * shown in the dialogs of the Drawing tab (M3 decision 21, T3.16): `decimal_rules` holds the
   * decimal place rules and the rounding after unit conversion, `company_tables` the drawing
   * rule and the custom tables with their import. The general tolerance is a field on the Home
   * and Drawing tabs. Rust lists the tables and classes, checks every value and every imported
   * file; each change is one undoable command that carries only what changed:
   * `update_tolerance_settings` for drawing rule and rounding, `add_decimal_rule` and
   * `remove_decimal_rule` for decimal place rules (MF F4).
   */
  import { m, tableImportText } from "$lib/i18n";
  import ErrorDetails from "./ErrorDetails.svelte";
  import type { DecimalPlaceRule, TableClass, ToleranceTableInfo } from "$lib/ipc/bindings";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";

  interface Props {
    section: "decimal_rules" | "company_tables";
    project?: ProjectStore;
    tolerance?: ToleranceStore;
  }

  let { section, project = appProject, tolerance = appTolerance }: Props = $props();

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
    // Rust keeps the rules in order of places; it refuses a second rule for the same places.
    if (await tolerance.addDecimalRule(rule)) {
      newPlaces = "";
      newTolerance = "";
    }
  }

  function setRule(count: number, tolerance_: string) {
    void tolerance.replaceDecimalRule({ places: count, tolerance: tolerance_.trim() });
  }

  function removeRule(count: number) {
    void tolerance.removeDecimalRule(count);
  }

  const fieldClass = "min-h-8 rounded border border-border-strong bg-surface px-2 py-1 text-text";
  const buttonClass =
    "min-h-8 rounded border border-border px-2 py-1 text-xs hover:bg-hover disabled:opacity-50";
</script>

<div class="flex flex-col gap-4 text-sm text-text" data-testid="tolerance-settings">
  {#if settings === null}
    <p class="text-text-muted">{m.settings_no_project()}</p>
  {:else if section === "company_tables"}
    {@const choice = settings.drawing_rule}
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex flex-wrap items-center gap-2">
        <span class="w-40">{m.tolerance_drawing_rule()}</span>
        <select
          class={fieldClass}
          data-setting="drawing_rule"
          value={choice?.table.id ?? NONE}
          onchange={(e) => {
            void tolerance.setSettings([
              { field: "drawing_rule", value: pickTable(e.currentTarget.value, choice) },
            ]);
          }}
        >
          <option value={NONE}>{m.tolerance_none()}</option>
          {#each tolerance.customTables as t (t.table.id)}
            <option value={t.table.id}>{tableLabel(t)}</option>
          {/each}
        </select>
      </label>
      {#if choice}
        <label class="flex items-center gap-2">
          <span>{m.tolerance_class()}</span>
          <select
            class={fieldClass}
            data-setting="drawing_rule_class"
            value={choice.class}
            onchange={(e) => {
              void tolerance.setSettings([
                { field: "drawing_rule", value: { ...choice, class: e.currentTarget.value } },
              ]);
            }}
          >
            {#each classesOf(choice) as cls (cls)}
              <option value={cls}>{cls}</option>
            {/each}
          </select>
        </label>
        {#if isDraft(choice)}
          <span class="draft text-warning" title={m.badge_draft_title()}>{m.badge_draft()}</span>
        {/if}
      {/if}
    </div>
    <p class="text-xs text-text-muted">{m.tolerance_drawing_rule_hint()}</p>

    <section class="flex flex-col gap-2" aria-labelledby="custom-tables-title">
      <h3 id="custom-tables-title" class="font-semibold">{m.tolerance_custom_tables()}</h3>
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
            ? m.tolerance_import_refused({ message: tableImportText(refusal.reason) })
            : m.tolerance_import_refused_line({
                line: String(refusal.line),
                message: tableImportText(refusal.reason),
              })}
        </p>
        <ErrorDetails detail={refusal.message} />
      {:else if tolerance.imported}
        <p class="text-xs" role="status" data-testid="import-status">
          ✓
          {tolerance.importUnchanged
            ? m.tolerance_import_unchanged({ table: tolerance.imported })
            : m.tolerance_imported({ table: tolerance.imported })}
        </p>
      {/if}
    </section>
  {:else}
    <section class="flex flex-col gap-2" aria-labelledby="decimal-rules-title">
      <h3 id="decimal-rules-title" class="font-semibold">{m.tolerance_decimal_rules()}</h3>
      <p class="text-xs text-text-muted">{m.tolerance_decimal_rules_hint()}</p>
      {#each settings.decimal_rules as rule (rule.places)}
        <div class="flex flex-wrap items-center gap-2">
          <span class="w-40 tabular-nums">{m.tolerance_places()}: {rule.places}</span>
          <label class="flex items-center gap-2">
            <span>{m.tolerance_value()}</span>
            <input
              class="{fieldClass} w-24 font-mono tabular-nums"
              type="text"
              spellcheck="false"
              value={rule.tolerance}
              onchange={(e) => {
                setRule(rule.places, e.currentTarget.value);
              }}
            />
          </label>
          <button
            type="button"
            class={buttonClass}
            aria-label={m.tolerance_remove_rule({ places: String(rule.places) })}
            onclick={() => {
              removeRule(rule.places);
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
          <span class="w-40">{m.tolerance_places()}</span>
          <input
            class="{fieldClass} w-16 font-mono tabular-nums"
            type="text"
            inputmode="numeric"
            bind:value={newPlaces}
          />
        </label>
        <label class="flex items-center gap-2">
          <span>{m.tolerance_value()}</span>
          <input
            class="{fieldClass} w-24 font-mono tabular-nums"
            type="text"
            spellcheck="false"
            bind:value={newTolerance}
          />
        </label>
        <button type="submit" class={buttonClass}>{m.tolerance_add_rule()}</button>
      </form>
    </section>

    <section class="flex flex-col gap-2" aria-labelledby="unit-rounding-title">
      <h3 id="unit-rounding-title" class="font-semibold">{m.tolerance_rounding()}</h3>
      {#each [{ key: "mm_places", label: m.tolerance_mm_places() }, { key: "inch_places", label: m.tolerance_inch_places() }] as const as row (row.key)}
        <label class="flex flex-wrap items-center gap-2">
          <span class="w-40">{row.label}</span>
          <input
            class="{fieldClass} w-16 font-mono tabular-nums"
            type="text"
            inputmode="numeric"
            value={String(settings.unit_rounding[row.key])}
            onchange={(e) => {
              const count = places(e.currentTarget.value);
              placesInvalid = count === null;
              if (count === null) {
                return;
              }
              void tolerance.setSettings([{ field: row.key, value: count }]);
            }}
          />
        </label>
      {/each}
    </section>
    {#if placesInvalid}
      <p class="text-xs text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {m.tolerance_places_invalid()}
      </p>
    {/if}
  {/if}
</div>

<style>
  .draft {
    border: 1px dashed currentColor;
    border-radius: 3px;
    padding: 0 0.25rem;
    font-size: 0.6875rem;
    font-weight: 600;
  }
</style>
