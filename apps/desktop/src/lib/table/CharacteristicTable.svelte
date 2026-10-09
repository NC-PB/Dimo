<script lang="ts">
  import {
    columnSizingFeature,
    createColumnHelper,
    createTable,
    tableFeatures,
  } from "@tanstack/svelte-table";
  import { untrack } from "svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { m } from "$lib/i18n";
  import type { CharId, Characteristic, Command } from "$lib/ipc/bindings";
  import { isMacPlatform, matchShortcut } from "$lib/shortcuts";
  import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection, type SelectionStore } from "$lib/stores/selection.svelte";
  import CellEditor, { type EditMove } from "./CellEditor.svelte";
  import ChoiceEditor from "./ChoiceEditor.svelte";
  import {
    COLUMNS,
    REQUIREMENT_COLUMN,
    choiceOptions,
    displayText,
    fieldValue,
    isEditable,
    rawText,
  } from "./columns";
  import { lockExplanation, refusalDetail } from "./refusal";
  import { moveStep, moveToGap } from "./reorder";
  import { RowVirtualizer } from "./virtual.svelte";

  interface Props {
    /** The project shown; the app's project store unless a test passes its own. */
    project?: ProjectStore;
    /** The shared selection; the app's selection unless a test passes its own. */
    selected?: SelectionStore;
  }

  let { project = projectStore, selected = selection }: Props = $props();

  /** Row and header height in CSS px. One fixed height keeps virtualization exact. */
  const ROW_HEIGHT = 28;
  const HEADER_HEIGHT = 28;
  /** Rows rendered above and below the visible ones. */
  const OVERSCAN = 8;
  /** Distance from the top or bottom edge where a drag scrolls the list. */
  const AUTO_SCROLL_EDGE = 24;

  // TanStack Table holds the column model and the row model (placement order, D-21). There is
  // no sorting or filtering: the order of the table is the numbering order.
  const features = tableFeatures({ columnSizingFeature });
  const helper = createColumnHelper<typeof features, Characteristic>();
  const columns = COLUMNS.map((spec) =>
    helper.accessor((c: Characteristic): unknown => rawText(c, spec.id), {
      id: spec.id,
      size: spec.size,
    }),
  );
  const table = createTable({
    features,
    columns,
    get data() {
      return project.characteristics as Characteristic[];
    },
    getRowId: (c: Characteristic) => c.id,
  });

  const rows = $derived(table.getRowModel().rows);
  const order = $derived(rows.map((r) => r.id));
  const indexById = $derived(new Map(order.map((id, i) => [id, i])));
  const totalWidth = $derived(table.getTotalSize());
  const columnLeft = $derived(
    COLUMNS.map((_, i) => COLUMNS.slice(0, i).reduce((sum, c) => sum + c.size, 0)),
  );
  const lock = $derived(project.project?.numbering.lock ?? null);

  const virtualizer = new RowVirtualizer({
    rowHeight: ROW_HEIGHT,
    overscan: OVERSCAN,
    headerHeight: HEADER_HEIGHT,
  });

  let grid = $state<HTMLElement | null>(null);
  let body = $state<HTMLElement | null>(null);

  $effect(() => {
    const element = grid;
    if (element === null) {
      return;
    }
    if (DEV_TOOLS_ENABLED) {
      devTools.tableScroller = element;
    }
    const detach = virtualizer.attach(element);
    return () => {
      detach();
      if (DEV_TOOLS_ENABLED && devTools.tableScroller === element) {
        devTools.tableScroller = null;
      }
    };
  });

  $effect.pre(() => {
    virtualizer.setCount(rows.length);
  });

  /** The cell keyboard input goes to, by characteristic ID and column index. */
  let active = $state<{ id: CharId; col: number } | null>(null);
  /** Start of a Shift range selection. */
  let anchor: CharId | null = null;

  /** The cell being edited. `error`: Rust refused the last attempt. */
  let editing = $state<{
    id: CharId;
    col: number;
    text: string;
    selectAll: boolean;
    pending: boolean;
    error: boolean;
  } | null>(null);

  /** Line under the header: why the last change was refused or why moving is not possible. */
  let notice = $state<string | null>(null);

  /** A running row drag. `gap`: where the rows would go (0 = before the first row). */
  let drag = $state<{ pointer: number; moving: ReadonlySet<CharId>; gap: number | null } | null>(
    null,
  );
  let dragY = 0;
  let autoScroll = 0;

  const activeRow = $derived(active === null ? -1 : (indexById.get(active.id) ?? -1));

  function cellId(id: CharId, col: number): string {
    return `ct-${id}-${COLUMNS[col]?.id ?? ""}`;
  }

  const activeDescendant = $derived.by(() => {
    if (active === null || activeRow < 0) {
      return undefined;
    }
    const shown = virtualizer.items.some((item) => item.index === activeRow);
    return shown ? cellId(active.id, active.col) : undefined;
  });

  function focusGrid() {
    grid?.focus({ preventScroll: true });
  }

  /** Scrolls row `index` and column `col` into view. */
  function reveal(index: number, col: number) {
    virtualizer.scrollToIndex(index);
    const element = grid;
    const left = columnLeft[col];
    const width = COLUMNS[col]?.size ?? 0;
    if (element === null || left === undefined) {
      return;
    }
    if (left < element.scrollLeft) {
      element.scrollLeft = left;
    } else if (left + width > element.scrollLeft + element.clientWidth) {
      element.scrollLeft = left + width - element.clientWidth;
    }
  }

  function setActive(index: number, col: number) {
    const id = order[index];
    if (id === undefined) {
      return;
    }
    active = { id, col: Math.max(0, Math.min(COLUMNS.length - 1, col)) };
    reveal(index, active.col);
  }

  /** Selects the rows from the anchor to `index`. */
  function selectRange(index: number) {
    const from = anchor === null ? index : (indexById.get(anchor) ?? index);
    const [a, b] = from <= index ? [from, index] : [index, from];
    selected.select(order.slice(a, b + 1), "replace");
  }

  /** Moves the active row to `index`, selecting it or extending the selection. */
  function goToRow(index: number, extend: boolean) {
    const target = Math.max(0, Math.min(order.length - 1, index));
    const id = order[target];
    if (id === undefined) {
      return;
    }
    setActive(target, active?.col ?? 0);
    if (extend) {
      selectRange(target);
    } else {
      anchor = id;
      selected.select([id], "replace");
    }
  }

  /**
   * Sends a command. Returns `null` on success, else Rust's reason, which the table shows itself
   * (so the window wide error line does not repeat it).
   */
  async function send(command: Command): Promise<string | null> {
    const patch = await project.execute(command);
    if (patch !== undefined) {
      return null;
    }
    const error = project.error;
    project.dismissError();
    return error === null ? "" : refusalDetail(error);
  }

  function startEdit(index: number, col: number, typed: string | null = null) {
    const c = rows[index]?.original;
    const column = COLUMNS[col];
    if (c === undefined || column === undefined || !isEditable(column)) {
      return;
    }
    setActive(index, col);
    if (column.editor === "check") {
      void toggleInspect(c);
      return;
    }
    if (column.editor === "choice" && typed !== null) {
      return;
    }
    notice = null;
    editing = {
      id: c.id,
      col,
      text: typed ?? rawText(c, column.id),
      selectAll: typed === null,
      pending: false,
      error: false,
    };
  }

  function cancelEdit() {
    editing = null;
    notice = null;
    focusGrid();
  }

  /** Confirms the edit; on success applies `move`. Rust parses and validates the value. */
  async function commitEdit(move: EditMove) {
    const edit = editing;
    if (edit === null || edit.pending) {
      return;
    }
    const c = project.characteristicById.get(edit.id);
    const column = COLUMNS[edit.col];
    const value = column === undefined ? null : fieldValue(column.id, edit.text);
    if (c === undefined || column === undefined || edit.text === rawText(c, column.id) || !value) {
      editing = null;
      afterEdit(edit.id, edit.col, move);
      return;
    }
    editing = { ...edit, pending: true };
    const refused = await send({ type: "update_fields", ids: [edit.id], values: [value] });
    if (editing?.id !== edit.id || editing.col !== edit.col) {
      return;
    }
    if (refused === null) {
      editing = null;
      notice = null;
      afterEdit(edit.id, edit.col, move);
    } else {
      editing = { ...edit, pending: false, error: true };
      notice = m.table_value_refused({ message: refused });
    }
  }

  function afterEdit(id: CharId, col: number, move: EditMove) {
    if (move === "blur") {
      return;
    }
    focusGrid();
    const index = indexById.get(id);
    if (index === undefined) {
      return;
    }
    switch (move) {
      case "up":
        goToRow(index - 1, false);
        break;
      case "down":
        goToRow(index + 1, false);
        break;
      case "next":
        setActive(index, nextEditable(col, 1));
        break;
      case "previous":
        setActive(index, nextEditable(col, -1));
        break;
      case "none":
        break;
    }
  }

  function nextEditable(col: number, step: 1 | -1): number {
    for (let c = col + step; c >= 0 && c < COLUMNS.length; c += step) {
      const column = COLUMNS[c];
      if (column && isEditable(column)) {
        return c;
      }
    }
    return col;
  }

  async function pickChoice(value: string) {
    const edit = editing;
    const column = edit === null ? undefined : COLUMNS[edit.col];
    const field = column === undefined ? null : fieldValue(column.id, value);
    if (edit === null || field === null) {
      return;
    }
    const refused = await send({ type: "update_fields", ids: [edit.id], values: [field] });
    notice = refused === null ? null : m.table_value_refused({ message: refused });
  }

  function closeChoice() {
    editing = null;
    // Bits UI gives the focus back to the trigger, which is gone by now.
    requestAnimationFrame(focusGrid);
  }

  async function toggleInspect(c: Characteristic) {
    const refused = await send({
      type: "update_fields",
      ids: [c.id],
      values: [{ field: "inspect", value: !c.inspect }],
    });
    notice = refused === null ? null : m.table_value_refused({ message: refused });
  }

  /** IDs to move: the selection, or the active row if it is not selected. */
  function movingIds(): ReadonlySet<CharId> {
    if (active !== null && !selected.has(active.id)) {
      return new Set([active.id]);
    }
    return selected.ids;
  }

  async function sendMove(command: Command | null) {
    if (command === null) {
      return;
    }
    if (lock !== null) {
      notice = lockExplanation(lock);
      return;
    }
    const refused = await send(command);
    notice = refused === null ? null : m.table_move_refused({ message: refused });
    if (refused === null && active !== null) {
      const index = indexById.get(active.id);
      if (index !== undefined) {
        reveal(index, active.col);
      }
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.target !== grid || event.defaultPrevented || event.isComposing) {
      return; // Keys inside an editor or picker belong to it.
    }
    const shortcut = matchShortcut(event, isMacPlatform(), "table");
    const index = activeRow;
    const col = active?.col ?? 0;
    if (shortcut === null) {
      // Typing a character starts editing a text or number cell with that character.
      const column = COLUMNS[col];
      const printable = event.key.length === 1 && !event.ctrlKey && !event.metaKey;
      if (printable && index >= 0 && column && column.editor !== "check") {
        if (column.editor !== "choice" && isEditable(column)) {
          event.preventDefault();
          startEdit(index, col, event.key);
        }
      }
      return;
    }
    event.preventDefault();
    if (order.length === 0) {
      return;
    }
    const current = Math.max(0, index);
    switch (shortcut.action) {
      case "table_up":
        goToRow(index < 0 ? 0 : index - 1, false);
        break;
      case "table_down":
        goToRow(index + 1, false);
        break;
      case "table_extend_up":
        goToRow(current - 1, true);
        break;
      case "table_extend_down":
        goToRow(current + 1, true);
        break;
      case "table_left":
        setActive(current, col - 1);
        break;
      case "table_right":
        setActive(current, col + 1);
        break;
      case "table_first_column":
        setActive(current, 0);
        break;
      case "table_last_column":
        setActive(current, COLUMNS.length - 1);
        break;
      case "table_first_row":
        goToRow(0, false);
        break;
      case "table_last_row":
        goToRow(order.length - 1, false);
        break;
      case "table_select_all":
        selected.select(order, "replace");
        break;
      case "table_toggle": {
        const id = order[current];
        if (COLUMNS[col]?.editor === "check") {
          startEdit(current, col);
        } else if (id !== undefined) {
          setActive(current, col);
          anchor = id;
          selected.select([id], "toggle");
        }
        break;
      }
      case "table_edit":
        startEdit(current, col);
        break;
      case "table_cancel":
        notice = null;
        break;
      case "table_move_up":
        void sendMove(moveStep(order, movingIds(), -1));
        break;
      case "table_move_down":
        void sendMove(moveStep(order, movingIds(), 1));
        break;
      default:
        break;
    }
  }

  /** Row and column of the cell under a pointer event, from the cell's data attributes. */
  function cellAt(target: EventTarget | null): { index: number; col: number } | null {
    const cell = target instanceof Element ? target.closest<HTMLElement>("[data-col]") : null;
    const row = cell?.closest<HTMLElement>("[data-row]");
    if (!cell || !row) {
      return null;
    }
    return { index: Number(row.dataset.row), col: Number(cell.dataset.col) };
  }

  function onCellPointerDown(event: PointerEvent) {
    if (event.button !== 0) {
      return;
    }
    const target = event.target instanceof Element ? event.target : null;
    const hit = cellAt(target);
    if (hit === null) {
      return;
    }
    const id = order[hit.index];
    if (id === undefined) {
      return;
    }
    if (target?.closest("[data-handle]")) {
      startDrag(event, id);
      return;
    }
    if (target?.closest("input, button")) {
      return; // The editor, the picker and the check box handle their own pointer.
    }
    setActive(hit.index, hit.col);
    const toggle = isMacPlatform() ? event.metaKey : event.ctrlKey;
    if (event.shiftKey) {
      selectRange(hit.index);
    } else if (toggle) {
      anchor = id;
      selected.select([id], "toggle");
    } else {
      anchor = id;
      selected.select([id], "replace");
    }
  }

  function onCellDoubleClick(event: MouseEvent) {
    const target = event.target instanceof Element ? event.target : null;
    const hit = cellAt(target);
    if (hit === null || target?.closest("input, button, [data-handle]")) {
      return;
    }
    if (COLUMNS[hit.col]?.editor !== "check") {
      startEdit(hit.index, hit.col);
    }
  }

  // Row drag (FR-BAL-06): drag the number handle, drop between rows. One command on release.

  function startDrag(event: PointerEvent, id: CharId) {
    event.preventDefault();
    focusGrid();
    if (lock !== null) {
      notice = lockExplanation(lock);
      return;
    }
    if (!selected.has(id)) {
      anchor = id;
      selected.select([id], "replace");
    }
    const index = indexById.get(id) ?? 0;
    active = { id, col: 0 };
    grid?.setPointerCapture(event.pointerId);
    dragY = event.clientY;
    drag = { pointer: event.pointerId, moving: new Set(selected.ids), gap: index };
    autoScroll = requestAnimationFrame(scrollWhileDragging);
  }

  function gapAt(clientY: number): number {
    const top = body?.getBoundingClientRect().top ?? 0;
    return Math.max(0, Math.min(order.length, Math.round((clientY - top) / ROW_HEIGHT)));
  }

  function scrollWhileDragging() {
    if (drag === null || grid === null) {
      return;
    }
    const rect = grid.getBoundingClientRect();
    if (dragY < rect.top + HEADER_HEIGHT + AUTO_SCROLL_EDGE) {
      grid.scrollTop -= ROW_HEIGHT / 2;
    } else if (dragY > rect.bottom - AUTO_SCROLL_EDGE) {
      grid.scrollTop += ROW_HEIGHT / 2;
    }
    drag = { ...drag, gap: gapAt(dragY) };
    autoScroll = requestAnimationFrame(scrollWhileDragging);
  }

  function onPointerMove(event: PointerEvent) {
    if (drag?.pointer === event.pointerId) {
      dragY = event.clientY;
    }
  }

  function endDrag(event: PointerEvent, drop: boolean) {
    const current = drag;
    if (current?.pointer !== event.pointerId) {
      return;
    }
    cancelAnimationFrame(autoScroll);
    drag = null;
    if (drop) {
      void sendMove(moveToGap(order, current.moving, gapAt(event.clientY)));
    }
  }

  // Selection made elsewhere (the drawing): bring the first selected row into view.
  $effect(() => {
    const ids = selected.ids;
    untrack(() => {
      if (ids.size === 0) {
        return;
      }
      if (active !== null && ids.has(active.id)) {
        if (activeRow >= 0) {
          virtualizer.scrollToIndex(activeRow);
        }
        return;
      }
      const first = order.findIndex((id) => ids.has(id));
      if (first >= 0) {
        anchor = order[first] ?? null;
        setActive(first, active?.col ?? 0);
      }
    });
  });

  // "Edit this characteristic" (for example right after placing a balloon): focus its
  // requirement cell for typing.
  let handledFocus = 0;
  $effect(() => {
    const request = selected.focusRequest;
    untrack(() => {
      if (request === null || request.seq === handledFocus) {
        return;
      }
      handledFocus = request.seq;
      const index = indexById.get(request.id);
      if (index === undefined) {
        return;
      }
      if (!selected.has(request.id)) {
        anchor = request.id;
        selected.select([request.id], "replace");
      }
      startEdit(index, REQUIREMENT_COLUMN);
    });
  });
