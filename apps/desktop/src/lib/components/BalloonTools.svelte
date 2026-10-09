<script lang="ts">
  import { Popover, ToggleGroup } from "bits-ui";
  import { m, shortcutLabel } from "$lib/i18n";
  import type { BalloonShape, Color } from "$lib/ipc/bindings";
  import { shortcutKeys, type ShortcutAction } from "$lib/shortcuts";
  import { balloonTools, type Tool } from "$lib/stores/balloon-tools.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";

  const SHAPES: readonly BalloonShape[] = ["circle", "flag", "rectangle"];
  /** Balloon sizes offered, in mm on the printed sheet (D-24 default 7). */
  const SIZES_MM: readonly number[] = [5, 6, 7, 8, 10, 12];
  /**
   * Outline colors offered: the D-24 blue, black and three colors of a color blind safe
   * palette. Each one is named, so the choice never depends on seeing the color.
   */
  const COLORS: readonly { color: Color; label: () => string }[] = [
    { color: "#0057B8", label: () => m.color_blue() },
    { color: "#000000", label: () => m.color_black() },
    { color: "#009988", label: () => m.color_teal() },
    { color: "#CC3311", label: () => m.color_red() },
    { color: "#EE3377", label: () => m.color_magenta() },
  ];

  const hasDocument = $derived(documentStore.current !== null);
  const count = $derived(selection.size);
  /** Style of the primary selected balloon on this sheet, shown as the current choice. */
  const current = $derived(
    balloonGestures.rendered.find((b) => b.characteristic === selection.primary)?.style ??
      balloonGestures.rendered.find((b) => selection.ids.has(b.characteristic))?.style ??
      null,
  );

  function hint(action: ShortcutAction): string {
    return `${shortcutLabel(action)} (${shortcutKeys(action)})`;
  }

  function shapeLabel(shape: BalloonShape): string {
    switch (shape) {
      case "circle":
        return m.shape_circle();
      case "flag":
        return m.shape_flag();
      case "rectangle":
        return m.shape_rectangle();
    }
  }

  const buttonClass =
    "rounded px-2 py-1 text-sm text-text hover:bg-surface-raised disabled:opacity-40 disabled:hover:bg-transparent";
  const itemClass =
    "rounded px-2 py-1 text-sm text-text hover:bg-surface-raised data-[state=on]:bg-accent data-[state=on]:text-accent-text";
  const optionClass =
    "flex items-center gap-1 rounded border border-border px-2 py-1 text-xs text-text hover:bg-surface-raised aria-pressed:border-accent aria-pressed:ring-2 aria-pressed:ring-accent";
</script>

<div class="ml-2 flex items-center gap-1" role="group" aria-label={m.balloon_tools_label()}>
  <ToggleGroup.Root
    type="single"
    class="flex items-center gap-0.5"
    aria-label={m.tool_label()}
    value={balloonTools.tool}
    onValueChange={(value) => {
      if (value === "select" || value === "place") {
        balloonTools.setTool(value satisfies Tool);
      }
    }}
    disabled={!hasDocument}
  >
    <ToggleGroup.Item value="select" class={itemClass} title={hint("select_tool")}>
      {m.select_tool()}
    </ToggleGroup.Item>
    <ToggleGroup.Item value="place" class={itemClass} title={hint("place_tool")}>
      {m.place_tool()}
    </ToggleGroup.Item>
  </ToggleGroup.Root>

  <Popover.Root bind:open={balloonTools.styleOpen}>
    <Popover.Trigger class={buttonClass} disabled={count === 0} title={hint("restyle")}>
      {m.balloon_style()}
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content
        class="z-50 flex w-72 flex-col gap-3 rounded border border-border bg-surface p-3 text-sm text-text shadow-lg"
        sideOffset={4}
        aria-label={m.balloon_style()}
      >
        <p class="text-xs text-text-muted">
          {m.balloon_style_applies({ count: String(count) })}
        </p>
        <fieldset class="flex flex-col gap-1">
          <legend class="mb-1 text-xs font-semibold">{m.balloon_shape()}</legend>
          <div class="flex gap-1">
            {#each SHAPES as shape (shape)}
              <button
                type="button"
                class={optionClass}
                aria-pressed={current?.shape === shape}
                onclick={() => void balloonTools.restyle({ shape })}
              >
                <svg viewBox="0 0 24 16" class="h-3 w-5" aria-hidden="true">
                  {#if shape === "circle"}
                    <circle cx="12" cy="8" r="6.5" fill="none" stroke="currentColor" />
                  {:else if shape === "flag"}
                    <polygon
                      points="8,1.5 22,1.5 22,14.5 8,14.5 2,8"
                      fill="none"
                      stroke="currentColor"
                    />
                  {:else}
                    <rect x="2" y="1.5" width="20" height="13" fill="none" stroke="currentColor" />
                  {/if}
                </svg>
                {shapeLabel(shape)}
              </button>
            {/each}
          </div>
        </fieldset>
        <label class="flex items-center gap-2">
          <input
            type="checkbox"
            checked={current?.leader ?? true}
            onchange={(e) => void balloonTools.restyle({ leader: e.currentTarget.checked })}
          />
          {m.balloon_leader()}
        </label>
        <label class="flex items-center gap-2">
          {m.balloon_size()}
          <select
            class="rounded border border-border bg-surface px-2 py-0.5 text-text"
            value={String(current?.sizeMm ?? 7)}
            onchange={(e) => void balloonTools.restyle({ size_mm: Number(e.currentTarget.value) })}
          >
            {#each SIZES_MM as size (size)}
              <option value={String(size)}>{m.size_mm({ size: String(size) })}</option>
            {/each}
          </select>
        </label>
        <fieldset class="flex flex-col gap-1">
          <legend class="mb-1 text-xs font-semibold">{m.balloon_color()}</legend>
          <div class="flex flex-wrap gap-1">
            {#each COLORS as option (option.color)}
              <button
                type="button"
                class={optionClass}
                aria-pressed={current?.outlineColor === option.color}
                onclick={() => void balloonTools.restyle({ outline_color: option.color })}
              >
                <span
                  class="inline-block h-3 w-3 rounded-full border border-border"
                  style:background={option.color}
                  aria-hidden="true"
                ></span>
                {option.label()}
              </button>
            {/each}
          </div>
        </fieldset>
        <button
          type="button"
          class="self-start rounded border border-border px-2 py-1 text-xs hover:bg-surface-raised"
          onclick={() => void balloonTools.resetStyle()}
        >
          {m.balloon_style_reset()}
        </button>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>

  <button
    type="button"
    class={buttonClass}
    disabled={count === 0}
    title={hint("delete_selection")}
    onclick={() => void balloonTools.deleteSelection()}
  >
    {m.delete_selection()}
  </button>
</div>
