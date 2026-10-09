<script lang="ts">
  import { untrack } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { DUMMY_STYLE, dummyBalloons, leaderLine } from "$lib/dev/dummy-balloons";
  import { commandErrorMessage, m } from "$lib/i18n";
  import { commands, type TileRange } from "$lib/ipc/bindings";
  import { documentStore } from "$lib/stores/document.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import {
    backdropZoomFor,
    fullTileRange,
    placeTiles,
    rangesKey,
    seamOverlap,
    tileRangeFor,
    tileZoomFor,
    zoomScale,
  } from "$lib/viewport/tiles";
  import {
    matrix,
    snapToDevicePixels,
    visibleSheetRect,
    wheelZoomFactor,
  } from "$lib/viewport/view-math";

  /** Wheel events of a trackpad pinch (ctrl + wheel) carry small deltas. */
  const PINCH_GAIN = 8;
  /** Tiles requested around the visible ones, so short pans show no empty tiles. */
  const PREFETCH_TILES = 1;
  /** Retries of a tile that came back empty, for example cancelled by an old interest. */
  const MAX_TILE_RETRIES = 3;

  let element = $state<HTMLElement | null>(null);
  let devicePixelRatio = $state(window.devicePixelRatio || 1);

  const doc = $derived(documentStore.current?.doc ?? null);
  const sheetIndex = $derived(documentStore.sheet);
  const sheet = $derived(documentStore.sheetSize);

  // The view the layers draw: the store's view with the translation on device pixels.
  const shown = $derived(snapToDevicePixels(viewport.view, devicePixelRatio));
  const zoom = $derived(tileZoomFor(shown.scale, devicePixelRatio));

  $effect(() => {
    viewport.sheet = sheet;
  });

  // Fit when a document or sheet is shown for the first time and the viewport has a size.
  let fittedFor = "";
  $effect(() => {
    const key = doc === null ? "" : `${doc}/${String(sheetIndex)}`;
    if (key !== "" && key !== fittedFor && viewport.size.width > 0 && viewport.size.height > 0) {
      fittedFor = key;
      untrack(() => {
        viewport.fit();
      });
    }
  });

  /** Tile ranges the view needs: the backdrop of the whole sheet and the visible tiles. */
  const wantedRanges = $derived.by((): TileRange[] => {
    if (doc === null || sheet === null || viewport.size.width <= 0) {
      return [];
    }
    const backdrop = backdropZoomFor(sheet);
    const ranges: TileRange[] = [fullTileRange(sheetIndex, sheet, backdrop)];
    if (zoom !== backdrop) {
      const area = visibleSheetRect(shown, viewport.size);
      const visible = tileRangeFor(sheetIndex, sheet, zoom, area, PREFETCH_TILES);
      if (visible) {
        ranges.push(visible);
      }
    }
    return ranges;
  });

  // Ranges whose tiles are in the DOM. They follow `wantedRanges` once Rust knows the new
  // interest, so new requests are never cancelled by the previous one.
  let requested = $state<{ key: string; doc: string; ranges: TileRange[] } | null>(null);
  let interestSequence = 0;
  $effect(() => {
    const ranges = wantedRanges;
    if (doc === null) {
      requested = null;
      return;
    }
    const key = rangesKey(doc, ranges);
    if (key === untrack(() => requested?.key)) {
      return;
    }
    const sequence = ++interestSequence;
    const target = doc;
    void commands
      .setTileInterest(target, ranges)
      .catch(() => undefined)
      .finally(() => {
        if (sequence === interestSequence) {
          requested = { key, doc: target, ranges };
        }
      });
  });

  const loaded = new SvelteSet<string>();
  const failed = new SvelteSet<string>();
  const retries = new SvelteMap<string, number>();

  $effect(() => {
    // A new document starts with empty tile state.
    void doc;
    untrack(() => {
      loaded.clear();
      failed.clear();
      retries.clear();
    });
  });

  const layers = $derived.by(() => {
    if (requested === null || sheet === null || requested.doc !== doc) {
      return [];
    }
    const current = requested;
    return current.ranges.map((range) => ({
      zoom: range.zoom,
      tiles: placeTiles(current.doc, sheet, range).filter((t) => !failed.has(t.url)),
    }));
  });

  function onTileError(url: string) {
    const count = (retries.get(url) ?? 0) + 1;
    retries.set(url, count);
    failed.add(url);
    if (count <= MAX_TILE_RETRIES) {
      // Removing the image and adding it again requests the tile once more.
      setTimeout(() => failed.delete(url), 150 * count);
    }
  }

  /** The sheet outline in screen space, under the tiles. */
  const sheetBox = $derived(
    sheet === null
      ? null
      : {
          x: shown.tx,
          y: shown.ty,
          width: sheet.width * shown.scale,
          height: sheet.height * shown.scale,
        },
  );

  const balloons = $derived(
    DEV_TOOLS_ENABLED && devTools.balloons && sheet !== null
      ? dummyBalloons(sheet, devTools.count, sheetIndex + 1, devTools.anchors)
      : [],
  );

  // Wheel zoom to the cursor. Added by hand because the listener must not be passive.
  $effect(() => {
    const el = element;
    if (!el) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      if (doc === null) {
        return;
      }
      event.preventDefault();
      const rect = el.getBoundingClientRect();
      const delta = event.ctrlKey ? event.deltaY * PINCH_GAIN : event.deltaY;
      viewport.zoomBy(wheelZoomFactor(delta, event.deltaMode, rect.height), {
        x: event.clientX - rect.left,
        y: event.clientY - rect.top,
      });
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      el.removeEventListener("wheel", onWheel);
    };
  });

  let drag: { pointer: number; x: number; y: number } | null = null;
  let dragging = $state(false);

  function onPointerDown(event: PointerEvent) {
    // Left or middle button pans.
    if (doc === null || (event.button !== 0 && event.button !== 1)) {
      return;
    }
    event.preventDefault();
    element?.focus({ preventScroll: true });
    element?.setPointerCapture(event.pointerId);
    drag = { pointer: event.pointerId, x: event.clientX, y: event.clientY };
    dragging = true;
  }

  function onPointerMove(event: PointerEvent) {
    if (drag?.pointer !== event.pointerId) {
      return;
    }
    viewport.panBy(event.clientX - drag.x, event.clientY - drag.y);
    drag = { pointer: event.pointerId, x: event.clientX, y: event.clientY };
  }

  function onPointerUp(event: PointerEvent) {
    if (drag?.pointer === event.pointerId) {
      drag = null;
      dragging = false;
    }
  }
