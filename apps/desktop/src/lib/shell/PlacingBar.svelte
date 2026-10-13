<script lang="ts">
  import { m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import {
    balloonTools as appTools,
    type BalloonToolsStore,
  } from "$lib/stores/balloon-tools.svelte";

  /**
   * The bar on the drawing while Place balloon is on (M3 decision 19): what to do and a Stop
   * button. Esc, right click on the drawing and Select stop placing too.
   */
  interface Props {
    tools?: BalloonToolsStore;
  }

  let { tools = appTools }: Props = $props();
</script>

{#if tools.tool === "place"}
  <div
    class="pointer-events-auto absolute top-2.5 left-1/2 z-20 flex max-w-[calc(100%-2rem)] -translate-x-1/2 items-center gap-2.5 rounded-full border border-accent bg-surface py-1.5 pr-1.5 pl-3.5 text-[13px] text-text shadow-lg"
    role="status"
    data-testid="placing-bar"
  >
    <icons.place size={16} strokeWidth={2} aria-hidden="true" class="shrink-0 text-accent-strong" />
    <span><strong>{m.placing_title()}</strong> {m.placing_hint()}</span>
    <button
      type="button"
      class="min-h-8 shrink-0 rounded-full bg-accent-strong px-3 text-xs font-semibold text-accent-ink"
      onclick={() => {
        tools.setTool("select");
      }}
    >
      {m.placing_stop()}
    </button>
  </div>
{/if}