</script>

<!-- The characteristic table (T1.7): a virtualized ARIA grid. Focus stays on the grid and
     `aria-activedescendant` names the active cell; editors take the focus while they are open. -->
<div class="flex min-h-0 flex-1 flex-col">
  <!-- One status line: why the last change was refused, else the numbering lock. A fixed line,
       so a message never shifts the rows. -->
  <header class="flex h-6 shrink-0 items-center gap-3 px-3 text-xs text-text-muted">
    <h2 class="font-semibold text-text">{m.table_label()}</h2>
    <span class="shrink-0">{m.table_count({ count: String(rows.length) })}</span>
    <span
      class="min-w-0 truncate"
      class:flex-1={notice !== null}
      role="alert"
      title={notice ?? undefined}
    >
      {#if notice !== null}
        <span class="text-danger"><span aria-hidden="true">⚠</span> {notice}</span>
      {/if}
    </span>
    {#if notice === null && lock !== null}
      <span class="min-w-0 flex-1 truncate" title={lockExplanation(lock)}>
        <span aria-hidden="true">🔒</span>
        {lockExplanation(lock)}
      </span>
    {/if}
  </header>
  <div
    bind:this={grid}
    class="grid-scroll relative min-h-0 flex-1 overflow-auto text-sm outline-none"
    class:dragging={drag !== null}
    role="grid"
    tabindex="0"
    aria-label={m.table_label()}
    aria-rowcount={rows.length + 1}
    aria-colcount={COLUMNS.length}
    aria-multiselectable="true"
    aria-activedescendant={activeDescendant}
    onkeydown={onKeyDown}
    onpointerdown={onCellPointerDown}
    onpointermove={onPointerMove}
    onpointerup={(event) => {
      endDrag(event, true);
    }}
    onpointercancel={(event) => {
      endDrag(event, false);
    }}
    ondblclick={onCellDoubleClick}
  >
    <div class="sticky top-0 z-10" role="rowgroup" style:width="{totalWidth}px">
      <div
        class="flex border-b border-border bg-surface-raised text-xs font-semibold text-text-muted"
        role="row"
        aria-rowindex={1}
        style:height="{HEADER_HEIGHT}px"
      >
        {#each table.getFlatHeaders() as header, col (header.id)}
          {@const spec = COLUMNS[col]}
          <div
            class="flex shrink-0 items-center truncate border-r border-border px-2"
            class:justify-end={spec?.numeric}
            role="columnheader"
            aria-colindex={col + 1}
            style:width="{header.getSize()}px"
          >
            {spec?.label()}
          </div>
        {/each}
      </div>
    </div>
    {#if rows.length === 0}
      <p class="px-3 py-4 text-text-muted">{m.table_empty()}</p>
    {/if}
    <div
      bind:this={body}
      class="relative"
      role="rowgroup"
      style:height="{virtualizer.totalSize}px"
      style:width="{totalWidth}px"
    >
      {#each virtualizer.items as item (rows[item.index]?.id ?? item.key)}
        {@const c = rows[item.index]?.original}
        {#if c}
          {@const isSelected = selected.has(c.id)}
          <div
            class="row absolute left-0 flex border-b border-border"
            class:selected={isSelected}
            class:moving={drag?.moving.has(c.id)}
            role="row"
            aria-rowindex={item.index + 2}
            aria-selected={isSelected}
            data-row={item.index}
            style:transform="translateY({virtualizer.rowTop(item)}px)"
            style:height="{ROW_HEIGHT}px"
            style:width="{totalWidth}px"
          >
            {#each COLUMNS as column, col (column.id)}
              {@const isActive = active?.id === c.id && active.col === col}
              {@const isEditing = editing?.id === c.id && editing.col === col}
              <div
                id={cellId(c.id, col)}
                class="cell relative flex shrink-0 items-center border-r border-border"
                class:active={isActive}
                class:justify-end={column.numeric}
                role="gridcell"
                aria-colindex={col + 1}
                aria-readonly={!isEditable(column)}
                data-col={col}
                style:width="{column.size}px"
              >
                {#if isEditing && editing && column.editor === "choice"}
                  <ChoiceEditor
                    value={rawText(c, column.id)}
                    options={choiceOptions(column.id)}
                    label={column.label()}
                    onPick={(value) => void pickChoice(value)}
                    onClose={closeChoice}
                  />
                {:else if isEditing && editing}
                  <CellEditor
                    text={editing.text}
                    pending={editing.pending}
                    invalid={editing.error}
                    numeric={column.numeric ?? false}
                    label={column.label()}
                    selectAll={editing.selectAll}
                    onInput={(text) => {
                      if (editing) {
                        editing = { ...editing, text, error: false };
                      }
                    }}
                    onCommit={(move) => void commitEdit(move)}
                    onCancel={cancelEdit}
                  />
                {:else if column.id === "number"}
                  <span
                    class="handle px-1 text-text-muted"
                    class:locked={lock !== null}
                    data-handle
                    title={lock === null
                      ? m.table_drag_handle({ number: String(c.number) })
                      : lockExplanation(lock)}
                    aria-hidden="true">⠿</span
                  >
                  <span class="flex-1 px-2 text-right font-semibold tabular-nums">{c.number}</span>
                {:else if column.editor === "check"}
                  <input
                    type="checkbox"
                    class="mx-auto"
                    tabindex="-1"
                    checked={c.inspect}
                    aria-label={m.table_inspect_cell({ number: String(c.number) })}
                    onclick={(event) => {
                      event.preventDefault();
                      setActive(item.index, col);
                      void toggleInspect(c);
                    }}
                  />
                {:else}
                  {@const text = displayText(c, column.id)}
                  <span class="truncate px-2" class:tabular-nums={column.numeric} title={text}
                    >{text}</span
                  >
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {/each}
      {#if drag !== null && drag.gap !== null}
        <div
          class="drop-line pointer-events-none absolute left-0 h-0.5 bg-accent"
          style:top="{drag.gap * ROW_HEIGHT - 1}px"
          style:width="{totalWidth}px"
        ></div>
      {/if}
    </div>
  </div>
</div>

<style>
  .row {
    background: var(--dimo-surface);
    top: 0;
  }

  .row.selected {
    background: color-mix(in srgb, var(--dimo-accent) 16%, var(--dimo-surface));
  }

  /* Shape as well as color: a bar at the start of selected rows. */
  .row.selected::before {
    content: "";
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    width: 3px;
    background: var(--dimo-accent);
    z-index: 1;
  }

  .row.moving {
    opacity: 0.55;
  }

  .grid-scroll:focus-visible .cell.active,
  .grid-scroll:focus .cell.active {
    outline: 2px solid var(--dimo-focus);
    outline-offset: -2px;
  }

  .cell.active {
    outline: 1px dashed var(--dimo-border);
    outline-offset: -2px;
  }

  .handle {
    cursor: grab;
    touch-action: none;
  }

  .handle.locked {
    cursor: not-allowed;
    opacity: 0.4;
  }

  .dragging,
  .dragging .handle {
    cursor: grabbing;
  }
</style>
