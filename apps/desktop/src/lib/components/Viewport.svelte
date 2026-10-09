<script lang="ts">
  import { untrack } from "svelte";
  import BalloonEditor from "$lib/components/BalloonEditor.svelte";
  import BalloonLayer from "$lib/components/BalloonLayer.svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { DUMMY_STYLE, dummyBalloons, leaderLine } from "$lib/dev/dummy-balloons";
  import { m } from "$lib/i18n";
  import { commands, type TileRange } from "$lib/ipc/bindings";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonGestures, type PointerInput } from "$lib/viewport/gestures.svelte";
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
  import { revealPlan, viewAfterSwitch } from "$lib/viewport/reveal";
  import {
    centerOn,
    matrix,
    sheetScreenRect,
    snapToDevicePixels,
    viewRotation,
    visibleSheetRect,
    wheelZoomFactor,
    gestureZoomFactor,
    type Point,
  } from "$lib/viewport/view-math";

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

  // The view follows the rotation stored on the sheet (FR-DOC-05), also for undo and redo. Tiles
  // and overlay share the one view transform, so both turn together. Declared before the fit
  // below, so a new sheet is fitted with its own rotation.
  const sheetRotation = $derived(viewRotation(projectStore.sheets[sheetIndex]?.rotation ?? "deg0"));
  $effect(() => {
    const rotation = sheetRotation;
    untrack(() => {
      viewport.setRotation(rotation);
    });
  });

  /**
   * A balloon to show once the sheet switch of a reveal has happened: its sheet index, center
   * in sheet space and the zoom before the switch.
   */
  let pendingReveal: { sheet: number; center: Point; scale: number } | null = null;

  // Fit when a document or sheet is shown for the first time and the viewport has a size. A
  // sheet switched to for a reveal shows the balloon instead.
  let fittedFor = "";
  $effect(() => {
    const key = doc === null ? "" : `${doc}/${String(sheetIndex)}`;
    if (key !== "" && key !== fittedFor && viewport.size.width > 0 && viewport.size.height > 0) {
      fittedFor = key;
      untrack(() => {
        const reveal = pendingReveal;
        pendingReveal = null;
        if (reveal !== null && reveal.sheet === sheetIndex && viewport.sheet !== null) {
          viewport.set(
            viewAfterSwitch(
              reveal.center,
              reveal.scale,
              viewport.sheet,
              viewport.size,
              viewport.view.rotation,
            ),
          );
        } else {
          viewport.fit();
        }
      });
    }
  });

  // A characteristic selected elsewhere (the table): show its sheet, and its balloon if it is
  // out of view (T1.7). Group selections do not move the view, see `revealPlan`.
  $effect(() => {
    const ids = selection.ids;
    untrack(() => {
      if (viewport.size.width <= 0) {
        return;
      }
      const plan = revealPlan(
        ids,
        projectStore.project?.balloons ?? [],
        projectStore.sheets,
        documentStore.sheet,
        visibleSheetRect(viewport.view, viewport.size),
      );
      if (plan.kind === "switch") {
        // The fit effect above shows the balloon once the new sheet is set up.
        pendingReveal = { sheet: plan.sheet, center: plan.center, scale: viewport.view.scale };
        documentStore.setSheet(plan.sheet);
      } else if (plan.kind === "center") {
        viewport.set(
          centerOn(plan.center, viewport.view.scale, viewport.size, viewport.view.rotation),
        );
      }
    });
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

  /** The sheet outline in screen space (turned with the view), under the tiles. */
  const sheetBox = $derived(sheet === null ? null : sheetScreenRect(shown, sheet));

  /** Dummy balloons of the performance check (development only, T0.8). */
  const dummies = $derived(
    DEV_TOOLS_ENABLED && devTools.balloons && sheet !== null
      ? dummyBalloons(sheet, devTools.count, sheetIndex + 1, devTools.anchors).map((b) => ({
          key: `dummy-${String(b.number)}`,
          balloon: b,
        }))
      : [],
  );

  // Selected characteristics that were deleted (also by undo) leave the selection.
  $effect(() => {
    const exists = projectStore.characteristicById;
    untrack(() => {
      selection.retain((id) => exists.has(id));
    });
  });

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
      viewport.zoomBy(wheelZoomFactor(event, el.getBoundingClientRect().height), at(event));
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
      viewport.zoomBy(gestureZoomFactor(gestureScale, gesture.scale), at(gesture));
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

  /** Pointer position relative to the viewport element. */
  function pointerInput(event: PointerEvent): PointerInput {
    const rect = element?.getBoundingClientRect();
    return {
      pointerId: event.pointerId,
      button: event.button,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
      ctrlKey: event.ctrlKey,
      x: event.clientX - (rect?.left ?? 0),
      y: event.clientY - (rect?.top ?? 0),
      timeStamp: event.timeStamp,
    };
  }

  // Pointer gestures (T1.6): select, move, place, box select and pan, see `gestures.svelte.ts`.
  function onPointerDown(event: PointerEvent) {
    if (doc === null) {
      return;
    }
    element?.focus({ preventScroll: true });
    if (balloonGestures.down(pointerInput(event))) {
      event.preventDefault();
      try {
        element?.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic events of the dev UI script have no active pointer to capture.
      }
    }
  }

  function onPointerMove(event: PointerEvent) {
    balloonGestures.move(pointerInput(event));
  }

  function onPointerUp(event: PointerEvent) {
    void balloonGestures.up(pointerInput(event));
  }

  function onPointerCancel() {
    balloonGestures.cancel();
  }

  /** Space held: the left button pans in every tool. Not while typing in a field. */
  function onSpace(event: KeyboardEvent, down: boolean) {
    const target = event.target;
    const typing =
      target instanceof HTMLElement &&
      (target.isContentEditable || target.closest("input, select, textarea, button") !== null);
    // A Space the table used (toggling a row) does not arm panning.
    if (event.key === " " && (!down || (!typing && !event.defaultPrevented))) {
      balloonGestures.spaceHeld = down;
      if (down && target === element) {
        event.preventDefault();
      }
    }
  }

  const cursor = $derived.by(() => {
    if (balloonGestures.panning) {
      return "grabbing";
    }
    if (balloonGestures.gesture?.kind === "move" || balloonGestures.hover === "balloon") {
      return "move";
    }
    if (balloonGestures.hover === "anchor" || balloonGestures.gesture?.kind === "anchor") {
      return "crosshair";
    }
    if (balloonGestures.spaceHeld) {
      return "grab";
    }
    return balloonTools.tool === "place" ? "crosshair" : "default";
  });

  function focusDrawing() {
    element?.focus({ preventScroll: true });
  }
</script>

<svelte:window
  onresize={() => {
    devicePixelRatio = window.devicePixelRatio || 1;
  }}
  onkeydown={(e) => {
    onSpace(e, true);
  }}
  onkeyup={(e) => {
    onSpace(e, false);
  }}
  onblur={() => {
    balloonGestures.spaceHeld = false;
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
      <p>{projectStore.busy ? m.working() : m.sample_greeting()}</p>
      <div class="flex gap-2">
        <button
          type="button"
          class="rounded bg-accent px-3 py-1.5 text-sm text-accent-text disabled:opacity-50"
          disabled={projectStore.busy}
          onclick={() => void projectStore.newProject()}
        >
          {m.new_project()}
        </button>
        <button
          type="button"
          class="rounded border border-border px-3 py-1.5 text-sm text-text hover:bg-surface-raised disabled:opacity-50"
          disabled={projectStore.busy}
          onclick={() => void projectStore.open()}
        >
          {m.open_project()}
        </button>
      </div>
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
        style:transform={matrix({ ...shown, scale: k })}
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
    {#if dummies.length > 0}
      <svg class="pointer-events-none absolute inset-0 h-full w-full" aria-hidden="true">
        <g transform={matrix(shown)}>
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
    <BalloonLayer view={shown} />
    <!-- The drawing surface on top of all layers: drag to pan, wheel to zoom, keys from the
         shortcut map. Focusable so keyboard users reach it (NFR-UX-01); "application" is the
         ARIA role for such a surface, but Svelte does not count it as interactive. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      bind:this={element}
      class="surface absolute inset-0 touch-none select-none"
      style:cursor
      role="application"
      aria-label={m.viewport_label()}
      tabindex="0"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerCancel}
      ondblclick={(e) => {
        e.preventDefault();
      }}
    ></div>
    <BalloonEditor view={shown} onDone={focusDrawing} />
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
