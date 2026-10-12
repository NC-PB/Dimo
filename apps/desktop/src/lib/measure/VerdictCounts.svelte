<script lang="ts">
  import { m } from "$lib/i18n";
  import type { VerdictCounts } from "$lib/ipc/bindings";
  import VerdictMark from "./VerdictMark.svelte";

  interface Props {
    /** Counts from Rust (`measurement_state`). */
    counts: VerdictCounts;
  }

  let { counts }: Props = $props();

  /** Every count in words, for the tooltip and screen readers. */
  const text = $derived(
    m.measure_counts({
      pass: String(counts.pass),
      fail: String(counts.fail),
      not_evaluated: String(counts.not_evaluated),
      missing: String(counts.missing),
    }),
  );
</script>

<!-- Row and sample summaries (M3 decision 9): each count with the mark of its verdict; missing
     cells as a dash, since they have no mark. -->
<span class="flex items-center gap-1.5 text-xs tabular-nums" title={text} aria-label={text}>
  {#if counts.pass > 0}
    <span class="flex items-center gap-0.5" aria-hidden="true"
      ><VerdictMark verdict="pass" size={11} />{counts.pass}</span
    >
  {/if}
  {#if counts.fail > 0}
    <span class="flex items-center gap-0.5 font-bold text-danger" aria-hidden="true"
      ><VerdictMark verdict="fail" size={11} />{counts.fail}</span
    >
  {/if}
  {#if counts.not_evaluated > 0}
    <span class="flex items-center gap-0.5" aria-hidden="true"
      ><VerdictMark verdict="not_evaluated" size={11} />{counts.not_evaluated}</span
    >
  {/if}
  {#if counts.missing > 0}
    <span class="text-text-muted" aria-hidden="true">–{counts.missing}</span>
  {/if}
</span>
