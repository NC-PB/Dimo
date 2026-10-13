<script lang="ts">
  import { disabledReason } from "$lib/commands.svelte";
  import { m } from "$lib/i18n";
  import type { ResultType } from "$lib/ipc/bindings";
  import { resultTypeLabel } from "$lib/measure/labels";
  import { measureUi } from "$lib/measure/measure-ui.svelte";
  import { parts } from "$lib/measure/parts.svelte";
  import { measurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import CommandButton from "./CommandButton.svelte";
  import { FIELD_CLASS, LABEL_CLASS, fieldTitle } from "./field";
  import Ribbon from "./Ribbon.svelte";

  /**
   * Measure tab (ADR 0007, mockup "Measure tab"): parts, values, the result type of the active
   * row, and row filters. The grid does the work; these act on its active cell (T3.15).
   */
  const RESULT_TYPES: readonly ResultType[] = ["auto", "variable", "attribute"];

  const addReason = $derived(disabledReason("add_parts"));
  const row = $derived(
    measureUi.activeRow === null
      ? undefined
      : projectStore.characteristicById.get(measureUi.activeRow),
  );
  const effective = $derived(
    row === undefined ? undefined : measurementStore.row(row.id)?.result_type,
  );
  const rowReason = $derived(
    projectStore.isOpen ? (row === undefined ? m.reason_no_row() : null) : m.reason_no_project(),
  );
</script>

{#snippet partsGroup()}
  <div class="flex flex-col gap-1">
    <CommandButton id="add_parts" size="large" />
  </div>
  <div class="flex flex-col gap-1">
    <label class={LABEL_CLASS} title={fieldTitle(m.cmd_add_parts_desc(), addReason)}>
      {m.samples_add_count()}
      <input
        type="number"
        min="1"
        max="100"
        class="{FIELD_CLASS} w-16"
        data-field="parts-count"
        disabled={addReason !== null}
        bind:value={parts.count}
        onkeydown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            void parts.add();
          }
        }}
      />
    </label>
  </div>
  <div class="flex flex-col gap-0.5">
    <CommandButton id="move_part_left" />
    <CommandButton id="move_part_right" />
    <CommandButton id="delete_part" />
  </div>
{/snippet}

{#snippet values()}
  <div class="flex flex-col gap-0.5">
    <CommandButton id="paste_values" />
    <CommandButton id="clear_cell" />
  </div>
{/snippet}

{#snippet rowGroup()}
  <label class={LABEL_CLASS} title={fieldTitle(m.field_row_result_desc(), rowReason)}>
    {m.field_row_result()}
    <select
      class="{FIELD_CLASS} min-w-44"
      data-field="row-result"
      disabled={row === undefined}
      value={row?.result_type ?? ""}
      onchange={(e) => {
        const value = RESULT_TYPES.find((t) => t === e.currentTarget.value);
        const select = e.currentTarget;
        if (value !== undefined) {
          void measureUi.setResultType(value).then(() => {
            // A refused change shows the stored type again; the grid explains why.
            select.value = row?.result_type ?? "";
          });
        }
      }}
    >
      {#each RESULT_TYPES as type (type)}
        <option value={type}>{resultTypeLabel(type, effective)}</option>
      {/each}
    </select>
  </label>
{/snippet}

{#snippet show()}
  <div class="flex flex-col gap-0.5">
    <CommandButton id="only_missing" />
    <CommandButton id="only_failed" />
  </div>
{/snippet}

<Ribbon
  groups={[
    { id: "parts", label: m.ribbon_group_parts(), content: partsGroup },
    { id: "values", label: m.ribbon_group_values(), content: values },
    { id: "row", label: m.ribbon_group_row(), content: rowGroup },
    { id: "show", label: m.ribbon_group_show(), content: show },
  ]}
/>
