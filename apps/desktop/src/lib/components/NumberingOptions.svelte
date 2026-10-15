<script lang="ts">
  import { fieldClass } from "$lib/ui/fields";
  import { m } from "$lib/i18n";
  import type { InsertPolicy, MultiInstance } from "$lib/ipc/bindings";
  import {
    INSERT_POLICIES,
    MULTI_INSTANCE,
    numberingStore as appNumbering,
    type NumberingStore,
  } from "$lib/stores/numbering.svelte";

  /**
   * Numbering options of the project (D-22, D-23, FR-BAL-11), in the dialog that "Numbering
   * options" on the Home tab opens (T3.18): how repeated features are numbered and which number
   * a characteristic gets when it is added while numbering is locked. The strategy, its preview
   * and Renumber are on the Home tab itself. Each change is one undoable Rust command.
   */
  interface Props {
    /** The numbering store; the app's by default, a fake in tests. */
    numberingStore?: NumberingStore;
  }

  let { numberingStore = appNumbering }: Props = $props();

  const settings = $derived(numberingStore.settings);

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

  const labelClass = "flex flex-col gap-1 text-sm text-text-muted";
  const inputClass = fieldClass("text-sm");
</script>

{#if settings}
  <div class="flex flex-col gap-3 text-sm text-text" data-testid="numbering-options">
    {#if numberingStore.locked}
      <p class="m-0 text-sm text-text-muted" role="note">{m.numbering_locked()}</p>
    {/if}
    <label class={labelClass}>
      {m.numbering_multi_instance()}
      <select
        class={inputClass}
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
    <label class={labelClass}>
      {m.numbering_insert_policy()}
      <select
        class={inputClass}
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
  </div>
{/if}
