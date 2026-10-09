<script lang="ts">
  import { m } from "$lib/i18n";
  import type { Characteristic } from "$lib/ipc/bindings";
  import { projectStore } from "$lib/stores/project.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { COLUMNS, displayText, type ColumnId } from "$lib/table/columns";
  import { selectedCharacteristics } from "$lib/selection-summary";

  /** Fields shown for one characteristic. Values are Rust's stored text (rule 5). */
  const FIELDS: readonly ColumnId[] = [
    "kind",
    "requirement_text",
    "nominal",
    "upper_dev",
    "lower_dev",
    "upper_limit",
    "lower_limit",
    "unit",
    "fit",
    "quantity",
  ];
  /** Rows listed for a group selection before "and n more". */
  const MAX_ROWS = 12;

  const chosen = $derived(selectedCharacteristics(selection.ids, projectStore.characteristicById));
  const single = $derived(chosen.length === 1 ? (chosen[0] ?? null) : null);

  function label(id: ColumnId): string {
    return COLUMNS.find((c) => c.id === id)?.label() ?? id;
  }

  function filled(c: Characteristic): ColumnId[] {
    return FIELDS.filter((id) => displayText(c, id) !== "");
  }
</script>

<!-- Compact summary of the selected characteristics (T1.9). Editing stays in the table. -->
<section class="flex-1 overflow-auto" aria-label={m.selection_summary_label()}>
  {#if chosen.length === 0}
    <p>{m.side_panel_empty()}</p>
  {:else if single}
    <h2 class="mb-2 text-base font-semibold text-text">
      {label("number")}
      {single.number}
    </h2>
    <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
      {#each filled(single) as id (id)}
        <dt>{label(id)}</dt>
        <dd class="break-words text-text">{displayText(single, id)}</dd>
      {/each}
    </dl>
  {:else}
    <h2 class="mb-2 font-semibold text-text">
      {m.selection_count({ count: String(chosen.length) })}
    </h2>
    <ul class="flex flex-col gap-1">
      {#each chosen.slice(0, MAX_ROWS) as c (c.id)}
        <li class="flex gap-2">
          <span class="w-8 shrink-0 text-right tabular-nums text-text">{c.number}</span>
          <span class="truncate"
            >{displayText(c, "requirement_text") || displayText(c, "kind")}</span
          >
        </li>
      {/each}
    </ul>
    {#if chosen.length > MAX_ROWS}
      <p class="mt-1">{m.selection_more({ count: String(chosen.length - MAX_ROWS) })}</p>
    {/if}
  {/if}
</section>
