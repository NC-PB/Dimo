<script lang="ts">
  import BalloonTools from "$lib/components/BalloonTools.svelte";
  import ProjectStatus from "$lib/components/ProjectStatus.svelte";
  import { m, shortcutLabel } from "$lib/i18n";
  import { shortcutKeys, type ShortcutAction } from "$lib/shortcuts";
  import { documentStore } from "$lib/stores/document.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { zoomPercent } from "$lib/viewport/view-math";

  interface Props {
    onShowShortcuts: () => void;
  }

  let { onShowShortcuts }: Props = $props();

  const hasDocument = $derived(documentStore.current !== null);
  const sheetIndexes = $derived(Array.from({ length: documentStore.sheetCount }, (_, i) => i));

  /** Tooltip with the shortcut, for example "Zoom in (+)". */
  function hint(action: ShortcutAction): string {
    return `${shortcutLabel(action)} (${shortcutKeys(action)})`;
  }

  function onSheetChange(event: Event & { currentTarget: HTMLSelectElement }) {
    documentStore.setSheet(Number(event.currentTarget.value));
  }

  const buttonClass =
    "rounded px-2 py-1 text-sm text-text hover:bg-surface-raised disabled:opacity-40 disabled:hover:bg-transparent";
</script>

<div
  class="flex items-center gap-2 border-b border-border bg-surface px-3 py-1.5"
  role="toolbar"
  aria-label={m.drawing_tools_label()}
>
  <ProjectStatus />

  {#if documentStore.sheetCount > 1}
    <div class="ml-2 flex items-center gap-1" role="group" aria-label={m.sheet_switch_label()}>
      <button
        type="button"
        class={buttonClass}
        title={hint("previous_sheet")}
        aria-label={m.previous_sheet()}
        disabled={documentStore.sheet === 0}
        onclick={() => {
          documentStore.previousSheet();
        }}>‹</button
      >
      <select
        class="rounded border border-border bg-surface px-2 py-1 text-sm text-text"
        aria-label={m.sheet_switch_label()}
        value={String(documentStore.sheet)}
        onchange={onSheetChange}
      >
        {#each sheetIndexes as index (index)}
          <option value={String(index)}>
            {m.sheet_option({
              number: String(index + 1),
              count: String(documentStore.sheetCount),
            })}
          </option>
        {/each}
      </select>
      <button
        type="button"
        class={buttonClass}
        title={hint("next_sheet")}
        aria-label={m.next_sheet()}
        disabled={documentStore.sheet >= documentStore.sheetCount - 1}
        onclick={() => {
          documentStore.nextSheet();
        }}>›</button
      >
    </div>
  {/if}

  {#if hasDocument}
    <BalloonTools />
  {/if}

  <div class="ml-auto flex items-center gap-1">
    <button
      type="button"
      class={buttonClass}
      title={hint("zoom_out")}
      aria-label={m.zoom_out()}
      disabled={!hasDocument}
      onclick={() => {
        viewport.zoomOut();
      }}>−</button
    >
    <output
      class="w-16 text-center text-sm text-text-muted tabular-nums"
      aria-label={m.zoom_level_label()}
      aria-live="off"
    >
      {hasDocument ? m.zoom_percent({ percent: String(zoomPercent(viewport.view.scale)) }) : ""}
    </output>
    <button
      type="button"
      class={buttonClass}
      title={hint("zoom_in")}
      aria-label={m.zoom_in()}
      disabled={!hasDocument}
      onclick={() => {
        viewport.zoomIn();
      }}>+</button
    >
    <button
      type="button"
      class={buttonClass}
      title={hint("fit")}
      disabled={!hasDocument}
      onclick={() => {
        viewport.fit();
      }}
    >
      {m.fit_sheet()}
    </button>
    <button
      type="button"
      class={buttonClass}
      title={hint("show_shortcuts")}
      aria-label={m.show_shortcuts()}
      onclick={onShowShortcuts}>?</button
    >
  </div>
</div>
