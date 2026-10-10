<script lang="ts">
  import { m } from "$lib/i18n";
  import type { InsertPolicy, MultiInstance, NumberingStrategy } from "$lib/ipc/bindings";
  import {
    INSERT_POLICIES,
    MULTI_INSTANCE,
    STRATEGIES,
    numberingStore as appNumbering,
    type NumberingStore,
  } from "$lib/stores/numbering.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";

  /**
   * Numbering strategies (T2.7, FR-BAL-04, FR-BAL-05, FR-BAL-07, FR-BAL-11): pick a strategy,
   * preview the new numbers as ghosts on the drawing and in the table, apply as one undo step.
   */
  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    numberingStore?: NumberingStore;
    projectStore?: ProjectStore;
  }

  let { numberingStore = appNumbering, projectStore = appProject }: Props = $props();

  const settings = $derived(numberingStore.settings);
  const preview = $derived(numberingStore.preview);

  // The preview follows the project: every change (also undo and redo) and every other strategy
  // asks Rust again.
  $effect(() => {
    void projectStore.revision;
    void projectStore.session;
    void numberingStore.strategy;
    void numberingStore.previewing;
    void numberingStore.refresh();
  });

  function strategyLabel(strategy: NumberingStrategy): string {
    switch (strategy) {
      case "sheet_zone":
        return m.numbering_strategy_sheet_zone();
      case "view":
        return m.numbering_strategy_view();
      case "view_clockwise":
        return m.numbering_strategy_view_clockwise();
      case "kind":
        return m.numbering_strategy_kind();
      case "manual":
        return m.numbering_strategy_manual();
    }
  }

  function strategyHint(strategy: NumberingStrategy): string {
    switch (strategy) {
      case "sheet_zone":
        return m.numbering_hint_sheet_zone();
      case "view":
        return m.numbering_hint_view();
      case "view_clockwise":
        return m.numbering_hint_view_clockwise();
      case "kind":
        return m.numbering_hint_kind();
      case "manual":
        return m.numbering_hint_manual();
    }
  }

  function multiLabel(value: MultiInstance): string {
    return value === "sub_number" ? m.numbering_multi_sub_number() : m.numbering_multi_quantity();
  }

  function policyLabel(policy: InsertPolicy): string {
    switch (policy) {
      case "next_free":
        return m.numbering_insert_next_free();
      case "sub_number":
        return m.numbering_insert_sub_number();
      case "letter_suffix":
        return m.numbering_insert_letter_suffix();
    }
  }

  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-sm text-text";
</script>

{#if settings}
  <section class="flex flex-col gap-2" aria-labelledby="numbering-title">
    <h2 id="numbering-title" class="text-sm font-semibold text-text">{m.numbering_title()}</h2>

    {#if numberingStore.locked}
      <p class="text-xs text-text-muted" role="note">{m.numbering_locked()}</p>
    {:else}
      <label class="flex flex-col gap-0.5 text-xs text-text-muted">
        {m.numbering_strategy()}
        <select
          class={fieldClass}
          value={numberingStore.strategy}
          onchange={(e) => {
            numberingStore.pick(e.currentTarget.value as NumberingStrategy);
          }}
        >
          {#each STRATEGIES as strategy (strategy)}
            <option value={strategy}>{strategyLabel(strategy)}</option>
          {/each}
        </select>
      </label>
      <p class="text-xs text-text-muted">{strategyHint(numberingStore.strategy)}</p>

      <label class="flex items-center gap-1.5 text-sm text-text">
        <input
          type="checkbox"
          checked={numberingStore.previewing}
          onchange={(e) => {
            numberingStore.setPreviewing(e.currentTarget.checked);
          }}
        />
        {m.numbering_preview()}
      </label>
      {#if numberingStore.previewing && preview}
        <p class="text-xs text-text-muted" aria-live="polite">
          {m.numbering_changes({ count: String(preview.changed) })}
        </p>
      {/if}
      {#if numberingStore.error}
        <p class="text-xs text-danger" role="alert">
          {m.numbering_preview_failed({ message: numberingStore.error.kind })}
        </p>
      {/if}
      <div>
        <button
          type="button"
          class="rounded bg-accent px-3 py-1 text-sm text-accent-text disabled:opacity-50"
          disabled={projectStore.characteristics.length === 0}
          onclick={() => void numberingStore.apply()}>{m.numbering_apply()}</button
        >
      </div>
    {/if}

    <label class="flex flex-col gap-0.5 text-xs text-text-muted">
      {m.numbering_multi_instance()}
      <select
        class={fieldClass}
        value={settings.multi_instance}
        onchange={(e) => {
          void numberingStore.setSettings({
            multi_instance: e.currentTarget.value as MultiInstance,
          });
        }}
      >
        {#each MULTI_INSTANCE as value (value)}
          <option {value}>{multiLabel(value)}</option>
        {/each}
      </select>
    </label>
    <label class="flex flex-col gap-0.5 text-xs text-text-muted">
      {m.numbering_insert_policy()}
      <select
        class={fieldClass}
        value={settings.insert_when_locked}
        onchange={(e) => {
          void numberingStore.setSettings({
            insert_when_locked: e.currentTarget.value as InsertPolicy,
          });
        }}
      >
        {#each INSERT_POLICIES as policy (policy)}
          <option value={policy}>{policyLabel(policy)}</option>
        {/each}
      </select>
    </label>
  </section>
{/if}
