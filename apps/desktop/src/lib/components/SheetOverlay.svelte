<script lang="ts">
  import { m } from "$lib/i18n";
  import { documentStore } from "$lib/stores/document.svelte";
  import { numberingStore } from "$lib/stores/numbering.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { sheetRegions } from "$lib/stores/sheet-regions.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";
  import { matrix, screenToSheet, type ViewTransform } from "$lib/viewport/view-math";
  import { frameOf, gridLines, labelPositions, rectBetween } from "$lib/zone-grid";

  /**
   * Zone grid, views and ghost numbers of the shown sheet (T2.7, FR-BAL-05), and the rectangle
   * tools of the sheet properties. Drawn in sheet space under the view transform, like the
   * balloons. While a rectangle tool is active, pointer events on the drawing go to the tool.
   */
  interface Props {
    view: ViewTransform;
  }

  let { view }: Props = $props();

  let svg = $state<SVGSVGElement | null>(null);

  const sheet = $derived(projectStore.sheets[documentStore.sheet]);
  const grid = $derived(sheet?.zone_grid ?? null);
  const lines = $derived(grid ? gridLines(grid) : null);
  const labels = $derived(grid ? labelPositions(grid) : []);
  const frame = $derived(grid ? frameOf(grid.frame) : null);
  const views = $derived((sheet?.views ?? []).map((v) => frameOf(v.rect)));

  /** Sheet units per CSS pixel, for strokes and text that keep their screen size. */
  const px = $derived(1 / view.scale);

  /** Ghost numbers next to the balloons whose number the preview changes or keeps. */
  const ghosts = $derived.by(() => {
    const ghosts = numberingStore.ghosts;
    return balloonGestures.shown.flatMap((b) => {
      const number = ghosts[b.characteristic];
      if (number === undefined) {
        return [];
      }
      const g = b.geometry;
      return [
        {
          id: b.id,
          number,
          changed: number !== b.text,
          at: { x: g.center.x + g.width / 2, y: g.center.y - g.height / 2 },
          size: g.height,
        },
      ];
    });
  });

  const dragRect = $derived(
    sheetRegions.drag ? frameOf(rectBetween(sheetRegions.drag.from, sheetRegions.drag.to)) : null,
  );

  /** Turns text back against the view rotation, so it stays upright on screen. */
  function upright(at: { x: number; y: number }): string | undefined {
    return view.rotation === 0
      ? undefined
      : `rotate(${String(-view.rotation)} ${String(at.x)} ${String(at.y)})`;
  }

  // The rectangle tools take the pointer events of the drawing surface before its own gestures
  // run (capture phase on the viewport), also the synthetic ones of the dev UI script.
  $effect(() => {
    const host = svg?.closest("main");
    if (!host) {
      return;
    }
    const at = (event: PointerEvent) => {
      const rect = host.getBoundingClientRect();
      return screenToSheet(view, { x: event.clientX - rect.left, y: event.clientY - rect.top });
    };
    const onDown = (event: PointerEvent) => {
      if (sheetRegions.tool === null || event.button !== 0) {
        return;
      }
      event.stopPropagation();
      event.preventDefault();
      sheetRegions.begin(at(event));
    };
    const onMove = (event: PointerEvent) => {
      if (sheetRegions.drag !== null) {
        event.stopPropagation();
        sheetRegions.moveTo(at(event));
      }
    };
    const onUp = (event: PointerEvent) => {
      if (sheetRegions.drag !== null) {
        event.stopPropagation();
        void sheetRegions.finish(at(event));
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && sheetRegions.tool !== null) {
        event.stopPropagation();
        sheetRegions.cancel();
      }
    };
    host.addEventListener("pointerdown", onDown, true);
    host.addEventListener("pointermove", onMove, true);
    host.addEventListener("pointerup", onUp, true);
    window.addEventListener("keydown", onKey, true);
    return () => {
      host.removeEventListener("pointerdown", onDown, true);
      host.removeEventListener("pointermove", onMove, true);
      host.removeEventListener("pointerup", onUp, true);
      window.removeEventListener("keydown", onKey, true);
    };
  });

  // Crosshair while a tool is active; the surface reads this attribute in its style below.
  $effect(() => {
    const host = svg?.closest("main");
    if (host) {
      host.toggleAttribute("data-region-tool", sheetRegions.tool !== null);
    }
  });
</script>

<!-- Hidden from assistive technology: the sheet properties list the grid and the views, the
     table lists the new numbers. -->
