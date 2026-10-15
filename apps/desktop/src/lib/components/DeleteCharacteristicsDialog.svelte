<script lang="ts">
  import AppDialog from "$lib/components/AppDialog.svelte";
  import { m } from "$lib/i18n";
  import {
    balloonTools as appTools,
    type BalloonToolsStore,
  } from "$lib/stores/balloon-tools.svelte";
  import { buttonClass } from "$lib/ui/fields";

  interface Props {
    /** The balloon tools; the app's unless a test passes its own. */
    tools?: BalloonToolsStore;
  }

  let { tools: balloonTools = appTools }: Props = $props();

  /** Number of results that go with the characteristics, counted by Rust (`delete_check`). */
  const results = $derived(balloonTools.pendingDelete?.results ?? 0);
</script>

<!-- Asked before characteristics with results are deleted (M3 decision 16, FR-MEA-01). -->
<AppDialog
  alert
  open={balloonTools.pendingDelete !== null}
  onClose={() => {
    balloonTools.cancelDelete();
  }}
  title={m.delete_results_title()}
  description={results === 1
    ? m.delete_results_body_one()
    : m.delete_results_body_other({ count: String(results) })}
  closeLabel={m.delete_results_no()}
  testid="delete-dialog"
>
  {#snippet actions()}
    <!-- Plain button: it answers first, which also closes the dialog. -->
    <button
      type="button"
      class={buttonClass.danger}
      data-answer="delete"
      onclick={() => void balloonTools.confirmDelete()}
    >
      {m.delete_results_yes()}
    </button>
  {/snippet}
</AppDialog>
