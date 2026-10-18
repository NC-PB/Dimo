<script lang="ts">
  /**
   * "Adjust image" (R.20, R01 decisions 8 to 11): straighten, correct the corners, trim and
   * put a photo or scan on its paper, with an optional clean up. The preview is drawn by Rust
   * for the stage of the active tool: the turned page for the corner handles, the corrected
   * page for the trim rectangle, otherwise the sheet as it will be. Apply sends the settings;
   * Rust sets the sheet and moves the balloons (one undo step), or asks first when balloons
   * would leave the sheet.
   */
  import { m } from "$lib/i18n";
  import type { Orientation, PaperSize, SheetCorners } from "$lib/ipc/bindings";
  import {
    MAX_ANGLE,
    PAPER_SIZES,
    SLIDER_ANGLE,
    decimalText,
    decimalValue,
    imagePoint,
    mapPoint,
    pageSize,
    pixelPoint,
    standardPaper,
    trimOf,
    trimRect,
    type TrimRect,
  } from "$lib/image-edit";
  import {
    imageEditor as defaultEditor,
    type ImageEditorStore,
    type ImageTool,
  } from "$lib/stores/image-editor.svelte";
  import { ViewportStore } from "$lib/stores/viewport.svelte";
  import { buttonClass, fieldClass } from "$lib/ui/fields";
  import {
    screenToSheet,
    sheetToScreen,
    type Point,
    type ViewTransform,
  } from "$lib/viewport/view-math";
  import AppDialog from "./AppDialog.svelte";
  import SheetCanvas, { type TileSource } from "./SheetCanvas.svelte";

  interface Props {
    editor?: ImageEditorStore;
  }

  let { editor = defaultEditor }: Props = $props();

  const canvasStore = new ViewportStore();

  // A project closed or replaced while the editor is open ends the edit.
  $effect(() => {
    if (editor.open && !editor.available) {
      editor.close();
    }
  });

  const source = $derived.by((): TileSource | null => {
    const preview = editor.preview;
    const sheet = preview?.doc.sheets[editor.page];
    if (preview === null || sheet === undefined) {
      return null;
    }
    return {
      doc: preview.doc.doc,
      page: editor.page,
      size: { width: sheet.width ?? 0, height: sheet.height ?? 0 },
    };
  });

  // Fit again only when the preview changes its size, not on every slider step.
  let fittedSize = "";
  function onFirstShow(shown: TileSource): boolean {
    const key = `${editor.stage}/${String(Math.round(shown.size.width))}x${String(Math.round(shown.size.height))}`;
    const keep = key === fittedSize;
    fittedSize = key;
    return keep;
  }

  const TOOLS: readonly { id: ImageTool; label: () => string }[] = [
    { id: "rotate", label: () => m.image_tool_rotate() },
    { id: "corners", label: () => m.image_tool_corners() },
    { id: "trim", label: () => m.image_tool_trim() },
    { id: "paper", label: () => m.image_tool_paper() },
  ];

  const ORIENTATIONS: readonly Orientation[] = ["portrait", "landscape"];

  const CORNERS: readonly (keyof SheetCorners)[] = [
    "top_left",
    "top_right",
    "bottom_right",
    "bottom_left",
  ];

  function paperLabel(size: PaperSize): string {
    switch (size) {
      case "letter":
        return m.paper_letter();
      case "tabloid":
        return m.paper_tabloid();
      default:
        return size.toUpperCase();
    }
  }

  // Paper form state: the select shows the format, custom sizes are typed in mm.
  const CUSTOM = "custom";
  const FROM_PHOTO = "photo";
  const paperChoice = $derived.by(() => {
    const paper = editor.edit.paper;
    return paper === null ? FROM_PHOTO : paper.format === "custom" ? CUSTOM : paper.size;
  });
  const orientation = $derived.by((): Orientation => {
    const paper = editor.edit.paper;
    if (paper?.format === "standard") {
      return paper.orientation;
    }
    return source !== null && source.size.width > source.size.height ? "landscape" : "portrait";
  });

  function choosePaper(value: string): void {
    if (value === FROM_PHOTO) {
      editor.setPaper(null);
    } else if (value === CUSTOM) {
      const size = source?.size;
      const mm = (units: number | undefined) => decimalText(((units ?? 595) * 25.4) / 72, 1);
      editor.setPaper({
        format: "custom",
        width_mm: mm(size?.width),
        height_mm: mm(size?.height),
      });
    } else {
      const size = PAPER_SIZES.find((s) => s === value);
      if (size !== undefined) {
        editor.setPaper(standardPaper(size, source?.size ?? null));
      }
    }
  }

  function chooseOrientation(next: Orientation): void {
    const paper = editor.edit.paper;
    if (paper?.format === "standard") {
      editor.setPaper({ ...paper, orientation: next });
    }
  }

  function customSide(side: "width_mm" | "height_mm", text: string): void {
    const paper = editor.edit.paper;
    const value = decimalValue(text.replace(",", "."));
    if (paper?.format === "custom" && value > 0) {
      editor.setPaper({ ...paper, [side]: decimalText(value, 1) });
    }
  }

  // Dragging handles: a corner, a trim corner, or the whole trim rectangle.
  type Drag =
    | { kind: "corner"; name: keyof SheetCorners }
    | { kind: "trim"; edge: "tl" | "tr" | "br" | "bl" }
    | { kind: "move"; from: Point; start: TrimRect };
  let drag = $state<Drag | null>(null);
  let overlayElement = $state<SVGSVGElement | null>(null);

  function sheetPoint(view: ViewTransform, event: PointerEvent): Point {
    const rect = overlayElement?.getBoundingClientRect();
    return screenToSheet(view, {
      x: event.clientX - (rect?.left ?? 0),
      y: event.clientY - (rect?.top ?? 0),
    });
  }

  const page = $derived(editor.cornerStart === null ? null : pageSize(editor.cornerStart.page));

  function onMove(view: ViewTransform, event: PointerEvent): void {
    const current = drag;
    const preview = editor.preview;
    if (current === null || preview === null || source === null) {
      return;
    }
    const at = sheetPoint(view, event);
    if (current.kind === "corner") {
      const pixel = mapPoint(preview.from_view, at);
      if (pixel !== null && page !== null) {
        editor.moveCorner(current.name, imagePoint(pixel, page));
      }
      return;
    }
    const rect = trimRect(editor.edit.trim, source.size);
    let next: TrimRect;
    if (current.kind === "move") {
      const w = current.start.right - current.start.left;
      const h = current.start.bottom - current.start.top;
      const left = Math.min(
        source.size.width - w,
        Math.max(0, current.start.left + at.x - current.from.x),
      );
      const top = Math.min(
        source.size.height - h,
        Math.max(0, current.start.top + at.y - current.from.y),
      );
      next = { left, top, right: left + w, bottom: top + h };
    } else {
      next = {
        left: current.edge === "tl" || current.edge === "bl" ? at.x : rect.left,
        right: current.edge === "tr" || current.edge === "br" ? at.x : rect.right,
        top: current.edge === "tl" || current.edge === "tr" ? at.y : rect.top,
        bottom: current.edge === "bl" || current.edge === "br" ? at.y : rect.bottom,
      };
    }
    editor.setTrim(trimOf(next, source.size));
  }

  function startDrag(event: PointerEvent, next: Drag): void {
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    drag = next;
  }

  function cornerHandles(view: ViewTransform): { name: keyof SheetCorners; at: Point }[] {
    const preview = editor.preview;
    const corners = editor.edit.corners;
    if (preview === null || corners === null) {
      return [];
    }
    return CORNERS.flatMap((name) => {
      const at = mapPoint(preview.to_view, pixelPoint(corners[name]));
      return at === null ? [] : [{ name, at: sheetToScreen(view, at) }];
    });
  }

  function trimHandles(view: ViewTransform): {
    box: Point[];
    corners: { edge: "tl" | "tr" | "br" | "bl"; at: Point }[];
  } {
    if (source === null) {
      return { box: [], corners: [] };
    }
    const r = trimRect(editor.edit.trim, source.size);
    const pts = {
      tl: { x: r.left, y: r.top },
      tr: { x: r.right, y: r.top },
      br: { x: r.right, y: r.bottom },
      bl: { x: r.left, y: r.bottom },
    };
    const edges = ["tl", "tr", "br", "bl"] as const;
    return {
      box: edges.map((e) => sheetToScreen(view, pts[e])),
      corners: edges.map((edge) => ({ edge, at: sheetToScreen(view, pts[edge]) })),
    };
  }

  const points = (list: Point[]) => list.map((p) => `${String(p.x)},${String(p.y)}`).join(" ");

  function onAngleInput(text: string): void {
    const value = Number(text.replace(",", "."));
    if (Number.isFinite(value)) {
      editor.setAngle(value);
    }
  }

  const tab = (active: boolean) =>
    `rounded-md px-2.5 py-1.5 text-[13px] font-medium ${active ? "bg-accent-soft text-text border border-accent" : "text-text-muted hover:bg-hover border border-transparent"}`;
  const inputClass = fieldClass("text-[13px]");
  const angleText = $derived(decimalText(editor.angle, 2));
