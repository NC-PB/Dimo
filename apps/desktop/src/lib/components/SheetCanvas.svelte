<script lang="ts" module>
  import type { Size } from "$lib/viewport/view-math";

  /** The page a canvas shows: a document of the tile service, its page and the page size. */
  export interface TileSource {
    /** Document key of the tile service (`DocumentInfo.doc`). */
    doc: string;
    /** Page index in the document. */
    page: number;
    /** Size of the page on screen in sheet units. */
    size: Size;
  }
</script>

<script lang="ts">
  /**
   * One sheet on screen (FR-DOC-04, MF F8): the tiles of the tile service (`dimo://tile/...`)
   * under one view transform, the sheet outline, wheel and pinch zoom to the cursor, and a fit
   * when a page is shown for the first time. Pan and zoom live in the `ViewportStore` it is
   * given, so buttons, keys and gestures change the view in one place. Everything drawn in sheet
   * space on top (balloons, handles of the image editor) goes into the `overlay` snippet, which
   * gets the view transform the tiles use, snapped to device pixels.
   *
   * The drawing viewport (`Viewport.svelte`) is a canvas plus the balloon layer and gestures.
   */
  import { untrack, type Snippet } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import { commands, type TileRange } from "$lib/ipc/bindings";
  import type { ViewportStore } from "$lib/stores/viewport.svelte";
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
    gestureZoomFactor,
    matrix,
    sheetScreenRect,
    snapToDevicePixels,
    viewRotation,
    visibleSheetRect,
    wheelZoomFactor,
    type ViewTransform,
  } from "$lib/viewport/view-math";
  import type { Rotation } from "$lib/ipc/bindings";

  interface Props {
    /** Pan, zoom and rotation of this canvas. */
    store: ViewportStore;
    /** The page to show, `null` while there is none. */
    source: TileSource | null;
    /** Quarter turn of the page on screen (FR-DOC-05); the view follows it. */
    rotation?: Rotation;
    /** Accessible name of the canvas. */
    label: string;
    /** The element of the canvas: `main` for the drawing area. */
    tag?: "div" | "main";
    /**
     * Called the first time a page is shown at a size. Returns whether it set the view itself
     * (for example to show a balloon); otherwise the canvas fits the page.
     */
    onFirstShow?: (source: TileSource) => boolean;
    /** Layers in sheet space on top of the tiles; gets the view transform of the tiles. */
    overlay?: Snippet<[ViewTransform]>;
    /** Shown while there is no page. */
    empty?: Snippet;
    /** The canvas element, for its owner (focus, shortcut scope). */
    element?: HTMLElement | null;
  }

  let {
    store,
    source,
    rotation = "deg0",
    label,
    tag = "div",
    onFirstShow,
    overlay,
    empty,
    element = $bindable(null),
  }: Props = $props();

  /** Tiles requested around the visible ones, so short pans show no empty tiles. */
  const PREFETCH_TILES = 1;
  /** Retries of a tile that came back empty, for example cancelled by an old interest. */
  const MAX_TILE_RETRIES = 3;

  let devicePixelRatio = $state(window.devicePixelRatio || 1);

  const doc = $derived(source?.doc ?? null);
  const page = $derived(source?.page ?? 0);
  const sheet = $derived(source?.size ?? null);

  // The view the layers draw: the store's view with the translation on device pixels.
  const shown = $derived(snapToDevicePixels(store.view, devicePixelRatio));
  const zoom = $derived(tileZoomFor(shown.scale, devicePixelRatio));

  $effect(() => {
    store.sheet = sheet;
  });

  // The view follows the rotation of the page (FR-DOC-05), also for undo and redo. Tiles and
  // overlay share the one view transform, so both turn together. Declared before the fit below,
  // so a new page is fitted with its own rotation.
  const turn = $derived(viewRotation(rotation));
  $effect(() => {
    const next = turn;
    untrack(() => {
      store.setRotation(next);
    });
  });

  // Fit when a page is shown for the first time and the canvas has a size, unless the owner
  // sets the view itself.
  let fittedFor = "";
  $effect(() => {
    const current = source;
    const key = current === null ? "" : `${current.doc}/${String(current.page)}`;
    if (current !== null && key !== fittedFor && store.size.width > 0 && store.size.height > 0) {
      fittedFor = key;
      untrack(() => {
        if (onFirstShow?.(current) !== true) {
          store.fit();
        }
      });
    }
  });

  /** Tile ranges the view needs: the backdrop of the whole page and the visible tiles. */
  const wantedRanges = $derived.by((): TileRange[] => {
    if (doc === null || sheet === null || store.size.width <= 0) {
      return [];
    }
    const backdrop = backdropZoomFor(sheet);
    const ranges: TileRange[] = [fullTileRange(page, sheet, backdrop)];
    if (zoom !== backdrop) {
      const area = visibleSheetRect(shown, store.size);
      const visible = tileRangeFor(page, sheet, zoom, area, PREFETCH_TILES);
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

  /** The page outline in screen space (turned with the view), under the tiles. */
  const sheetBox = $derived(sheet === null ? null : sheetScreenRect(shown, sheet));

  // Wheel and pinch zoom to the cursor. Added by hand because the listeners must not be passive.
  // A pinch arrives as wheel events with ctrlKey (Chromium, WebKit in Tauri) or, in Safari, as
  // gesture events. While a gesture runs its wheel events are ignored, so nothing zooms twice.
  $effect(() => {
    const el = element;
    if (!el) {
      return;
    }
    const at = (event: { clientX: number; clientY: number }) => {
      const rect = el.getBoundingClientRect();
      return { x: event.clientX - rect.left, y: event.clientY - rect.top };
    };
    let gestureScale: number | null = null;
    const onWheel = (event: WheelEvent) => {
      if (doc === null) {
        return;
      }
      event.preventDefault();
      if (gestureScale !== null && event.ctrlKey) {
        return;
      }
      store.zoomBy(wheelZoomFactor(event, el.getBoundingClientRect().height), at(event));
    };
    // Safari gesture events are not in the DOM typings.
    type GestureEvent = Event & { scale: number; clientX: number; clientY: number };
    const onGestureStart = (event: Event) => {
      event.preventDefault();
      gestureScale = 1;
    };
    const onGestureChange = (event: Event) => {
      event.preventDefault();
      const gesture = event as GestureEvent;
      if (doc === null || gestureScale === null) {
        return;
      }
      store.zoomBy(gestureZoomFactor(gestureScale, gesture.scale), at(gesture));
      gestureScale = gesture.scale;
    };
    const onGestureEnd = (event: Event) => {
      event.preventDefault();
      gestureScale = null;
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("gesturestart", onGestureStart);
    el.addEventListener("gesturechange", onGestureChange);
    el.addEventListener("gestureend", onGestureEnd);
    return () => {
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("gesturestart", onGestureStart);
      el.removeEventListener("gesturechange", onGestureChange);
      el.removeEventListener("gestureend", onGestureEnd);
    };
  });
</script>

<svelte:window
  onresize={() => {
    devicePixelRatio = window.devicePixelRatio || 1;
  }}
/>

<svelte:element
  this={tag}
  bind:this={element}
  bind:clientWidth={store.size.width}
  bind:clientHeight={store.size.height}
  class="relative min-h-0 min-w-0 flex-1 overflow-hidden bg-sunken"
  aria-label={label}
  data-sheet-canvas
>
  {#if source === null}
    {@render empty?.()}
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
        style:transform={matrix({ ...shown, scale: k })}
        data-zoom={layer.zoom}
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
    {@render overlay?.(shown)}
  {/if}
</svelte:element>

<style>
  .sheet {
    /* Paper is white in both themes; the drawing is printed on it. */
    background: #ffffff;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.35);
    transform-origin: 0 0;
    pointer-events: none;
  }

  .tile-layer {
    transform-origin: 0 0;
    pointer-events: none;
  }

  .tile {
    position: absolute;
    max-width: none;
    visibility: hidden;
  }

  .tile.loaded {
    visibility: visible;
  }
</style>
