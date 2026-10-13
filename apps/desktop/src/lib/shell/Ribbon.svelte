<script lang="ts" module>
  import type { Snippet } from "svelte";

  /** One labelled group of the ribbon. */
  export interface RibbonGroup {
    id: string;
    label: string;
    content: Snippet;
  }
</script>

<script lang="ts">
  import { Popover } from "bits-ui";
  import { m } from "$lib/i18n";
  import { fitGroups } from "./ribbon-fit";

  /**
   * One ribbon row (ADR 0007, T3.15): labelled groups side by side, never wrapped. When the
   * window is too narrow, the groups that do not fit move in order into the "More" menu at the
   * end, so every command stays reachable.
   */
  interface Props {
    groups: readonly RibbonGroup[];
  }

  let { groups }: Props = $props();

  /** Width of the "More" button with its margin, in CSS px. */
  const MORE_WIDTH = 80;

  let available = $state(0);
  /** Last measured width of each group; kept while a group sits in the menu. */
  const widths = $state<Record<string, number>>({});
  let moreOpen = $state(false);

  const count = $derived(
    available === 0
      ? groups.length
      : fitGroups(
          groups.map((g) => widths[g.id] ?? 0),
          available,
          MORE_WIDTH,
        ),
  );
  const inline = $derived(groups.slice(0, count));
  const hidden = $derived(groups.slice(count));

  /** Keeps the width of a group shown in the row; a group in the menu keeps its last width. */
  function measure(id: string) {
    return (element: HTMLElement) => {
      const update = () => {
        if (element.offsetWidth > 0) {
          widths[id] = element.offsetWidth;
        }
      };
      update();
      if (typeof ResizeObserver === "undefined") {
        return;
      }
      const observer = new ResizeObserver(update);
      observer.observe(element);
      return () => {
        observer.disconnect();
      };
    };
  }
</script>

{#snippet group(g: RibbonGroup, inRow: boolean)}
  <div
    class="flex shrink-0 flex-col {inRow ? 'border-r border-border px-2.5' : 'px-1'}"
    role="group"
    aria-label={g.label}
    data-group={g.id}
  >
    <div class="flex min-h-[6.5rem] items-start gap-1">
      {@render g.content()}
    </div>
    <div class="pt-0.5 pb-1.5 text-center text-xs text-text-muted" aria-hidden="true">
      {g.label}
    </div>
  </div>
{/snippet}

<div class="flex min-w-0 flex-1 items-stretch overflow-hidden" bind:clientWidth={available}>
  {#each inline as g (g.id)}
    <div class="flex shrink-0" {@attach measure(g.id)}>
      {@render group(g, true)}
    </div>
  {/each}
  {#if hidden.length > 0}
    <Popover.Root bind:open={moreOpen}>
      <Popover.Trigger
        class="mx-1 my-1 flex min-h-10 shrink-0 flex-col items-center justify-center gap-0.5 self-start rounded-lg px-2 py-1 text-[13px] text-text hover:bg-hover"
        aria-label={m.ribbon_more()}
      >
        <span aria-hidden="true" class="text-lg leading-none text-text-muted">⋯</span>
        <span>{m.ribbon_more()}</span>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          class="z-50 flex max-h-[70vh] max-w-[90vw] flex-col gap-2 overflow-auto rounded-lg border border-border bg-surface p-2 text-text shadow-lg"
          sideOffset={4}
          align="end"
        >
          {#each hidden as g (g.id)}
            {@render group(g, false)}
          {/each}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  {/if}
</div>
