<script lang="ts">
  import { autosaveLabel, m } from "$lib/i18n";
  import { projectStore } from "$lib/stores/project.svelte";

  const status = $derived(projectStore.status);
  const title = $derived(status?.file_name ?? m.untitled_project());
</script>

{#if projectStore.isOpen && status}
  <div class="flex min-w-0 items-center gap-3 text-sm" role="status" aria-live="polite">
    <span class="max-w-56 truncate font-medium text-text" {title}>
      {title}{status.modified ? " *" : ""}
    </span>
    <span class="text-text-muted">
      {m.characteristic_count({ count: String(projectStore.characteristics.length) })}
    </span>
    <!-- Autosave state with shape and color (NFR-UX-05). -->
    <span
      class="flex items-center gap-1 {status.autosave.state === 'failed'
        ? 'text-danger'
        : 'text-text-muted'}"
      data-autosave={status.autosave.state}
    >
      <span aria-hidden="true">
        {status.autosave.state === "failed" ? "⚠" : status.autosave.state === "clean" ? "✓" : "●"}
      </span>
      {autosaveLabel(status.autosave, status.file_name !== null)}
    </span>
  </div>
{/if}
