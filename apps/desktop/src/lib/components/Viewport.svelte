<script lang="ts">
  /**
   * The drawing viewport (FR-DOC-04, T1.6): a `SheetCanvas` with the sheet of the open drawing,
   * the balloon layer, the sheet overlay (zones, views, box select), the value editor and the
   * proposal card, and the pointer gestures of `gestures.svelte.ts`. Its keys are the `drawing`
   * shortcut scope while the drawing is shown (MF F8).
   */
  import { untrack } from "svelte";
  import BalloonEditor from "$lib/components/BalloonEditor.svelte";
  import BalloonLayer from "$lib/components/BalloonLayer.svelte";
  import ProposalCard from "$lib/components/ProposalCard.svelte";
  import SheetCanvas, { type TileSource } from "$lib/components/SheetCanvas.svelte";
  import SheetOverlay from "$lib/components/SheetOverlay.svelte";
  import PlacingBar from "$lib/shell/PlacingBar.svelte";
  import { m } from "$lib/i18n";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { boxSelect } from "$lib/stores/box-select.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { keyboard } from "$lib/stores/keyboard.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonGestures, type PointerInput } from "$lib/viewport/gestures.svelte";
  import { revealPlan, viewAfterSwitch } from "$lib/viewport/reveal";
  import { centerOn, visibleSheetRect, type Point } from "$lib/viewport/view-math";

  /** The canvas element (`main`): keys typed in it belong to the drawing. */
  let canvas = $state<HTMLElement | null>(null);
  /** The drawing surface on top of all layers, which takes pointer input and focus. */
  let element = $state<HTMLElement | null>(null);

  const sheetIndex = $derived(documentStore.sheet);
  const source = $derived.by((): TileSource | null => {
    const doc = documentStore.current?.doc;
    const size = documentStore.sheetSize;
    return doc === undefined || size === null ? null : { doc, page: sheetIndex, size };
  });
  const doc = $derived(source?.doc ?? null);

  // The drawing's shortcut scope (MF F8): active while the drawing is shown.
  $effect(() => {
    return keyboard.register({
      scope: "drawing",
      element: () => canvas,
      shown: () => view.current === "drawing",
    });
  });

  /**
   * A balloon to show once the sheet switch of a reveal has happened: its sheet index, center
   * in sheet space and the zoom before the switch.
   */
  let pendingReveal: { sheet: number; center: Point; scale: number } | null = null;

  /** A sheet switched to for a reveal shows the balloon instead of the whole sheet. */
  function onFirstShow(shown: TileSource): boolean {
    const reveal = pendingReveal;
    pendingReveal = null;
    if (reveal === null || reveal.sheet !== shown.page || viewport.sheet === null) {
      return false;
    }
    viewport.set(
      viewAfterSwitch(
        reveal.center,
        reveal.scale,
        viewport.sheet,
        viewport.size,
        viewport.view.rotation,
      ),
    );
    return true;
  }

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
        // The canvas calls `onFirstShow` once the new sheet is set up.
        pendingReveal = { sheet: plan.sheet, center: plan.center, scale: viewport.view.scale };
        documentStore.setSheet(plan.sheet);
      } else if (plan.kind === "center") {
        viewport.set(
          centerOn(plan.center, viewport.view.scale, viewport.size, viewport.view.rotation),
        );
      }
    });
  });

  // Selected characteristics that were deleted (also by undo) leave the selection.
  $effect(() => {
    const exists = projectStore.characteristicById;
    untrack(() => {
      selection.retain((id) => exists.has(id));
    });
  });

  /** Dummy balloons of the performance check, loaded only in development builds (T0.8). */
  const dummyLayer = import.meta.env.DEV ? import("$lib/dev/DummyBalloons.svelte") : null;

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

<SheetCanvas
  store={viewport}
  {source}
  rotation={projectStore.sheets[sheetIndex]?.rotation ?? "deg0"}
  label={m.viewport_label()}
  tag="main"
  {onFirstShow}
  bind:element={canvas}
>
  {#snippet empty()}
    <!-- Without a project the start screen covers the drawing (T3.17); this shows only while a
         drawing is being opened. -->
    {#if projectStore.isOpen}
      <p class="flex h-full items-center justify-center text-sm text-text-muted" role="status">
        {m.viewport_opening()}
      </p>
    {/if}
  {/snippet}
  {#snippet overlay(shown)}
    {#if dummyLayer !== null && source !== null}
      {#await dummyLayer then { default: DummyBalloons }}
        <DummyBalloons view={shown} sheet={source.size} {sheetIndex} />
      {/await}
    {/if}
    <BalloonLayer view={shown} />
    <SheetOverlay view={shown} />
    <!-- The drawing surface on top of all layers: drag to pan, wheel to zoom, keys from the
         shortcut map (or kept for the value editor right after a placing click). Focusable so
         keyboard users reach it (NFR-UX-01); "application" is the ARIA role for such a surface,
         but Svelte does not count it as interactive. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
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
      oncontextmenu={(e) => {
        // Right click stops placing (M3 decision 19); there is no context menu yet.
        e.preventDefault();
        if (balloonTools.tool === "place") {
          balloonGestures.cancel();
          balloonTools.setTool("select");
        }
      }}
      onkeydown={(e) => {
        // Keys typed right after a placing click belong to the value editor (FR-BAL-02).
        if (balloonTools.typeAhead(e)) {
          e.preventDefault();
        }
      }}
    ></div>
    <BalloonEditor view={shown} onDone={focusDrawing} />
    <ProposalCard
      store={boxSelect}
      view={shown}
      width={viewport.size.width}
      height={viewport.size.height}
      onDone={focusDrawing}
      takeTyped={() => balloonTools.takeTyped("card")}
    />
    <PlacingBar />
  {/snippet}
</SheetCanvas>

<style>
  .surface:focus-visible {
    outline-offset: -2px;
  }
</style>
