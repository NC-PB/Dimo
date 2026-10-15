<script lang="ts">
  /**
   * Dummy balloons of the viewport performance check (T0.8), drawn over the sheet. Development
   * builds only: `Viewport.svelte` loads this component through a dynamic import.
   */
  import { devTools } from "$lib/dev/dev-tools.svelte";
  import { DUMMY_STYLE, dummyBalloons, leaderLine } from "$lib/dev/dummy-balloons";
  import { matrix, type Size, type ViewTransform } from "$lib/viewport/view-math";

  interface Props {
    view: ViewTransform;
    sheet: Size;
    /** Index of the sheet, so every sheet shows its own set. */
    sheetIndex: number;
  }

  let { view, sheet, sheetIndex }: Props = $props();

  const dummies = $derived(
    devTools.balloons
      ? dummyBalloons(sheet, devTools.count, sheetIndex + 1, devTools.anchors).map((b) => ({
          key: `dummy-${String(b.number)}`,
          balloon: b,
        }))
      : [],
  );
</script>

{#if dummies.length > 0}
  <svg class="pointer-events-none absolute inset-0 h-full w-full" aria-hidden="true">
    <g transform={matrix(view)}>
      {#each dummies as { key, balloon } (key)}
        {@const leader = leaderLine(balloon)}
        {#if leader}
          <line
            x1={leader.from.x}
            y1={leader.from.y}
            x2={leader.to.x}
            y2={leader.to.y}
            stroke={DUMMY_STYLE.color}
            stroke-width={DUMMY_STYLE.stroke}
          />
        {/if}
        <circle
          cx={balloon.center.x}
          cy={balloon.center.y}
          r={DUMMY_STYLE.radius}
          fill="#ffffff"
          stroke={DUMMY_STYLE.color}
          stroke-width={DUMMY_STYLE.stroke}
        />
        <text
          x={balloon.center.x}
          y={balloon.center.y}
          font-size={DUMMY_STYLE.fontSize}
          class="balloon-number">{balloon.number}</text
        >
      {/each}
    </g>
  </svg>
{/if}

<style>
  .balloon-number {
    fill: #000000;
    font-family: system-ui, sans-serif;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: central;
  }
</style>
