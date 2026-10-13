<script lang="ts">
  import { commandTooltip, disabledReason, runCommand } from "$lib/commands.svelte";
  import { m, unitLabel } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import { scaleText, shownSheet } from "$lib/sheet-properties";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { measurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { toleranceStore } from "$lib/stores/tolerance.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { ACTUAL_SIZE_SCALE, zoomPercent } from "$lib/viewport/view-math";

  /**
   * Status bar (ADR 0007): sheet tabs, the active tool, counts (in Measure the totals of all
   * parts, from Rust), the general tolerance, unit and scale of the sheet on screen, and the
   * zoom with minus, slider, plus and percent.
   */
  const ZOOM_MIN = 10;
  const ZOOM_MAX = 400;

  const open = $derived(projectStore.isOpen);
  const sheetIndexes = $derived(Array.from({ length: documentStore.sheetCount }, (_, i) => i));
  const sheet = $derived(shownSheet());
  const measuring = $derived(view.current === "measure");
  const counts = $derived(measurementStore.state?.counts ?? null);
  const zoomReason = $derived(disabledReason("zoom_in"));
  const percent = $derived(zoomPercent(viewport.view.scale));

  const general = $derived.by(() => {
    const choice = toleranceStore.settings?.general ?? null;
    if (choice === null) {
      return m.tolerance_none();
    }
    const title = toleranceStore.tables.find((t) => t.table.id === choice.table.id)?.title;
    return `${title ?? choice.table.id} ${choice.class}`;
  });

  function zoomTo(value: number): void {
    const target = (value / 100) * ACTUAL_SIZE_SCALE;
    if (viewport.view.scale > 0) {
      viewport.zoomBy(target / viewport.view.scale);
    }
  }

  const zoomButton =
    "flex size-8 items-center justify-center rounded text-text-muted hover:bg-hover hover:text-text aria-disabled:opacity-45 aria-disabled:hover:bg-transparent";
</script>

<footer
  class="flex min-h-9 shrink-0 flex-wrap items-center gap-x-3 gap-y-0.5 border-t border-border bg-surface px-2 text-xs text-text-muted"
  aria-label={m.status_bar_label()}
>
  {#if documentStore.sheetCount > 0}
    <div class="-mt-px flex self-stretch" role="group" aria-label={m.status_sheets_label()}>
      {#each sheetIndexes as index (index)}
        {@const on = documentStore.sheet === index}
        <button
          type="button"
          class="min-h-8 rounded-b-md border border-t-0 border-b-[3px] border-border px-3 text-xs {on
            ? 'border-b-accent font-semibold text-text'
            : 'border-b-transparent text-text-muted hover:text-text'}"
          aria-current={on ? "page" : undefined}
          data-sheet={index}
          onclick={() => {
            documentStore.setSheet(index);
          }}
        >
          {m.status_sheet({ number: String(index + 1) })}
        </button>
      {/each}
    </div>
  {/if}

  {#if open && !measuring}
    <span
      class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 font-semibold text-text {balloonTools.tool ===
      'place'
        ? 'border-accent bg-accent-soft'
        : 'border-border'}"
      aria-label={m.status_tool_label()}
      data-testid="tool-chip"
      data-tool={balloonTools.tool}
    >
      {balloonTools.tool === "place" ? m.place_tool() : m.select_tool()}
    </span>
    <span>
      {m.status_characteristics({ count: String(projectStore.characteristics.length) })}
      {#if !selection.isEmpty}
        · {m.status_selected({ count: String(selection.size) })}
      {/if}
    </span>
  {:else if open && counts !== null}
    <span data-testid="measure-totals">
      {m.status_totals({
        pass: String(counts.pass),
        fail: String(counts.fail),
        missing: String(counts.missing),
      })}
    </span>
  {/if}

  <span class="flex-1"></span>

  {#if open}
    <span>{m.status_general({ value: general })}</span>
    {#if sheet}
      <span>· {unitLabel(sheet.unit)} · {scaleText(sheet.scale)}</span>
    {/if}
  {/if}

  <div class="flex items-center gap-1" role="group" aria-label={m.status_zoom()}>
    <button
      type="button"
      class={zoomButton}
      title={commandTooltip("zoom_out")}
      aria-label={m.zoom_out()}
      aria-disabled={zoomReason !== null ? "true" : undefined}
      onclick={() => {
        runCommand("zoom_out");
      }}
    >
      <icons.zoomOut size={16} aria-hidden="true" />
    </button>
    <input
      type="range"
      class="w-28"
      min={ZOOM_MIN}
      max={ZOOM_MAX}
      step="5"
      aria-label={m.zoom_level_label()}
      disabled={zoomReason !== null}
      value={Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, percent))}
      oninput={(e) => {
        zoomTo(Number(e.currentTarget.value));
      }}
    />
    <button
      type="button"
      class={zoomButton}
      title={commandTooltip("zoom_in")}
      aria-label={m.zoom_in()}
      aria-disabled={zoomReason !== null ? "true" : undefined}
      onclick={() => {
        runCommand("zoom_in");
      }}
    >
      <icons.zoomIn size={16} aria-hidden="true" />
    </button>
    <output class="w-12 text-right tabular-nums" aria-label={m.zoom_level_label()}>
      {zoomReason === null ? m.zoom_percent({ percent: String(percent) }) : ""}
    </output>
  </div>
</footer>
