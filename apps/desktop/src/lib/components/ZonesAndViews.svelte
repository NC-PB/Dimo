<script lang="ts">
  import { m } from "$lib/i18n";
  import {
    commands,
    type AxisScheme,
    type Sheet,
    type SheetView,
    type ZoneAxis,
    type ZoneGrid,
  } from "$lib/ipc/bindings";
  import { projectStore } from "$lib/stores/project.svelte";
  import { sheetRegions } from "$lib/stores/sheet-regions.svelte";
  import {
    MAX_DIVISIONS,
    labelsText,
    parseLabels,
    viewsCommand,
    zoneGridCommand,
  } from "$lib/zone-grid";

  /**
   * Zone grid and views of the shown sheet (T2.7, M2 decision 1, D-21): drawn by hand, used by
   * the numbering strategies. Every change is one undoable command; Rust validates it. Default
   * grids, axis labels and the scheme of each axis come from Rust (`zone_grid_form`, T2.7a).
   */
  interface Props {
    sheet: Sheet;
  }

  let { sheet }: Props = $props();

  const grid = $derived(sheet.zone_grid);

  type Axis = ZoneAxis;

  /** Scheme choices per axis, as `kind:reversed` values of the select. */
  const SCHEMES: Record<Axis, { value: string; scheme: AxisScheme; label: () => string }[]> = {
    columns: [
      {
        value: "numbers",
        scheme: { kind: "numbers", reversed: false },
        label: () => m.zones_columns_numbers(),
      },
      {
        value: "numbers_reversed",
        scheme: { kind: "numbers", reversed: true },
        label: () => m.zones_columns_numbers_reversed(),
      },
      {
        value: "letters",
        scheme: { kind: "letters", reversed: false },
        label: () => m.zones_columns_letters(),
      },
      {
        value: "letters_reversed",
        scheme: { kind: "letters", reversed: true },
        label: () => m.zones_columns_letters_reversed(),
      },
    ],
    rows: [
      {
        value: "letters",
        scheme: { kind: "letters", reversed: false },
        label: () => m.zones_rows_letters(),
      },
      {
        value: "letters_reversed",
        scheme: { kind: "letters", reversed: true },
        label: () => m.zones_rows_letters_reversed(),
      },
      {
        value: "numbers",
        scheme: { kind: "numbers", reversed: false },
        label: () => m.zones_rows_numbers(),
      },
      {
        value: "numbers_reversed",
        scheme: { kind: "numbers", reversed: true },
        label: () => m.zones_rows_numbers_reversed(),
      },
    ],
  };
  const CUSTOM = "custom";

  function labelsOf(g: ZoneGrid, axis: Axis): string[] {
    return axis === "columns" ? g.column_labels : g.row_labels;
  }

  /**
   * The label scheme of each axis of the shown grid as Rust reads it, `null` for labels typed by
   * hand; `undefined` until the first answer arrives.
   */
  let schemes = $state<{ columns: AxisScheme | null; rows: AxisScheme | null } | undefined>();

  $effect(() => {
    const g = grid;
    if (!g) {
      schemes = undefined;
      return;
    }
    let current = true;
    commands
      .zoneGridForm({ type: "describe", grid: g })
      .then((form) => {
        if (current) {
          schemes = { columns: form.column_scheme, rows: form.row_scheme };
        }
      })
      .catch(() => undefined);
    return () => {
      current = false;
    };
  });

  /** The select value of an axis: its scheme, `custom` for typed labels, empty while unknown. */
  function schemeValue(axis: Axis): string {
    if (schemes === undefined) {
      return "";
    }
    const scheme = schemes[axis];
    const found = SCHEMES[axis].find(
      (s) => scheme && s.scheme.kind === scheme.kind && s.scheme.reversed === scheme.reversed,
    );
    return found?.value ?? CUSTOM;
  }

  function setGrid(next: ZoneGrid | null): void {
    void projectStore.execute(zoneGridCommand(sheet, next));
  }

  /** Relabels one axis through Rust; without a scheme the axis keeps its current one. */
  async function relabel(
    g: ZoneGrid,
    axis: Axis,
    count: number,
    scheme: AxisScheme | null,
  ): Promise<void> {
    const form = await commands.zoneGridForm({ type: "axis", grid: g, axis, count, scheme });
    setGrid(form.grid);
  }

  async function addGrid(): Promise<void> {
    const form = await commands.zoneGridForm({ type: "default", size: sheet.size });
    setGrid(form.grid);
  }

  function onCount(event: Event & { currentTarget: HTMLInputElement }, axis: Axis): void {
    const g = grid;
    const count = Number(event.currentTarget.value);
    if (!g || !Number.isInteger(count) || count < 1 || count > MAX_DIVISIONS) {
      event.currentTarget.value = String(g ? labelsOf(g, axis).length : 1);
      return;
    }
    if (count !== labelsOf(g, axis).length) {
      void relabel(g, axis, count, null);
    }
  }

  function onScheme(event: Event & { currentTarget: HTMLSelectElement }, axis: Axis): void {
    const g = grid;
    const choice = SCHEMES[axis].find((s) => s.value === event.currentTarget.value);
    if (g && choice) {
      void relabel(g, axis, labelsOf(g, axis).length, choice.scheme);
    }
  }

  function onLabels(event: Event & { currentTarget: HTMLInputElement }, axis: Axis): void {
    const g = grid;
    if (!g) {
      return;
    }
    const labels = parseLabels(event.currentTarget.value);
    if (labels.length === 0) {
      event.currentTarget.value = labelsText(labelsOf(g, axis));
      return;
    }
    setGrid(axis === "columns" ? { ...g, column_labels: labels } : { ...g, row_labels: labels });
  }

  function setViews(views: SheetView[]): void {
    void projectStore.execute(viewsCommand(sheet, views));
  }

  function renameView(index: number, label: string): void {
    const views = sheet.views.map((v, i) => (i === index ? { ...v, label: label.trim() } : v));
    setViews(views);
  }

  function removeView(index: number): void {
    setViews(sheet.views.filter((_, i) => i !== index));
  }

  const buttonClass =
    "rounded border border-border px-2 py-1 text-sm text-text hover:bg-surface-raised aria-pressed:bg-accent aria-pressed:text-accent-text";
  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-sm text-text";
