<script lang="ts">
  /**
   * The offer after the general tolerance changed (M3 decision 22): "n characteristics have no
   * tolerance yet ... [Update n] [Not now]", and the confirmation after Update. Shown where the
   * general tolerance can be changed: the settings page and above the drawing (Home and Drawing
   * tabs, T3.15).
   */
  import { m } from "$lib/i18n";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";

  interface Props {
    tolerance?: ToleranceStore;
    /** Extra classes for the placement. */
    class?: string;
    /** Also show the line after an update (the settings page; the drawing shows the limits). */
    confirmation?: boolean;
  }

  let { tolerance = appTolerance, class: extra = "", confirmation = true }: Props = $props();

  const buttonClass =
    "min-h-8 rounded border border-border px-3 text-xs hover:bg-surface-raised disabled:opacity-50";
</script>

{#if tolerance.generalOffer}
  {@const offer = tolerance.generalOffer}
  {@const count = String(offer.count)}
  <div
    class="flex flex-wrap items-center gap-2 rounded border border-accent bg-surface px-3 py-2 text-sm text-text {extra}"
    role="status"
    data-testid="general-offer"
  >
    <span aria-hidden="true">ℹ</span>
    <p class="flex-1">
      {#if offer.removing}
        {offer.count === 1
          ? m.tolerance_general_remove_one()
          : m.tolerance_general_remove_other({ count })}
      {:else}
        {offer.count === 1
          ? m.tolerance_general_offer_one()
          : m.tolerance_general_offer_other({ count })}
      {/if}
    </p>
    <button
      type="button"
      class="min-h-8 rounded bg-accent-strong px-3 text-xs text-accent-ink disabled:opacity-50"
      disabled={tolerance.busy}
      onclick={() => void tolerance.applyGeneralUpdate()}
      >{m.tolerance_general_update({ count })}</button
    >
    <button
      type="button"
      class={buttonClass}
      onclick={() => {
        tolerance.declineGeneralUpdate();
      }}>{m.tolerance_general_not_now()}</button
    >
  </div>
{:else if confirmation && tolerance.generalUpdated !== null}
  <p class="text-xs text-text {extra}" role="status" data-testid="general-updated">
    ✓ {m.tolerance_general_updated({ count: String(tolerance.generalUpdated) })}
  </p>
{/if}
