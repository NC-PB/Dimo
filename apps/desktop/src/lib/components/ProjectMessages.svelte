<script lang="ts">
  import { commandErrorMessage, m, noticeLines } from "$lib/i18n";
  import { projectStore } from "$lib/stores/project.svelte";

  const buttonClass = "shrink-0 rounded px-2 py-0.5 text-xs hover:bg-surface-raised";
</script>

<!-- Recovery notice after opening (NFR-REL-01) and the last failed action. -->
{#if projectStore.notice}
  <div
    class="flex items-start gap-3 border-b border-border bg-surface-raised px-3 py-2 text-sm text-text"
    role="status"
  >
    <span aria-hidden="true">ℹ</span>
    <div class="flex-1">
      {#each noticeLines(projectStore.notice) as line (line)}
        <p>{line}</p>
      {/each}
    </div>
    <button
      type="button"
      class={buttonClass}
      onclick={() => {
        projectStore.dismissNotice();
      }}
    >
      {m.dismiss()}
    </button>
  </div>
{/if}
{#if projectStore.error}
  <div
    class="flex items-start gap-3 border-b border-border bg-surface px-3 py-2 text-sm text-danger"
    role="alert"
  >
    <span aria-hidden="true">⚠</span>
    <p class="flex-1">{commandErrorMessage(projectStore.error)}</p>
    <button
      type="button"
      class="{buttonClass} text-text"
      onclick={() => {
        projectStore.dismissError();
      }}
    >
      {m.dismiss()}
    </button>
  </div>
{/if}