</script>

<fieldset class="flex flex-col gap-1.5">
  <legend class="mb-1 text-xs text-text-muted">{m.zones_title()}</legend>
  {#if grid}
    <div class="grid grid-cols-2 gap-1.5">
      <label class="flex flex-col gap-0.5 text-xs text-text-muted">
        {m.zones_columns()}
        <input
          type="number"
          min="1"
          max={MAX_DIVISIONS}
          step="1"
          class={fieldClass}
          value={grid.column_labels.length}
          onchange={(e) => {
            onCount(e, "columns");
          }}
        />
      </label>
      <label class="flex flex-col gap-0.5 text-xs text-text-muted">
        {m.zones_rows()}
        <input
          type="number"
          min="1"
          max={MAX_DIVISIONS}
          step="1"
          class={fieldClass}
          value={grid.row_labels.length}
          onchange={(e) => {
            onCount(e, "rows");
          }}
        />
      </label>
    </div>
    {#each ["columns", "rows"] as const as axis (axis)}
      <label class="flex flex-col gap-0.5 text-xs text-text-muted">
        {axis === "columns" ? m.zones_column_scheme() : m.zones_row_scheme()}
        <select
          class={fieldClass}
          value={schemeValue(axis)}
          onchange={(e) => {
            onScheme(e, axis);
          }}
        >
          {#each SCHEMES[axis] as option (option.value)}
            <option value={option.value}>{option.label()}</option>
          {/each}
          {#if schemeValue(axis) === CUSTOM}
            <option value={CUSTOM} disabled>{m.zones_scheme_custom()}</option>
          {/if}
        </select>
      </label>
      <label class="flex flex-col gap-0.5 text-xs text-text-muted">
        {axis === "columns" ? m.zones_column_labels() : m.zones_row_labels()}
        <input
          type="text"
          class={fieldClass}
          value={labelsText(labelsOf(grid, axis))}
          onchange={(e) => {
            onLabels(e, axis);
          }}
        />
      </label>
    {/each}
    <p class="text-xs text-text-muted">{m.zones_labels_hint()}</p>
    <div class="flex flex-wrap gap-1.5">
      <button
        type="button"
        class={buttonClass}
        aria-pressed={sheetRegions.tool === "zone_frame"}
        onclick={() => {
          sheetRegions.toggle("zone_frame");
        }}>{m.zones_draw_frame()}</button
      >
      <button
        type="button"
        class={buttonClass}
        onclick={() => {
          setGrid(null);
        }}>{m.zones_remove()}</button
      >
    </div>
  {:else}
    <p class="text-xs text-text-muted">{m.zones_none()}</p>
    <div class="flex flex-wrap gap-1.5">
      <button type="button" class={buttonClass} onclick={() => void addGrid()}
        >{m.zones_add()}</button
      >
      <button
        type="button"
        class={buttonClass}
        aria-pressed={sheetRegions.tool === "zone_frame"}
        onclick={() => {
          sheetRegions.toggle("zone_frame");
        }}>{m.zones_draw_frame()}</button
      >
    </div>
  {/if}
</fieldset>

<fieldset class="flex flex-col gap-1.5">
  <legend class="mb-1 text-xs text-text-muted">{m.views_title()}</legend>
  {#if sheet.views.length === 0}
    <p class="text-xs text-text-muted">{m.views_none()}</p>
  {:else}
    <ol class="flex flex-col gap-1">
      {#each sheet.views as view, index (index)}
        <li class="flex items-center gap-1.5">
          <input
            type="text"
            class="{fieldClass} min-w-0 flex-1"
            aria-label={m.views_label({ number: String(index + 1) })}
            placeholder={m.views_unnamed({ number: String(index + 1) })}
            value={view.label}
            onchange={(e) => {
              renameView(index, e.currentTarget.value);
            }}
          />
          <button
            type="button"
            class="rounded border border-border px-2 py-1 text-sm text-text hover:bg-surface-raised"
            aria-label={m.views_remove({ number: String(index + 1) })}
            title={m.views_remove({ number: String(index + 1) })}
            onclick={() => {
              removeView(index);
            }}>×</button
          >
        </li>
      {/each}
    </ol>
  {/if}
  <div>
    <button
      type="button"
      class={buttonClass}
      aria-pressed={sheetRegions.tool === "view"}
      onclick={() => {
        sheetRegions.toggle("view");
      }}>{m.views_draw()}</button
    >
  </div>
  {#if sheetRegions.tool !== null}
    <p class="text-xs text-text-muted" role="status">{m.regions_drawing_hint()}</p>
  {/if}
</fieldset>