</script>

<svelte:window
  onresize={() => {
    devicePixelRatio = window.devicePixelRatio || 1;
  }}
/>

<main
  bind:clientWidth={viewport.size.width}
  bind:clientHeight={viewport.size.height}
  class="relative min-h-0 min-w-0 flex-1 overflow-hidden bg-bg"
  aria-label={m.viewport_label()}
>
  {#if doc === null}
    <div class="flex h-full flex-col items-center justify-center gap-3 text-text-muted">
      <p>{documentStore.busy ? m.opening_drawing() : m.sample_greeting()}</p>
      {#if documentStore.error}
        <p class="max-w-md text-center text-sm" role="alert">
          {commandErrorMessage(documentStore.error)}
        </p>
      {/if}
      <button
        type="button"
        class="rounded bg-accent px-3 py-1.5 text-sm text-accent-text disabled:opacity-50"
        disabled={documentStore.busy}
        onclick={() => void documentStore.openWithDialog()}
      >
        {m.open_drawing()}
      </button>
    </div>
  {:else}
    {#if sheetBox}
      <div
        class="sheet absolute top-0 left-0"
        style:transform="translate({sheetBox.x}px, {sheetBox.y}px)"
        style:width="{sheetBox.width}px"
        style:height="{sheetBox.height}px"
      ></div>
    {/if}
    {#each layers as layer (layer.zoom)}
      {@const k = shown.scale / zoomScale(layer.zoom)}
      {@const overlap = seamOverlap(k, devicePixelRatio)}
      <div
        class="tile-layer absolute top-0 left-0"
        style:transform={matrix({ scale: k, tx: shown.tx, ty: shown.ty })}
      >
        {#each layer.tiles as tile (tile.url)}
          <img
            src={tile.url}
            alt=""
            draggable="false"
            decoding="async"
            class="tile"
            class:loaded={loaded.has(tile.url)}
            style:left="{tile.left}px"
            style:top="{tile.top}px"
            style:width="{tile.width + (tile.hasRight ? overlap : 0)}px"
            style:height="{tile.height + (tile.hasBelow ? overlap : 0)}px"
            onload={() => loaded.add(tile.url)}
            onerror={() => {
              onTileError(tile.url);
            }}
          />
        {/each}
      </div>
    {/each}
    <svg class="pointer-events-none absolute inset-0 h-full w-full" aria-hidden="true">
      <g transform={matrix(shown)}>
        {#each balloons as balloon (balloon.number)}
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
    <!-- The drawing surface on top of all layers: drag to pan, wheel to zoom, keys from the
         shortcut map. Focusable so keyboard users reach it (NFR-UX-01); "application" is the
         ARIA role for such a surface, but Svelte does not count it as interactive. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      bind:this={element}
      class="surface absolute inset-0 touch-none select-none"
      class:cursor-grab={!dragging}
      class:cursor-grabbing={dragging}
      role="application"
      aria-label={m.viewport_label()}
      tabindex="0"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
    ></div>
    {#if documentStore.error}
      <p
        class="absolute top-2 left-1/2 -translate-x-1/2 rounded border border-border bg-surface px-3 py-1 text-sm text-text"
        role="alert"
      >
        {commandErrorMessage(documentStore.error)}
      </p>
    {/if}
  {/if}
</main>

<style>
  .sheet {
    background: #ffffff;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.35);
    transform-origin: 0 0;
  }

  .tile-layer {
    transform-origin: 0 0;
    pointer-events: none;
  }

  .sheet {
    pointer-events: none;
  }

  .surface:focus-visible {
    outline-offset: -2px;
  }

  .tile {
    position: absolute;
    max-width: none;
    visibility: hidden;
  }

  .tile.loaded {
    visibility: visible;
  }

  .balloon-number {
    fill: #000000;
    font-family: system-ui, sans-serif;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: central;
  }
</style>