<svg
  bind:this={svg}
  class="pointer-events-none absolute inset-0 h-full w-full"
  aria-hidden="true"
  data-testid="sheet-overlay"
>
  <g transform={matrix(view)}>
    {#if frame && lines}
      <rect
        class="zone"
        x={frame.x}
        y={frame.y}
        width={frame.width}
        height={frame.height}
        stroke-width={1 * px}
      />
      {#each lines.xs as x (x)}
        <line
          class="zone"
          x1={x}
          y1={frame.y}
          x2={x}
          y2={frame.y + frame.height}
          stroke-width={1 * px}
          stroke-dasharray="{6 * px} {4 * px}"
        />
      {/each}
      {#each lines.ys as y (y)}
        <line
          class="zone"
          x1={frame.x}
          y1={y}
          x2={frame.x + frame.width}
          y2={y}
          stroke-width={1 * px}
          stroke-dasharray="{6 * px} {4 * px}"
        />
      {/each}
      {#each labels as label, i (i)}
        <!-- Column labels just above the frame, row labels just left of it. -->
        {@const at =
          label.axis === "columns"
            ? { x: label.at.x, y: label.at.y - 8 * px }
            : { x: label.at.x - 8 * px, y: label.at.y }}
        <text class="zone-label" x={at.x} y={at.y} font-size={11 * px} transform={upright(at)}
          >{label.text}</text
        >
      {/each}
    {/if}

    {#each views as v, i (i)}
      <rect
        class="view"
        x={v.x}
        y={v.y}
        width={v.width}
        height={v.height}
        stroke-width={1.5 * px}
        stroke-dasharray="{2 * px} {3 * px}"
      />
      <text
        class="view-label"
        x={v.x + 4 * px}
        y={v.y + 12 * px}
        font-size={11 * px}
        transform={upright({ x: v.x + 4 * px, y: v.y + 12 * px })}
        >{sheet?.views[i]?.label || m.views_unnamed({ number: String(i + 1) })}</text
      >
    {/each}

    {#each ghosts as ghost (ghost.id)}
      <!-- Ghost number: a dashed box with the new number, bold when it changes (shape and
           weight, not color alone). -->
      <g transform={upright(ghost.at)}>
        <rect
          class="ghost"
          class:changed={ghost.changed}
          x={ghost.at.x}
          y={ghost.at.y - ghost.size * 0.35}
          width={ghost.size * (0.45 + 0.3 * ghost.number.length)}
          height={ghost.size * 0.7}
          rx={ghost.size * 0.12}
          stroke-width={1.2 * px}
          stroke-dasharray="{3 * px} {2 * px}"
        />
        <text
          class="ghost-number"
          class:changed={ghost.changed}
          x={ghost.at.x + (ghost.size * (0.45 + 0.3 * ghost.number.length)) / 2}
          y={ghost.at.y}
          font-size={ghost.size * 0.5}>{ghost.number}</text
        >
      </g>
    {/each}

    {#if dragRect}
      <rect
        class="drawing"
        x={dragRect.x}
        y={dragRect.y}
        width={dragRect.width}
        height={dragRect.height}
        stroke-width={1.5 * px}
        stroke-dasharray="{4 * px} {3 * px}"
      />
    {/if}
  </g>
</svg>

<style>
  .zone {
    fill: none;
    stroke: var(--dimo-balloon-rejected);
  }

  .zone-label,
  .view-label {
    fill: var(--dimo-balloon-region);
    font-family: system-ui, sans-serif;
    font-weight: 600;
  }

  .zone-label {
    text-anchor: middle;
    dominant-baseline: central;
  }

  .view {
    fill: none;
    stroke: var(--dimo-balloon-region);
  }

  .ghost {
    fill: #ffffff;
    stroke: var(--dimo-balloon-rejected);
  }

  .ghost.changed {
    stroke: var(--dimo-balloon-selected);
  }

  .ghost-number {
    fill: var(--dimo-balloon-rejected);
    font-family: "Open Sans", system-ui, sans-serif;
    font-style: italic;
    text-anchor: middle;
    dominant-baseline: central;
  }

  .ghost-number.changed {
    fill: var(--dimo-balloon-selected);
    font-weight: 700;
  }

  .drawing {
    fill: var(--dimo-balloon-region-fill);
    stroke: var(--dimo-balloon-region);
  }

  :global(main[data-region-tool] .surface) {
    cursor: crosshair !important;
  }
</style>
