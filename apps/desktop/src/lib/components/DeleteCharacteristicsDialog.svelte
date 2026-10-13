<script lang="ts">
  import { AlertDialog } from "bits-ui";
  import { m } from "$lib/i18n";
  import {
    balloonTools as appTools,
    type BalloonToolsStore,
  } from "$lib/stores/balloon-tools.svelte";

  interface Props {
    /** The balloon tools; the app's unless a test passes its own. */
    tools?: BalloonToolsStore;
  }

  let { tools: balloonTools = appTools }: Props = $props();

  const buttonClass = "rounded px-3 py-1 text-sm";
  const results = $derived(balloonTools.pendingDelete?.results ?? 0);
</script>

<!-- Asked before characteristics with results are deleted (M3 decision 16, FR-MEA-01). -->
<AlertDialog.Root
  bind:open={
    () => balloonTools.pendingDelete !== null,
    (open) => {
      if (!open) {
        balloonTools.cancelDelete();
      }
    }
  }
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="fixed inset-0 z-40 bg-black/40" />
    <AlertDialog.Content
      class="fixed top-1/2 left-1/2 z-50 w-96 max-w-[90vw] -translate-x-1/2 -translate-y-1/2 rounded border border-border bg-surface p-4 text-text shadow-lg"
    >
      <AlertDialog.Title class="mb-2 text-base font-semibold"
        >{m.delete_results_title()}</AlertDialog.Title
      >
      <AlertDialog.Description class="text-sm text-text-muted">
        {results === 1
          ? m.delete_results_body_one()
          : m.delete_results_body_other({ count: String(results) })}
      </AlertDialog.Description>
      <div class="mt-4 flex justify-end gap-2">
        <AlertDialog.Cancel class="{buttonClass} border border-border hover:bg-surface-raised">
          {m.delete_results_no()}
        </AlertDialog.Cancel>
        <!-- Plain button: it answers first, which also closes the dialog. -->
        <button
          type="button"
          class="{buttonClass} bg-danger text-accent-text"
          onclick={() => void balloonTools.confirmDelete()}
        >
          {m.delete_results_yes()}
        </button>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