</script>

<AppDialog
  open={editor.open}
  onClose={() => {
    editor.close();
  }}
  title={m.image_editor_title()}
  description={m.image_editor_lead()}
  closeLabel={m.image_cancel()}
  width="editor"
  testid="image-editor"
>
  <div class="flex min-h-0 flex-1 gap-4">
    <div class="flex w-72 shrink-0 flex-col gap-3 overflow-y-auto text-sm">
      <div class="flex flex-wrap gap-1" role="tablist" aria-label={m.image_editor_tools()}>
        {#each TOOLS as tool (tool.id)}
          <button
            type="button"
            role="tab"
            aria-selected={editor.tool === tool.id}
            class={tab(editor.tool === tool.id)}
            data-tool={tool.id}
            onclick={() => {
              editor.setTool(tool.id);
            }}>{tool.label()}</button
          >
        {/each}
      </div>

      {#if editor.tool === "rotate"}
        <div class="flex gap-2">
          <button
            type="button"
            class={buttonClass.secondary}
            onclick={() => {
              editor.turn(-1);
            }}>{m.rotate_left()}</button
          >
          <button
            type="button"
            class={buttonClass.secondary}
            onclick={() => {
              editor.turn(1);
            }}>{m.rotate_right()}</button
          >
        </div>
        <label class="flex flex-col gap-1 text-xs text-text-muted">
          {m.image_straighten()}
          <input
            type="range"
            min={-SLIDER_ANGLE}
            max={SLIDER_ANGLE}
            step="0.1"
            disabled={editor.edit.corners !== null}
            value={Math.max(-SLIDER_ANGLE, Math.min(SLIDER_ANGLE, editor.angle))}
            oninput={(e) => {
              editor.setAngle(Number(e.currentTarget.value));
            }}
            data-field="angle-slider"
          />
        </label>
        <label class="flex items-center gap-2 text-xs text-text-muted">
          {m.image_angle()}
          <input
            class="{inputClass} w-20"
            inputmode="decimal"
            disabled={editor.edit.corners !== null}
            value={angleText}
            onchange={(e) => {
              onAngleInput(e.currentTarget.value);
            }}
            data-field="angle"
          />
          °
        </label>
        <p class="text-xs leading-snug text-text-muted">
          {editor.edit.corners !== null
            ? m.image_angle_with_corners()
            : m.image_angle_hint({ max: String(MAX_ANGLE) })}
        </p>
      {:else if editor.tool === "corners"}
        <label class="flex items-center gap-2">
          <input
            type="checkbox"
            class="accent-accent"
            checked={editor.edit.corners !== null}
            onchange={(e) => {
              void editor.setCornersOn(e.currentTarget.checked);
            }}
            data-field="corners"
          />
          {m.image_corners_on()}
        </label>
        <p class="text-xs leading-snug text-text-muted">{m.image_corners_hint()}</p>
        {#if editor.cornerStart !== null && editor.cornerStart.proposed === null && editor.edit.corners !== null}
          <p class="text-xs leading-snug text-text-muted" role="status">
            {m.image_corners_not_found()}
          </p>
        {/if}
      {:else if editor.tool === "trim"}
        <p class="text-xs leading-snug text-text-muted">{m.image_trim_hint()}</p>
        <button
          type="button"
          class={buttonClass.secondary}
          disabled={editor.edit.trim === null}
          onclick={() => {
            editor.setTrim(null);
          }}>{m.image_trim_none()}</button
        >
      {:else}
        <label class="flex flex-col gap-1 text-xs text-text-muted">
          {m.image_paper()}
          <select
            class={inputClass}
            value={paperChoice}
            onchange={(e) => {
              choosePaper(e.currentTarget.value);
            }}
            data-field="paper"
          >
            <option value={FROM_PHOTO}>{m.image_paper_photo()}</option>
            {#each PAPER_SIZES as size (size)}
              <option value={size}>{paperLabel(size)}</option>
            {/each}
            <option value={CUSTOM}>{m.image_paper_custom()}</option>
          </select>
        </label>
        {#if editor.edit.paper?.format === "standard"}
          <div class="flex gap-3 text-[13px]" role="radiogroup" aria-label={m.image_orientation()}>
            {#each ORIENTATIONS as option (option)}
              <label class="flex items-center gap-1.5">
                <input
                  type="radio"
                  class="accent-accent"
                  name="image-orientation"
                  checked={orientation === option}
                  onchange={() => {
                    chooseOrientation(option);
                  }}
                />
                {option === "portrait" ? m.image_portrait() : m.image_landscape()}
              </label>
            {/each}
          </div>
        {:else if editor.edit.paper?.format === "custom"}
          <div class="flex gap-2">
            <label class="flex flex-col gap-0.5 text-xs text-text-muted">
              {m.image_paper_width()}
              <input
                class="{inputClass} w-24"
                inputmode="decimal"
                value={editor.edit.paper.width_mm}
                onchange={(e) => {
                  customSide("width_mm", e.currentTarget.value);
                }}
              />
            </label>
            <label class="flex flex-col gap-0.5 text-xs text-text-muted">
              {m.image_paper_height()}
              <input
                class="{inputClass} w-24"
                inputmode="decimal"
                value={editor.edit.paper.height_mm}
                onchange={(e) => {
                  customSide("height_mm", e.currentTarget.value);
                }}
              />
            </label>
          </div>
        {/if}
        <p class="text-xs leading-snug text-text-muted">{m.image_paper_hint()}</p>
      {/if}

      <label class="mt-auto flex items-center gap-2 border-t border-border pt-3">
        <input
          type="checkbox"
          class="accent-accent"
          checked={editor.edit.clean_up}
          onchange={(e) => {
            editor.setCleanUp(e.currentTarget.checked);
          }}
          data-field="clean-up"
        />
        {m.image_clean_up()}
      </label>
      <p class="text-xs leading-snug text-text-muted">{m.image_clean_up_hint()}</p>
      {#if editor.error !== null}
        <p class="text-xs text-danger" role="alert">{editor.error}</p>
      {/if}
    </div>

    <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden rounded-lg border border-border">
      <SheetCanvas store={canvasStore} {source} label={m.image_preview()} {onFirstShow}>
        {#snippet empty()}
          <p class="flex h-full items-center justify-center text-sm text-text-muted" role="status">
            {m.image_preview_loading()}
          </p>
        {/snippet}
        {#snippet overlay(view)}
          <svg
            bind:this={overlayElement}
            class="absolute inset-0 h-full w-full touch-none"
            role="presentation"
            onpointermove={(e) => {
              onMove(view, e);
            }}
            onpointerup={() => {
              drag = null;
            }}
            onpointercancel={() => {
              drag = null;
            }}
          >
            {#if editor.tool === "corners"}
              {@const handles = cornerHandles(view)}
              {#if handles.length === 4}
                <polygon
                  points={points(handles.map((h) => h.at))}
                  class="fill-accent/10 stroke-accent"
                  stroke-width="2"
                />
              {/if}
              {#each handles as handle (handle.name)}
                <circle
                  cx={handle.at.x}
                  cy={handle.at.y}
                  r="9"
                  class="cursor-move fill-surface stroke-accent"
                  stroke-width="3"
                  data-corner={handle.name}
                  role="presentation"
                  onpointerdown={(e) => {
                    startDrag(e, { kind: "corner", name: handle.name });
                  }}
                />
              {/each}
            {:else if editor.tool === "trim" && source !== null}
              {@const handles = trimHandles(view)}
              <polygon
                points={points(handles.box)}
                class="cursor-move fill-accent/10 stroke-accent"
                stroke-width="2"
                role="presentation"
                onpointerdown={(e) => {
                  if (source !== null) {
                    startDrag(e, {
                      kind: "move",
                      from: sheetPoint(view, e),
                      start: trimRect(editor.edit.trim, source.size),
                    });
                  }
                }}
              />
              {#each handles.corners as handle (handle.edge)}
                <rect
                  x={handle.at.x - 7}
                  y={handle.at.y - 7}
                  width="14"
                  height="14"
                  class="cursor-nwse-resize fill-surface stroke-accent"
                  stroke-width="3"
                  data-trim={handle.edge}
                  role="presentation"
                  onpointerdown={(e) => {
                    startDrag(e, { kind: "trim", edge: handle.edge });
                  }}
                />
              {/each}
            {/if}
          </svg>
        {/snippet}
      </SheetCanvas>
    </div>
  </div>

  {#snippet actions()}
    <button
      type="button"
      class={buttonClass.secondary}
      onclick={() => {
        editor.reset();
      }}
      data-action="reset">{m.image_reset()}</button
    >
    <button
      type="button"
      class={buttonClass.primary}
      disabled={editor.busy}
      onclick={() => {
        void editor.apply();
      }}
      data-action="apply">{m.image_apply()}</button
    >
  {/snippet}
</AppDialog>

<AppDialog
  open={editor.offSheet !== null}
  onClose={() => {
    void editor.answerOffSheet(false);
  }}
  title={m.image_off_sheet_title()}
  description={m.image_off_sheet_text({ count: String(editor.offSheet ?? 0) })}
  closeLabel={m.image_cancel()}
  alert
  testid="image-off-sheet"
>
  {#snippet actions()}
    <button
      type="button"
      class={buttonClass.primary}
      onclick={() => {
        void editor.answerOffSheet(true);
      }}
      data-action="apply-anyway">{m.image_off_sheet_apply()}</button
    >
  {/snippet}
</AppDialog>
