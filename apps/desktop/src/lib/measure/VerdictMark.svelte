<script lang="ts">
  import type { Verdict } from "$lib/ipc/bindings";

  interface Props {
    /** The verdict Rust computed; `missing` draws nothing. */
    verdict: Verdict;
    /** Size in CSS px. */
    size?: number;
  }

  let { verdict, size = 14 }: Props = $props();

  /**
   * Shape plus color, never color alone (M3 decision 9, NFR-UX-05, D-24): pass is a check mark,
   * fail a cross, not evaluated a hollow circle, missing nothing. The shapes also tell the
   * verdicts apart in greyscale.
   */
</script>

{#if verdict === "pass"}
  <svg
    class="shrink-0 text-success"
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    aria-hidden="true"
    data-mark="pass"
  >
    <path
      d="M2.5 8.5 6.5 12.5 13.5 3.5"
      stroke="currentColor"
      stroke-width="2.2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
{:else if verdict === "fail"}
  <svg
    class="shrink-0 text-danger"
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    aria-hidden="true"
    data-mark="fail"
  >
    <path
      d="M3.5 3.5 12.5 12.5M12.5 3.5 3.5 12.5"
      stroke="currentColor"
      stroke-width="2.6"
      stroke-linecap="round"
    />
  </svg>
{:else if verdict === "not_evaluated"}
  <svg
    class="shrink-0 text-text-muted"
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    aria-hidden="true"
    data-mark="not_evaluated"
  >
    <circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="1.8" />
  </svg>
{/if}
