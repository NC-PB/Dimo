<script lang="ts">
  import type { Snippet } from "svelte";
  import { icons } from "$lib/icons";

  /**
   * The frame of a properties pane right of a work area (ADR 0007, T3.18, mockup "Main"): a
   * header with the title and an optional close button, then the sections, which scroll.
   * The Drawing view, Measure and Report use the same frame; each fills it with
   * `PaneSection`s. Width and colors come from the theme tokens.
   */
  interface Props {
    /** Heading of the pane, for example "Characteristic 4". */
    title: string;
    /** Accessible name of the pane landmark, for example "Characteristic properties". */
    label: string;
    /** Shows a close button (icon only, with tooltip) that calls this. */
    onclose?: () => void;
    /** Name and tooltip of the close button. */
    closeLabel?: string;
    testid?: string;
    /** The sections; none for an empty pane (no project open). */
    children?: Snippet;
  }

  let { title, label, onclose, closeLabel = "", testid, children }: Props = $props();
  const uid = $props.id();
  const Close = icons.close;
</script>

<aside
  class="flex min-h-0 w-80 shrink-0 flex-col border-l border-border bg-surface text-sm text-text"
  aria-label={label}
  data-testid={testid}
>
  <header
    class="flex min-h-12 shrink-0 items-center justify-between gap-2 border-b border-border py-1.5 pr-2 pl-3.5"
  >
    <h2 id="{uid}-title" class="min-w-0 font-heading text-base font-semibold break-words">
      {title}
    </h2>
    {#if onclose}
      <button
        type="button"
        class="flex size-8 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-hover hover:text-text"
        aria-label={closeLabel}
        title={closeLabel}
        data-testid="pane-close"
        onclick={onclose}
      >
        <Close size={18} strokeWidth={1.75} aria-hidden="true" />
      </button>
    {/if}
  </header>
  <div class="min-h-0 flex-1 overflow-y-auto">
    {@render children?.()}
  </div>
</aside>
