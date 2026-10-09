<script lang="ts">
  import { devTools } from "$lib/dev/dev-tools.svelte";
  import { m } from "$lib/i18n";
  import { documentStore } from "$lib/stores/document.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";

  // Rendered only in development builds (see SidePanel). Not a feature.
  const ms = (value: number) => value.toFixed(1);
</script>

<section class="flex flex-col gap-2 border-t border-border pt-2" aria-label={m.dev_tools_label()}>
  <h2 class="text-xs font-semibold tracking-wide uppercase">{m.dev_tools_label()}</h2>
  <label class="flex items-center gap-2">
    <input type="checkbox" bind:checked={devTools.balloons} />
    {m.dev_balloons()}
  </label>
  <label class="flex items-center gap-2">
    {m.dev_balloon_count()}
    <input
      type="number"
      min="0"
      max="5000"
      step="100"
      class="w-24 rounded border border-border bg-surface px-2 py-0.5 text-text"
      bind:value={devTools.count}
    />
  </label>
  <button
    type="button"
    class="self-start rounded border border-border px-2 py-1 hover:bg-surface-raised disabled:opacity-40"
    disabled={devTools.running || documentStore.current === null}
    onclick={() => void devTools.measurePan(viewport)}
  >
    {devTools.running ? m.dev_pan_running() : m.dev_pan_check()}
  </button>
  <button
    type="button"
    class="self-start rounded border border-border px-2 py-1 hover:bg-surface-raised disabled:opacity-40"
    disabled={devTools.running || documentStore.current === null}
    onclick={() => void devTools.measureTableScroll()}
  >
    {devTools.running ? m.dev_pan_running() : m.dev_table_check()}
  </button>
  {#if devTools.tableResult}
    {@const r = devTools.tableResult}
    <p class="text-xs" role="status">
      {m.dev_table_result({
        frames: String(r.frames),
        mean: ms(r.meanMs),
        p95: ms(r.p95Ms),
        max: ms(r.maxMs),
      })}
    </p>
  {/if}
  {#if devTools.result}
    {@const r = devTools.result}
    <p class="text-xs" role="status">
      {m.dev_pan_result({
        frames: String(r.frames),
        mean: ms(r.meanMs),
        p95: ms(r.p95Ms),
        max: ms(r.maxMs),
      })}
    </p>
  {/if}
</section>
