<script lang="ts">
  import type { Snippet } from "svelte";
  import { icons } from "$lib/icons";
  import { paneFolds as appFolds, type PaneFolds } from "./folds.svelte";

  /**
   * One section of a properties pane (T3.18): a heading that folds and unfolds the content. The
   * fold of each section is remembered per user (`folds.svelte.ts`), so a section folded for one
   * characteristic stays folded for the next. `id` names the section across panes, for example
   * `characteristic.history`. A section with `onclose` belongs to a ribbon command (zone grid,
   * views, numbering preview) and has a close button instead of folding.
   */
  interface Props {
    id: string;
    title: string;
    /** Closes the section (and ends its command); such a section does not fold. */
    onclose?: () => void;
    closeLabel?: string;
    folds?: PaneFolds;
    children: Snippet;
  }

  let { id, title, onclose, closeLabel = "", folds = appFolds, children }: Props = $props();
  const uid = $props.id();
  const open = $derived(onclose !== undefined || folds.isOpen(id));
  const Chevron = icons.unfold;
  const Close = icons.close;

  const headingClass = "text-xs font-medium tracking-[0.08em] text-text-muted uppercase";
</script>

<section class="border-b border-border px-3.5 py-2" data-section={id}>
  <h3 class="m-0 flex items-center justify-between gap-2">
    {#if onclose}
      <span id="{uid}-heading" class="flex min-h-8 items-center {headingClass}">{title}</span>
      <button
        type="button"
        class="flex size-8 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-hover hover:text-text"
        aria-label={closeLabel}
        title={closeLabel}
        onclick={onclose}
      >
        <Close size={16} strokeWidth={1.75} aria-hidden="true" />
      </button>
    {:else}
      <button
        type="button"
        id="{uid}-heading"
        class="-mx-1.5 flex min-h-8 flex-1 items-center gap-1.5 rounded-md px-1.5 text-left hover:bg-hover {headingClass}"
        aria-expanded={open}
        aria-controls="{uid}-content"
        onclick={() => {
          folds.toggle(id);
        }}
      >
        <Chevron
          size={16}
          strokeWidth={1.75}
          aria-hidden="true"
          class="shrink-0 transition-transform {open ? '' : '-rotate-90'}"
        />
        {title}
      </button>
    {/if}
  </h3>
  {#if open}
    <div
      id="{uid}-content"
      class="flex flex-col gap-2 pt-1 pb-1.5"
      role="group"
      aria-labelledby="{uid}-heading"
    >
      {@render children()}
    </div>
  {/if}
</section>
