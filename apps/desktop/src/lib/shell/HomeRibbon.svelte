<script lang="ts">
  import { disabledReason } from "$lib/commands.svelte";
  import { m, strategyHint, strategyLabel } from "$lib/i18n";
  import type { NumberingStrategy } from "$lib/ipc/bindings";
  import { STRATEGIES, numberingStore } from "$lib/stores/numbering.svelte";
  import BalloonStyleButton from "./BalloonStyleButton.svelte";
  import CommandButton from "./CommandButton.svelte";
  import { RIBBON_FIELD_CLASS, RIBBON_LABEL_CLASS, fieldTitle } from "$lib/ui/fields";
  import GeneralToleranceField from "./GeneralToleranceField.svelte";
  import Ribbon from "./Ribbon.svelte";

  /**
   * Home tab (ADR 0007, mockup "Home tab while ballooning"): tools, balloon, numbering and
   * tolerance. There is no separate "Read area" tool: Place balloon already reads a dragged box
   * (T2.6), so a second tool would do the same thing.
   */
  const orderReason = $derived(disabledReason("preview_numbers"));
</script>

{#snippet tools()}
  <CommandButton id="select_tool" size="large" />
  <CommandButton id="place_tool" size="large" />
{/snippet}

{#snippet balloon()}
  <div class="flex flex-col gap-0.5">
    <CommandButton id="edit_value" />
    <BalloonStyleButton />
    <CommandButton id="delete_selection" />
  </div>
{/snippet}

{#snippet numbering()}
  <CommandButton id="renumber" size="large" />
  <div class="flex flex-col gap-1">
    <label
      class={RIBBON_LABEL_CLASS}
      title={fieldTitle(strategyHint(numberingStore.strategy), orderReason)}
    >
      {m.field_numbering_order()}
      <select
        class="{RIBBON_FIELD_CLASS} min-w-40"
        data-field="numbering-order"
        disabled={orderReason !== null}
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
    <CommandButton id="preview_numbers" />
    <CommandButton id="numbering_options" />
  </div>
{/snippet}

{#snippet tolerance()}
  <GeneralToleranceField />
  <CommandButton id="update_limits" size="large" />
{/snippet}

<Ribbon
  groups={[
    { id: "tools", label: m.ribbon_group_tools(), content: tools },
    { id: "balloon", label: m.ribbon_group_balloon(), content: balloon },
    { id: "numbering", label: m.ribbon_group_numbering(), content: numbering },
    { id: "tolerance", label: m.ribbon_group_tolerance(), content: tolerance },
  ]}
/>
