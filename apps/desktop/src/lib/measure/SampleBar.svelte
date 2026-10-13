<script lang="ts">
  import { m } from "$lib/i18n";
  import { parts as appParts, type PartsStore } from "./parts.svelte";

  interface Props {
    /** The parts actions; the app's store unless a test passes its own. */
    parts?: PartsStore;
  }

  let { parts = appParts }: Props = $props();

  /**
   * The line above the measurement grid for the parts commands of the ribbon (T3.15): the
   * question before a part with results is deleted (M3 decision 16) and Rust's refusals. Shown
   * only while there is something to say.
   */
  const asking = $derived(
    parts.sample !== undefined && parts.confirming === parts.sample.id ? parts.sample : undefined,
  );
</script>

{#if asking !== undefined || parts.notice !== null}
  <div
    class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border bg-surface px-3 py-1.5 text-sm"
    role="group"
    aria-label={m.samples_bar_label()}
  >
    {#if asking !== undefined}
      <span class="text-danger" role="alert"
        >{m.samples_delete_confirm({
          serial: asking.serial,
          count: String(parts.resultCount(asking.id)),
        })}</span
      >
      <button
        type="button"
        class="min-h-8 rounded bg-danger px-3 text-sm text-accent-ink"
        onclick={() => void parts.remove()}>{m.samples_delete_yes()}</button
      >
      <button
        type="button"
        class="min-h-8 rounded border border-border px-3 text-sm"
        onclick={() => {
          parts.keep();
        }}>{m.samples_delete_no()}</button
      >
    {/if}
    {#if parts.notice !== null}
      <span class="text-danger" role="alert" data-testid="sample-notice"
        ><span aria-hidden="true">⚠</span> {parts.notice}</span
      >
    {/if}
  </div>
{/if}
