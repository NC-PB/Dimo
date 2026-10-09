<script lang="ts">
  import { m } from "$lib/i18n";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";
  import { sheetToScreen, type ViewTransform } from "$lib/viewport/view-math";

  interface Props {
    /** The view transform of the overlay. */
    view: ViewTransform;
    /** Called when the editor closes, to give the keyboard focus back to the drawing. */
    onDone: () => void;
  }

  let { view, onDone }: Props = $props();

  /** Space between the balloon and the editor, in CSS px. */
  const GAP_PX = 8;
  /** Width the editor needs on screen, to decide on which side of the balloon it fits. */
  const EDITOR_WIDTH_PX = 250;

  const id = $derived(balloonTools.editing);
  const characteristic = $derived(
    id === null ? undefined : projectStore.characteristicById.get(id),
  );
  const balloon = $derived(
    id === null ? undefined : balloonGestures.shown.find((b) => b.characteristic === id),
  );

  /** Screen position right of the balloon, also on a rotated sheet. */
  const at = $derived.by(() => {
    if (!balloon) {
      return null;
    }
    const g = balloon.geometry;
    const center = sheetToScreen(view, g.center);
    const across = view.rotation === 90 || view.rotation === 270 ? g.height : g.width;
    const half = (across / 2) * view.scale + GAP_PX;
    const right = center.x + half + EDITOR_WIDTH_PX <= viewport.size.width;
    return { x: right ? center.x + half : center.x - half, y: center.y, right };
  });

  // Closes the editor when its characteristic is gone (undo, delete) or on another sheet.
  $effect(() => {
    if (id !== null && (characteristic === undefined || balloon === undefined)) {
      balloonTools.stopEditing();
    }
  });

  let text = $state("");
  let input = $state<HTMLInputElement | null>(null);
  let shownFor: string | null = null;

  // A new editor starts with the stored text, selected, so typing replaces it.
  $effect(() => {
    if (id !== null && id !== shownFor && input) {
      shownFor = id;
      text = characteristic?.requirement_text ?? "";
      input.focus({ preventScroll: true });
      input.select();
    }
    if (id === null) {
      shownFor = null;
    }
  });

  /** Stores the text and closes. Runs once per editor, whether by Enter or by leaving. */
  function commit(): void {
    const target = shownFor;
    if (target === null || balloonTools.editing !== target) {
      return;
    }
    balloonTools.stopEditing();
    void balloonTools.commitText(target, text);
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.isComposing) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
      onDone();
    } else if (event.key === "Escape") {
      // Closes without storing; the balloon stays (undo removes it).
      event.preventDefault();
      event.stopPropagation();
      balloonTools.stopEditing();
      onDone();
    }
  }
</script>

{#if id !== null && characteristic && at}
  <div
    class="absolute z-10 flex items-center gap-1.5 rounded border border-border bg-surface px-2 py-1 shadow-md"
    style:left="{at.x}px"
    style:top="{at.y}px"
    style:transform="translate({at.right ? '0' : '-100%'}, -50%)"
  >
    <label for="balloon-value" class="text-sm font-semibold text-text tabular-nums">
      {m.balloon_value_label({ number: String(characteristic.number) })}
    </label>
    <input
      id="balloon-value"
      bind:this={input}
      bind:value={text}
      type="text"
      autocomplete="off"
      spellcheck="false"
      class="w-44 rounded border border-border bg-surface px-1.5 py-0.5 text-sm text-text"
      placeholder={m.balloon_value_placeholder()}
      onkeydown={onKeyDown}
      onblur={commit}
    />
  </div>
{/if}
