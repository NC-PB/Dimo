<script lang="ts">
  /**
   * Markers of a tolerance derivation (T2.8, FR-TOL-08, FR-CHR-08, D-43): draft table, no
   * tolerance defined, hints such as printed deviations that differ from the fit table, and
   * reference and basic dimensions. Each marker has its own shape as well as a color (D-24
   * spirit, NFR-UX-05) and a text for screen readers. Everything shown comes from Rust's
   * stored derivation; nothing is decided here.
   */
  import { hintText, m } from "$lib/i18n";
  import type { ToleranceDerivation } from "$lib/ipc/bindings";

  interface Props {
    derivation: ToleranceDerivation | null;
    /** Short form for a table cell. */
    compact?: boolean;
  }

  let { derivation, compact = false }: Props = $props();

  const hints = $derived(derivation?.hints ?? []);
  const reference = $derived(hints.some((h) => h.hint === "reference_dimension"));
  const basic = $derived(hints.some((h) => h.hint === "basic_dimension"));
  const notes = $derived(hints.map(hintText).filter((t): t is string => t !== null));
  const noTolerance = $derived(
    derivation?.rule.rule === "no_tolerance_defined" && !reference && !basic,
  );
</script>

{#if derivation}
  <span class="inline-flex shrink-0 items-center gap-1" class:text-xs={!compact}>
    {#if reference}
      <span class="badge reference" title={m.badge_reference_title()} data-badge="reference"
        >({m.badge_reference()})</span
      >
    {/if}
    {#if basic}
      <span class="badge basic" title={m.badge_basic_title()} data-badge="basic"
        >{m.badge_basic()}</span
      >
    {/if}
    {#if noTolerance}
      <span
        class="inline-flex items-center gap-0.5 text-danger"
        title={m.badge_no_tolerance_title()}
        data-badge="no_tolerance"
      >
        <!-- Circle with a bar: "no value". -->
        <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <circle cx="8" cy="8" r="6.25" stroke="currentColor" stroke-width="1.5" />
          <path d="M3.8 12.2 12.2 3.8" stroke="currentColor" stroke-width="1.5" />
        </svg>
        <span class:sr-only={compact}>{m.badge_no_tolerance()}</span>
      </span>
    {/if}
    {#if notes.length > 0}
      <span
        class="inline-flex items-center gap-0.5 text-warning"
        title={notes.join("\n")}
        data-badge="hint"
      >
        <!-- Triangle with an exclamation mark. -->
        <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M8 1.5 15 14H1L8 1.5Z"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
          />
          <path d="M8 6.5v3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          <circle cx="8" cy="12" r="0.9" fill="currentColor" />
        </svg>
        <span class="sr-only">{m.badge_hints()}: {notes.join(" ")}</span>
      </span>
    {/if}
    {#if derivation.draft}
      <span class="badge draft text-warning" title={m.badge_draft_title()} data-badge="draft"
        >{m.badge_draft()}</span
      >
    {/if}
  </span>
{/if}

<style>
  .badge {
    border: 1px solid currentColor;
    border-radius: 3px;
    padding: 0 0.25rem;
    font-size: 0.6875rem;
    line-height: 1rem;
    white-space: nowrap;
  }

  /* Draft: dashed outline, "not final yet". */
  .badge.draft {
    border-style: dashed;
    font-weight: 600;
  }

  /* Reference: round ends like the parentheses of a reference dimension. */
  .badge.reference {
    border-radius: 999px;
    border-color: var(--dimo-text-muted);
    color: var(--dimo-text-muted);
  }

  /* Basic: a sharp frame like the frame of a basic dimension. */
  .badge.basic {
    border-radius: 0;
    border-width: 1.5px;
  }
</style>
