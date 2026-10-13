<script lang="ts">
  import { Popover } from "bits-ui";
  import { COLORS, SHAPES, SIZES_MM, shapeLabel } from "$lib/balloon-style-options";
  import { command, commandTooltip, disabledReason } from "$lib/commands.svelte";
  import ShapeIcon from "$lib/components/ShapeIcon.svelte";
  import { m } from "$lib/i18n";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";

  /**
   * "Balloon style" on the Home ribbon (FR-BAL-03, FR-BAL-12): a small ribbon button that opens
   * the style picker of the selected balloons. Label, icon, tooltip and disabled reason come
   * from the command registry; S opens the same picker.
   */
  const def = command("balloon_style");
  const reason = $derived(disabledReason("balloon_style"));
  const count = $derived(selection.size);
  /** Style of the primary selected balloon on this sheet, shown as the current choice. */
  const current = $derived(
    balloonGestures.rendered.find((b) => b.characteristic === selection.primary)?.style ??
      balloonGestures.rendered.find((b) => selection.ids.has(b.characteristic))?.style ??
      null,
  );

  const optionClass =
    "flex min-h-8 items-center gap-1 rounded border border-border px-2 py-1 text-xs text-text hover:bg-surface-raised aria-pressed:border-accent aria-pressed:ring-2 aria-pressed:ring-accent";
</script>

<Popover.Root
  bind:open={
    () => balloonTools.styleOpen && reason === null,
    (open) => {
      balloonTools.styleOpen = open;
    }
  }
>
  <Popover.Trigger
    class="flex min-h-8 items-center gap-1.5 rounded-md border border-transparent px-2 py-0.5 text-left text-[13px] whitespace-nowrap text-text hover:bg-hover aria-disabled:cursor-default aria-disabled:opacity-45 aria-disabled:hover:bg-transparent"
    title={commandTooltip("balloon_style")}
    aria-disabled={reason !== null ? "true" : undefined}
    data-command="balloon_style"
    onclick={(event: MouseEvent) => {
      if (reason !== null) {
        event.preventDefault();
      }
    }}
  >
    <def.icon size={18} strokeWidth={1.75} aria-hidden="true" class="shrink-0 text-text-muted" />
    <span>{def.label()}</span>
    <span aria-hidden="true" class="text-text-muted">▾</span>
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content
      class="z-50 flex w-72 flex-col gap-3 rounded border border-border bg-surface p-3 text-sm text-text shadow-lg"
      sideOffset={4}
      aria-label={def.label()}
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
              <ShapeIcon {shape} />
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
          class="min-h-8 rounded border border-border bg-surface px-2 py-0.5 text-text"
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
        class="min-h-8 self-start rounded border border-border px-2 py-1 text-xs hover:bg-surface-raised"
        onclick={() => void balloonTools.resetStyle()}
      >
        {m.balloon_style_reset()}
      </button>
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>
