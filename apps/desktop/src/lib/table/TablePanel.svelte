<script lang="ts">
  import { m } from "$lib/i18n";
  import { projectStore } from "$lib/stores/project.svelte";
  import CharacteristicTable from "./CharacteristicTable.svelte";

  /** Height of the panel in CSS px, kept per user in local storage (view state only). */
  const STORAGE_KEY = "dimo.table.height";
  const DEFAULT_HEIGHT = 260;
  const MIN_HEIGHT = 96;
  /** Space the drawing keeps above the panel. */
  const MIN_VIEWPORT = 160;
  const KEY_STEP = 24;

  function storedHeight(): number {
    try {
      const value = Number(localStorage.getItem(STORAGE_KEY));
      return Number.isFinite(value) && value >= MIN_HEIGHT ? value : DEFAULT_HEIGHT;
    } catch {
      return DEFAULT_HEIGHT;
    }
  }

  let height = $state(storedHeight());
  let windowHeight = $state(window.innerHeight);
  const maxHeight = $derived(Math.max(MIN_HEIGHT, windowHeight - MIN_VIEWPORT));
  const shownHeight = $derived(Math.min(height, maxHeight));

  function setHeight(value: number) {
    height = Math.round(Math.max(MIN_HEIGHT, Math.min(maxHeight, value)));
    try {
      localStorage.setItem(STORAGE_KEY, String(height));
    } catch {
      // Storage is optional.
    }
  }

  let resize: { pointer: number; y: number; height: number } | null = null;

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    resize = { pointer: event.pointerId, y: event.clientY, height: shownHeight };
  }

  function onPointerMove(event: PointerEvent) {
    if (resize?.pointer === event.pointerId) {
      setHeight(resize.height + resize.y - event.clientY);
    }
  }

  function onPointerUp(event: PointerEvent) {
    if (resize?.pointer === event.pointerId) {
      resize = null;
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setHeight(shownHeight + KEY_STEP);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setHeight(shownHeight - KEY_STEP);
    }
  }
</script>

<svelte:window
  onresize={() => {
    windowHeight = window.innerHeight;
  }}
/>

{#if projectStore.isOpen}
  <section
    class="flex shrink-0 flex-col border-t border-border bg-surface text-text"
    style:height="{shownHeight}px"
    aria-label={m.table_label()}
  >
    <!-- Drag or use the arrow keys to change the height of the table. A focusable separator is
         a window splitter widget in ARIA, but Svelte does not count it as interactive. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="splitter h-1.5 shrink-0 cursor-row-resize touch-none"
      role="separator"
      tabindex="0"
      aria-orientation="horizontal"
      aria-label={m.table_resize()}
      aria-valuemin={MIN_HEIGHT}
      aria-valuemax={maxHeight}
      aria-valuenow={shownHeight}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onkeydown={onKeyDown}
    ></div>
    <CharacteristicTable />
  </section>
{/if}

<style>
  .splitter:hover,
  .splitter:focus-visible {
    background: var(--dimo-accent);
  }
</style>
