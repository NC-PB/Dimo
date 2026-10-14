<script lang="ts">
  import { m } from "$lib/i18n";

  /**
   * The diagnostic text of an error (Rust's English message, the path, the operating system
   * error), collapsed under "Details" below the translated text (MF F3). For bug reports, never
   * the text the user reads first.
   */
  let { detail }: { detail: string | null } = $props();
</script>

{#if detail}
  <details class="mt-1 text-xs text-text-muted" data-testid="error-details">
    <summary class="cursor-pointer select-none">{m.error_details()}</summary>
    <code class="block font-mono break-all whitespace-pre-wrap select-text">{detail}</code>
  </details>
{/if}
