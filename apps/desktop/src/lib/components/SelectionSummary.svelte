<script lang="ts">
  import { commandErrorMessage, m } from "$lib/i18n";
  import PaneSection from "$lib/pane/PaneSection.svelte";
  import { icons } from "$lib/icons";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection as appSelection, type SelectionStore } from "$lib/stores/selection.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import { displayText } from "$lib/table/columns";
  import { selectedCharacteristics } from "$lib/selection-summary";

  /**
   * Several characteristics selected (T1.9, T3.18): the list and the actions that act on all of
   * them at once, as one undo step each: Update limits (re-interpret, M3 decision 22) and the
   * inspect flag (FR-CHR-08). Values of single characteristics are edited in the table.
   */
  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    project?: ProjectStore;
    selected?: SelectionStore;
    tolerance?: ToleranceStore;
  }

  let { project = appProject, selected = appSelection, tolerance = appTolerance }: Props = $props();

  /** Rows listed before "and n more". */
  const MAX_ROWS = 12;

  const chosen = $derived(selectedCharacteristics(selected.ids, project.characteristicById));
  const done = $derived(tolerance.lastReinterpretation);
  const UpdateIcon = icons.updateLimits;

  function setInspect(inspect: boolean): void {
    const ids = chosen.filter((c) => c.inspect !== inspect).map((c) => c.id);
    if (ids.length > 0) {
      void project.execute({
        type: "update_fields",
        ids,
        values: [{ field: "inspect", value: inspect }],
      });
    }
  }

  const choiceClass =
    "min-h-8 rounded-md border border-border-strong px-3 text-[13px] text-text hover:bg-hover aria-pressed:border-accent aria-pressed:bg-accent-soft";
  const allInspected = $derived(chosen.length > 0 && chosen.every((c) => c.inspect));
  const noneInspected = $derived(chosen.every((c) => !c.inspect));
</script>

<div class="flex flex-col" data-testid="selection-properties">
  <PaneSection id="selection.list" title={m.pane_section_selected()}>
    <ul class="m-0 flex list-none flex-col gap-1 p-0 text-[13px]">
      {#each chosen.slice(0, MAX_ROWS) as c (c.id)}
        <li class="flex gap-2">
          <span class="w-8 shrink-0 text-right font-mono text-text">{c.number}</span>
          <span class="truncate text-text-muted"
            >{displayText(c, "requirement_text") || displayText(c, "kind")}</span
          >
        </li>
      {/each}
    </ul>
    {#if chosen.length > MAX_ROWS}
      <p class="m-0 text-xs text-text-muted">
        {m.selection_more({ count: String(chosen.length - MAX_ROWS) })}
      </p>
    {/if}
  </PaneSection>

  <PaneSection id="selection.actions" title={m.pane_section_actions()}>
    <button
      type="button"
      class="flex min-h-8 items-center gap-1.5 self-start rounded-md border border-border-strong px-2.5 text-[13px] text-text hover:bg-hover disabled:opacity-45"
      title={m.cmd_update_limits_desc()}
      disabled={tolerance.busy}
      data-action="update_limits"
      onclick={() => void tolerance.reinterpret(selected.ids)}
    >
      <UpdateIcon size={16} strokeWidth={1.75} aria-hidden="true" class="text-text-muted" />
      {m.cmd_update_limits()}
    </button>
    <p class="m-0 text-xs text-text-muted">{m.detail_reinterpret_hint()}</p>
    {#if done}
      <p class="m-0 text-xs" role="status">
        {m.detail_reinterpret_done({
          count: String(done.reinterpreted),
          manual: String(done.skipped_manual),
          unreadable: String(done.skipped_unreadable),
        })}
      </p>
    {/if}
    {#if tolerance.error}
      <p class="m-0 text-xs text-danger" role="alert">{commandErrorMessage(tolerance.error)}</p>
    {/if}
    <div
      class="flex flex-wrap items-center gap-2"
      role="group"
      aria-label={m.pane_inspect_selected()}
    >
      <span class="text-[13px] text-text-muted">{m.pane_inspect_selected()}</span>
      <button
        type="button"
        class={choiceClass}
        aria-pressed={allInspected}
        data-inspect="yes"
        onclick={() => {
          setInspect(true);
        }}>{m.pane_yes()}</button
      >
      <button
        type="button"
        class={choiceClass}
        aria-pressed={noneInspected}
        data-inspect="no"
        onclick={() => {
          setInspect(false);
        }}>{m.pane_no()}</button
      >
    </div>
  </PaneSection>
</div>
