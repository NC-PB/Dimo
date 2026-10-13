<script lang="ts">
  import { m, tabLabel } from "$lib/i18n";
  import { TABS, view as appView, type Tab, type ViewStore } from "$lib/stores/view.svelte";
  import DrawingRibbon from "./DrawingRibbon.svelte";
  import HomeRibbon from "./HomeRibbon.svelte";
  import MeasureRibbon from "./MeasureRibbon.svelte";
  import ReportRibbon from "./ReportRibbon.svelte";
  import ViewRibbon from "./ViewRibbon.svelte";

  /**
   * Tab bar and ribbon (ADR 0007): File (a full page, styled as the filled button), Home,
   * Drawing, Review, Measure, Report, View. The tabs follow the ARIA tabs pattern: one tab in the
   * Tab order, arrow keys, Home and End move between them, Enter or Space is not needed because
   * a tab opens when it gets the focus.
   */
  interface Props {
    view?: ViewStore;
  }

  let { view = appView }: Props = $props();

  const tabs: HTMLButtonElement[] = [];

  function onKeyDown(event: KeyboardEvent, index: number) {
    const last = TABS.length - 1;
    const next =
      event.key === "ArrowRight"
        ? index === last
          ? 0
          : index + 1
        : event.key === "ArrowLeft"
          ? index === 0
            ? last
            : index - 1
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    if (next === null) {
      return;
    }
    event.preventDefault();
    const tab = TABS[next];
    if (tab !== undefined) {
      view.setTab(tab);
      tabs[next]?.focus();
    }
  }

  function tabClass(tab: Tab, on: boolean): string {
    if (tab === "file") {
      return `my-1.5 mr-1.5 min-h-8 rounded-lg px-4 font-semibold ${
        on
          ? "bg-accent-strong text-accent-ink ring-2 ring-accent ring-offset-1 ring-offset-surface"
          : "bg-accent-strong text-accent-ink hover:opacity-90"
      }`;
    }
    return `min-h-10 border-b-[3px] px-4 pt-2 pb-1.5 ${
      on
        ? "border-accent font-semibold text-text"
        : "border-transparent text-text-muted hover:text-text"
    }`;
  }
</script>

<div class="border-b border-border bg-surface">
  <div class="flex items-stretch gap-0.5 px-2" role="tablist" aria-label={m.tabs_label()}>
    {#each TABS as tab, index (tab)}
      {@const on = view.tab === tab}
      <button
        bind:this={tabs[index]}
        type="button"
        role="tab"
        id="tab-{tab}"
        class="text-sm {tabClass(tab, on)}"
        aria-selected={on}
        aria-controls="tab-panel"
        tabindex={on ? 0 : -1}
        data-tab={tab}
        onclick={() => {
          view.setTab(tab);
        }}
        onkeydown={(event) => {
          onKeyDown(event, index);
        }}
      >
        {tabLabel(tab)}
      </button>
    {/each}
  </div>
  {#if view.tab !== "file"}
    <div
      id="tab-panel"
      class="flex border-t border-border px-1.5 pt-1.5"
      role="tabpanel"
      aria-labelledby="tab-{view.tab}"
    >
      {#if view.tab === "home"}
        <HomeRibbon />
      {:else if view.tab === "drawing"}
        <DrawingRibbon />
      {:else if view.tab === "measure"}
        <MeasureRibbon />
      {:else if view.tab === "report"}
        <ReportRibbon />
      {:else if view.tab === "view"}
        <ViewRibbon />
      {:else}
        <p class="flex min-h-[6.5rem] items-center px-3 text-sm text-text-muted">
          {m.view_planned()}
        </p>
      {/if}
    </div>
  {/if}
</div>
