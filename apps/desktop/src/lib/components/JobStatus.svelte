<script lang="ts">
  import { commandErrorMessage, errorDetail, m } from "$lib/i18n";
  import ErrorDetails from "./ErrorDetails.svelte";
  import type { ExportFormat } from "$lib/ipc/bindings";
  import { exportStore as appExport, type ExportStore } from "$lib/stores/export.svelte";

  interface Props {
    /** The export whose last job is shown. */
    format: ExportFormat;
    /** The export store; the app's unless a test passes its own. */
    store?: ExportStore;
  }

  let { format, store = appExport }: Props = $props();

  /**
   * State of the last export job of one format (T1.9, T3.8): waiting for the file dialog,
   * progress, done (with the issued record and an open error), cancelled or failed. Shared by the
   * Report pane and File > Export (T3.19).
   */
  const job = $derived(store.job(format));
  const startError = $derived(store.startError[format]);
</script>

<div aria-live="polite" class="text-sm">
  {#if store.asking[format]}
    <p class="text-text-muted">{m.export_waiting()}</p>
  {:else if startError}
    <p class="text-danger" role="alert">
      <span aria-hidden="true">⚠</span>
      {commandErrorMessage(startError)}
    </p>
    <ErrorDetails detail={errorDetail(startError)} />
  {:else if job?.state === "running"}
    {@const percent = String(Math.round(job.fraction * 100))}
    <div class="flex items-center gap-2">
      <progress
        class="h-2 flex-1"
        max="1"
        value={job.fraction}
        aria-label={m.export_progress_label()}
      ></progress>
      <span class="text-text-muted">{m.export_running({ percent })}</span>
    </div>
  {:else if job?.state === "done"}
    <p class="text-success">
      <span aria-hidden="true">✓</span>
      {m.export_done({ file: job.fileName })}
    </p>
    {#if job.openError}
      <p class="text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {m.export_open_failed({ message: commandErrorMessage(job.openError) })}
      </p>
      <ErrorDetails detail={errorDetail(job.openError)} />
    {/if}
    {#if job.issued?.state === "recorded"}
      <p class="text-success">
        <span aria-hidden="true">✓</span>
        {m.export_issued_recorded()}
      </p>
    {:else if job.issued?.state === "project_changed"}
      <p class="text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {m.export_issued_project_changed()}
      </p>
    {:else if job.issued?.state === "refused"}
      <p class="text-danger" role="alert">
        <span aria-hidden="true">⚠</span>
        {m.export_issued_refused({ message: commandErrorMessage(job.issued.error) })}
      </p>
      <ErrorDetails detail={errorDetail(job.issued.error)} />
    {/if}
  {:else if job?.state === "cancelled"}
    <p class="text-text-muted">
      <span aria-hidden="true">⊘</span>
      {m.export_cancelled()}
    </p>
  {:else if job?.state === "failed"}
    <p class="text-danger" role="alert">
      <span aria-hidden="true">⚠</span>
      {commandErrorMessage(job.error)}
    </p>
    <ErrorDetails detail={errorDetail(job.error)} />
  {/if}
</div>
