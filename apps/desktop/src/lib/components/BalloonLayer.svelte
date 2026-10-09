<script lang="ts">
  import { selection } from "$lib/stores/selection.svelte";
  import { grown, leaderStart, polygon, svgPoints } from "$lib/viewport/balloons";
  import {
    ANCHOR_HANDLE_PX,
    balloonGestures,
    type RenderedBalloon,
  } from "$lib/viewport/gestures.svelte";
  import { matrix, type ViewTransform } from "$lib/viewport/view-math";

  interface Props {
    /** The view transform of the tile layers: one transform for the whole overlay. */
    view: ViewTransform;
  }

  let { view }: Props = $props();

  /** Sheet units per CSS pixel, for strokes and handles that keep their screen size. */
  const px = $derived(1 / view.scale);

  const balloons = $derived(balloonGestures.shown);
  const selected = $derived(balloons.filter((b) => selection.ids.has(b.characteristic)));
  const dragRect = $derived(balloonGestures.dragRect);

  /** Turns the number back against the view rotation, so it stays upright on screen. */
  function upright(at: { x: number; y: number }): string | undefined {
    return view.rotation === 0
      ? undefined
      : `rotate(${String(-view.rotation)} ${String(at.x)} ${String(at.y)})`;
  }

  /** Proposed and rejected balloons get a dashed outline: status by shape plus color. */
  function dashed(b: RenderedBalloon): string | undefined {
    if (b.status === "rejected" || b.status === "proposed") {
      const dash = b.geometry.height / 7;
      return `${String(dash)} ${String(dash * 0.6)}`;
    }
    return undefined;
  }
</script>

<!-- Balloons of the shown sheet (FR-BAL-03, D-24) in sheet space under one transform. Hidden
     from assistive technology; the characteristic table is the accessible list. -->
<svg class="pointer-events-none absolute inset-0 h-full w-full" aria-hidden="true">
  <g transform={matrix(view)}>
    <!-- Leaders first, so they never cross a balloon (as in the ballooned PDF). -->
    {#each balloons as b (b.id)}
      {#if b.style.leader}
        {@const from = leaderStart(b.geometry, b.anchor)}
        {#if from}
          <line
            x1={from.x}
            y1={from.y}
            x2={b.anchor.x}
            y2={b.anchor.y}
            class:rejected-stroke={b.status === "rejected"}
            stroke={b.style.outlineColor}
            stroke-width={b.geometry.stroke}
            stroke-dasharray={dashed(b)}
          />
        {/if}
      {/if}
    {/each}

    <!-- Selection halo: a thick ring around the shape, so selection is not color alone. -->
    {#each selected as b (b.id)}
      {@const halo = grown(b.geometry, 3 * px)}
      {#if halo.shape === "circle"}
        <ellipse
          class="halo"
          cx={halo.center.x}
          cy={halo.center.y}
          rx={halo.width / 2}
          ry={halo.height / 2}
          stroke-width={3 * px}
        />
      {:else}
        <polygon class="halo" points={svgPoints(polygon(halo))} stroke-width={3 * px} />
      {/if}
    {/each}

    {#each balloons as b (b.id)}
      {@const g = b.geometry}
      {@const rejected = b.status === "rejected"}
      {#if g.shape === "circle"}
        <ellipse
          cx={g.center.x}
          cy={g.center.y}
          rx={g.width / 2}
          ry={g.height / 2}
          fill={b.style.fillColor}
          stroke={b.style.outlineColor}
          stroke-width={g.stroke}
          stroke-dasharray={dashed(b)}
          class:rejected-stroke={rejected}
        />
      {:else}
        <polygon
          points={svgPoints(polygon(g))}
          fill={b.style.fillColor}
          stroke={b.style.outlineColor}
          stroke-width={g.stroke}
          stroke-dasharray={dashed(b)}
          stroke-linejoin="miter"
          class:rejected-stroke={rejected}
        />
      {/if}
      <text
        x={g.center.x}
        y={g.center.y}
        font-size={g.fontSize}
        transform={upright(g.center)}
        fill={b.style.textColor}
        class="balloon-number"
        class:rejected-text={rejected}>{b.text}</text
      >
      {#if rejected}
        <!-- Rejected: struck through, besides the grey dashed outline (NFR-UX-05). -->
        <line
          class="rejected-stroke"
          x1={g.center.x - g.width / 2}
          y1={g.center.y + g.height / 2}
          x2={g.center.x + g.width / 2}
          y2={g.center.y - g.height / 2}
          stroke-width={g.stroke}
        />
      {/if}
    {/each}

    <!-- Leader anchor handles of selected balloons, dragged to move the anchor. -->
    {#each selected as b (b.id)}
      {#if b.style.leader}
        <rect
          class="handle"
          x={b.anchor.x - ANCHOR_HANDLE_PX * px}
          y={b.anchor.y - ANCHOR_HANDLE_PX * px}
          width={2 * ANCHOR_HANDLE_PX * px}
          height={2 * ANCHOR_HANDLE_PX * px}
          stroke-width={1.5 * px}
        />
      {/if}
    {/each}

    {#if dragRect}
      <rect
        class={dragRect.kind === "box" ? "box-select" : "region"}
        x={dragRect.rect.x}
        y={dragRect.rect.y}
        width={dragRect.rect.width}
        height={dragRect.rect.height}
        stroke-width={1.5 * px}
        stroke-dasharray="{4 * px} {3 * px}"
      />
    {/if}
  </g>
</svg>

<style>
  .balloon-number {
    font-family: "Open Sans", system-ui, sans-serif;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: central;
  }

  .halo {
    fill: none;
    stroke: var(--dimo-balloon-selected);
  }

  .handle {
    fill: #ffffff;
    stroke: var(--dimo-balloon-selected);
  }

  .rejected-stroke {
    stroke: var(--dimo-balloon-rejected);
  }

  .rejected-text {
    fill: var(--dimo-balloon-rejected);
  }

  .box-select {
    fill: var(--dimo-balloon-selected-fill);
    stroke: var(--dimo-balloon-selected);
  }

  .region {
    fill: var(--dimo-balloon-region-fill);
    stroke: var(--dimo-balloon-region);
  }
</style>
