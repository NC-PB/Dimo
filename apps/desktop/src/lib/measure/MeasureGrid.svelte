<script lang="ts">
  import { createColumnHelper, createTable, tableFeatures } from "@tanstack/svelte-table";
  import { untrack } from "svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import ToleranceBadges from "$lib/components/ToleranceBadges.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { m, unitLabel } from "$lib/i18n";
  import type {
    CharId,
    Characteristic,
    Command,
    CommandError,
    MeasurementResult,
    ResultType,
    RowState,
    Sample,
    SampleId,
  } from "$lib/ipc/bindings";
  import { isMacPlatform, matchShortcut } from "$lib/shortcuts";
  import { measurementStore, type MeasurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection, type SelectionStore } from "$lib/stores/selection.svelte";
  import CellEditor, { type EditMove } from "$lib/table/CellEditor.svelte";
  import { indexMap } from "$lib/sets";
  import { refusalDetail } from "$lib/table/refusal";
  import { RowVirtualizer } from "$lib/table/virtual.svelte";
  import { cellKey, editText, parseBlock, pasteEntries, resultsByCell, toggledText } from "./grid";
  import { measureUi, type MeasureUiStore } from "./measure-ui.svelte";
  import { resultTypeLabel, verdictLabel } from "./labels";
  import MeasureBar, { type BarMove } from "./MeasureBar.svelte";
  import VerdictCounts from "./VerdictCounts.svelte";
  import VerdictMark from "./VerdictMark.svelte";

  interface Props {
    /** The project shown; the app's project store unless a test passes its own. */
    project?: ProjectStore;
    /** Verdicts from Rust; the app's store unless a test passes its own. */
    measurement?: MeasurementStore;
    /** The shared selection; the app's selection unless a test passes its own. */
    selected?: SelectionStore;
    /** Called with the sample of the active column, for the sample panel. */
    onActiveSample?: (sample: SampleId | null) => void;
    /** Shared with the Measure ribbon (T3.15): active cell, row filters, grid actions. */
    ui?: MeasureUiStore;
  }

  let {
    project = projectStore,
    measurement = measurementStore,
    selected = selection,
    onActiveSample = () => undefined,
    ui = measureUi,
  }: Props = $props();

  /** Row height, header height and sample column width in CSS px. */
  const ROW_HEIGHT = 28;
  const HEADER_HEIGHT = 44;
  const SAMPLE_WIDTH = 104;
  const OVERSCAN = 8;
  /** Sample columns rendered left and right of the visible ones. */
  const COLUMN_OVERSCAN = 2;

  /** Frozen left columns (M3 decision 7). `type` is the editable result type (decision 4). */
  const FROZEN = [
    { id: "number", size: 52, label: () => m.col_number() },
    { id: "requirement", size: 168, label: () => m.col_requirement_text() },
    { id: "nominal", size: 72, label: () => m.col_nominal() },
    { id: "lower", size: 80, label: () => m.col_lower_limit() },
    { id: "upper", size: 80, label: () => m.col_upper_limit() },
    { id: "unit", size: 44, label: () => m.col_unit() },
    { id: "type", size: 112, label: () => m.field_result_type() },
    { id: "summary", size: 104, label: () => m.measure_col_summary() },
  ] as const;
  const FROZEN_WIDTH = FROZEN.reduce((sum, c) => sum + c.size, 0);
  /** Column index of the result type in keyboard terms: left of the first sample. */
  const TYPE_COL = -1;

  interface GridRow {
    state: RowState;
    c: Characteristic;
  }

  // TanStack Table holds the row model: the measured characteristics in number order, as Rust
  // lists them (decision 5). Sample columns are windowed by hand below.
  const features = tableFeatures({});
  const helper = createColumnHelper<typeof features, GridRow>();
  const columns = FROZEN.map((spec) =>
    helper.accessor((r: GridRow): unknown => r.c.id, { id: spec.id }),
  );
  /** The active cell: a row by characteristic and a sample column (or the result type). */
  let active = $state<{ id: CharId; col: number } | null>(null);

  /**
   * Row filters of the ribbon (T3.15) read Rust's verdicts only. The active row stays while a
   * filter is on, so a value typed into it never makes the cursor jump.
   */
  function shown(state: RowState): boolean {
    const missing = !ui.onlyMissing || state.counts.missing > 0;
    const failed = !ui.onlyFailed || state.verdict === "fail";
    return (missing && failed) || state.characteristic === active?.id;
  }

  const gridRows = $derived(
    measurement.rows.flatMap((state) => {
      const c = project.characteristicById.get(state.characteristic);
      return c === undefined || !shown(state) ? [] : [{ state, c }];
    }),
  );
  const table = createTable({
    features,
    columns,
    get data() {
      return gridRows;
    },
    getRowId: (r: GridRow) => r.c.id,
  });
  const rows = $derived(table.getRowModel().rows.map((r) => r.original));
  const order = $derived(rows.map((r) => r.c.id));
  const indexById = $derived(indexMap(order));

  const samples = $derived<readonly Sample[]>(project.project?.samples ?? []);
  const sampleIds = $derived(samples.map((s) => s.id));
  const results = $derived(resultsByCell(project.project?.results ?? []));
  const totalWidth = $derived(FROZEN_WIDTH + samples.length * SAMPLE_WIDTH);

  const virtualizer = new RowVirtualizer({
    rowHeight: ROW_HEIGHT,
    overscan: OVERSCAN,
    headerHeight: HEADER_HEIGHT,
  });

  let grid = $state<HTMLElement | null>(null);
  let trap = $state<HTMLTextAreaElement | null>(null);
  let scrollLeft = $state(0);
  let viewWidth = $state(1200);

  $effect(() => {
    const element = grid;
    if (element === null) {
      return;
    }
    const detach = virtualizer.attach(element);
    const measure = () => {
      viewWidth = element.clientWidth || element.offsetWidth || 1200;
    };
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(element);
    return () => {
      observer?.disconnect();
      detach();
    };
  });

  $effect.pre(() => {
    virtualizer.setCount(rows.length);
  });

  /** Sample columns in view plus a few on each side; all of them are never rendered. */
  const shownSamples = $derived.by(() => {
    const first = Math.max(0, Math.floor(scrollLeft / SAMPLE_WIDTH) - COLUMN_OVERSCAN);
    const visible = Math.ceil(Math.max(0, viewWidth - FROZEN_WIDTH) / SAMPLE_WIDTH);
    const last = Math.min(samples.length - 1, first + visible + 2 * COLUMN_OVERSCAN);
    const shown: { sample: Sample; col: number }[] = [];
    for (let col = first; col <= last; col++) {
      const sample = samples[col];
      if (sample) {
        shown.push({ sample, col });
      }
    }
    return shown;
  });

  /**
   * The cell being typed in. `error`: Rust refused the last attempt. `source`: typed in the cell
   * (its editor has the focus) or in the value field of the Measure bar (T3.19); both show the
   * same text.
   */
  let editing = $state<{
    id: CharId;
    sample: SampleId;
    text: string;
    selectAll: boolean;
    error: boolean;
    source: "cell" | "bar";
  } | null>(null);
  /** A pointer went down on the bar's value field while the cell editor had the focus. */
  let takeover = false;
  /** Why the last change was refused; one line above the grid. */
  let notice = $state<string | null>(null);

  const activeRow = $derived(active === null ? -1 : (indexById.get(active.id) ?? -1));

  $effect(() => {
    const col = active?.col ?? -1;
    const sample = col >= 0 ? (sampleIds[col] ?? null) : null;
    onActiveSample(sample);
    ui.activeSample = sample;
    ui.activeRow = active?.id ?? null;
  });

  // The ribbon's paste, clear and result type act through the grid (T3.15).
  $effect(() =>
    ui.attach({
      paste: (text) => paste(text),
      clear: async () => {
        const id = active?.id;
        const sample = active === null ? undefined : sampleIds[active.col];
        if (id !== undefined && sample !== undefined) {
          await clearCell(id, sample);
        }
      },
      setResultType: async (value) => {
        const c = active === null ? undefined : project.characteristicById.get(active.id);
        if (c !== undefined) {
          await setResultType(c, value);
        }
      },
      focus: () => {
        focusGrid();
      },
      notify: (text) => {
        notice = text;
      },
    }),
  );

  function cellId(id: CharId, col: number): string {
    return col === TYPE_COL ? `mg-type-${id}` : `mg-${id}-${String(col)}`;
  }

  const activeDescendant = $derived.by(() => {
    if (active === null || activeRow < 0) {
      return undefined;
    }
    const rowShown = virtualizer.items.some((item) => item.index === activeRow);
    const colShown = active.col === TYPE_COL || shownSamples.some((s) => s.col === active?.col);
    return rowShown && colShown ? `${cellId(active.id, active.col)}-cell` : undefined;
  });

  function focusGrid() {
    grid?.focus({ preventScroll: true });
  }

  /** Scrolls row `index` and sample column `col` into view. */
  function reveal(index: number, col: number) {
    virtualizer.scrollToIndex(index);
    const element = grid;
    if (element === null || col < 0) {
      return;
    }
    const left = col * SAMPLE_WIDTH; // scroll offset at which the column is first after the frozen ones
    const room = Math.max(SAMPLE_WIDTH, element.clientWidth - FROZEN_WIDTH);
    if (left < element.scrollLeft) {
      element.scrollLeft = left;
    } else if (left + SAMPLE_WIDTH > element.scrollLeft + room) {
      element.scrollLeft = left + SAMPLE_WIDTH - room;
    }
  }

  /** Makes a cell active (clamped) and selects its characteristic in the shared selection. */
  function setActive(index: number, col: number) {
    const target = Math.max(0, Math.min(order.length - 1, index));
    const id = order[target];
    if (id === undefined) {
      return;
    }
    const lastCol = samples.length - 1;
    active = { id, col: Math.max(TYPE_COL, Math.min(lastCol, col)) };
    reveal(target, active.col);
    if (selected.primary !== id || selected.size !== 1) {
      selected.select([id]);
    }
  }

  /**
   * Sends a command. Returns `null` on success, else Rust's refusal, which the grid shows itself
   * (so the window wide error line does not repeat it).
   */
  async function send(command: Command): Promise<CommandError | null> {
    const patch = await project.execute(command);
    if (patch !== undefined) {
      return null;
    }
    const error = project.error;
    project.dismissError();
    return error ?? { kind: "invalid_argument", message: "" };
  }

  /** Why a value for one cell was refused, naming the cell. */
  function cellRefused(error: CommandError, id: CharId, sample: SampleId): string {
    const c = project.characteristicById.get(id);
    const serial = samples.find((s) => s.id === sample)?.serial ?? "";
    let message = refusalDetail(error);
    if (error.kind === "rejected" && error.reason === "invalid_result") {
      message =
        measurement.row(id)?.result_type === "attribute"
          ? m.measure_refused_attribute()
          : m.measure_refused_number();
    }
    return m.measure_cell_refused({ number: c?.number ?? "?", serial, message });
  }

  /** Dev builds: time from a commit to the first frame with Rust's new verdicts. */
  function timeCommit(start: number) {
    if (!DEV_TOOLS_ENABLED) {
      return;
    }
    const before = measurement.state;
    void (async () => {
      for (let i = 0; i < 500 && measurement.state === before; i++) {
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
      devTools.commitTimes.push(performance.now() - start);
    })();
  }

  /**
   * Sets one cell from text and reports a refusal. The active cell has already moved on, so a
   * fast typist keeps going; a refused value is opened again for correction.
   */
  async function setCell(id: CharId, sample: SampleId, text: string, reopen: boolean) {
    const start = performance.now();
    const refused = await send({
      type: "set_results",
      results: [{ sample, characteristic: id, text, comment: null }],
    });
    if (refused === null) {
      notice = null;
      timeCommit(start);
      return;
    }
    notice = cellRefused(refused, id, sample);
    const index = indexById.get(id);
    const col = sampleIds.indexOf(sample);
    if (reopen && editing === null && index !== undefined && col >= 0) {
      setActive(index, col);
      editing = { id, sample, text, selectAll: false, error: true, source: "cell" };
    }
  }

  function startEdit(index: number, col: number, typed: string | null = null) {
    const r = rows[index];
    const sample = sampleIds[col];
    if (r === undefined) {
      return;
    }
    if (col === TYPE_COL) {
      setActive(index, col);
      document.getElementById(cellId(r.c.id, TYPE_COL))?.focus();
      return;
    }
    if (sample === undefined) {
      return;
    }
    setActive(index, col);
    takeover = false;
    editing = {
      id: r.c.id,
      sample,
      text: typed ?? editText(results.get(cellKey(sample, r.c.id))),
      selectAll: typed === null,
      error: false,
      source: "cell",
    };
  }

  function cancelEdit() {
    takeover = false;
    editing = null;
    notice = null;
    focusGrid();
  }

  /** Moves the active cell after a key or a confirmed edit. */
  function move(step: EditMove) {
    const index = Math.max(0, activeRow);
    const col = active?.col ?? 0;
    switch (step) {
      case "up":
        setActive(index - 1, col);
        break;
      case "down":
        setActive(index + 1, col);
        break;
      case "next":
        setActive(index, col + 1);
        break;
      case "previous":
        setActive(index, col - 1);
        break;
      case "none":
      case "blur":
        break;
    }
  }

  /** Confirms the edit: Rust parses and validates the text (M3 decisions 7 and 8). */
  function commitEdit(step: EditMove) {
    const edit = editing;
    if (edit === null) {
      return;
    }
    if (step === "blur" && takeover) {
      // The cell editor lost the focus to the bar's value field: the edit goes on there.
      takeover = false;
      editing = { ...edit, source: "bar" };
      return;
    }
    editing = null;
    if (step !== "blur") {
      focusGrid();
      move(step);
    }
    if (edit.text === editText(results.get(cellKey(edit.sample, edit.id))) && !edit.error) {
      return;
    }
    void setCell(edit.id, edit.sample, edit.text, step !== "blur");
  }

  // The Measure bar (T3.19): its value field edits the active cell like the cell editor does.
  const activeChar = $derived(active === null ? undefined : rows[activeRow]?.c);
  const activeSampleId = $derived(active === null ? undefined : sampleIds[active.col]);
  const activeEditing = $derived(
    editing !== null && editing.id === activeChar?.id && editing.sample === activeSampleId
      ? editing
      : null,
  );

  function onBarInput(text: string) {
    if (editing !== null) {
      editing = { ...editing, text, error: false };
      return;
    }
    const id = activeChar?.id;
    const sample = activeSampleId;
    if (id !== undefined && sample !== undefined) {
      editing = { id, sample, text, selectAll: false, error: false, source: "bar" };
    }
  }

  function onBarCommit(step: BarMove) {
    if (editing?.source === "bar") {
      commitEdit(step);
    } else if (editing === null && step !== "blur") {
      // Enter on an unchanged value moves on, as Enter on the grid does after an edit.
      focusGrid();
      move(step);
    }
  }

  function onBarCancel() {
    if (editing?.source === "bar") {
      cancelEdit();
    } else {
      focusGrid();
    }
  }

  function onBarTakeOver() {
    takeover = editing?.source === "cell";
  }

  async function clearCell(id: CharId, sample: SampleId) {
    if (!results.has(cellKey(sample, id))) {
      return;
    }
    const refused = await send({
      type: "clear_results",
      cells: [{ sample, characteristic: id }],
    });
    notice = refused === null ? null : cellRefused(refused, id, sample);
  }

  /** Sets the result type of a row; returns false when Rust refused it. */
  async function setResultType(c: Characteristic, value: ResultType): Promise<boolean> {
    if (value === c.result_type) {
      return true;
    }
    const refused = await send({ type: "set_result_type", ids: [c.id], result_type: value });
    if (refused === null) {
      notice = null;
      return true;
    }
    notice = m.measure_type_refused({ number: c.number, message: refusalDetail(refused) });
    return false;
  }

  async function onResultTypeChange(c: Characteristic, select: HTMLSelectElement) {
    if (!(await setResultType(c, select.value as ResultType))) {
      select.value = project.characteristicById.get(c.id)?.result_type ?? c.result_type;
    }
  }

  /** Pastes a block of cells from the clipboard as one `set_results` (one undo step). */
  async function paste(text: string) {
    const block = parseBlock(text);
    const index = activeRow;
    const col = active?.col ?? -1;
    if (block.length === 0 || index < 0) {
      return;
    }
    if (col < 0) {
      notice = m.measure_paste_no_cell();
      return;
    }
    const entries = pasteEntries(block, order, sampleIds, index, col);
    if (!Array.isArray(entries)) {
      notice = m.measure_paste_too_big({
        rows: String(entries.rows),
        columns: String(entries.columns),
      });
      return;
    }
    const refused = await send({ type: "set_results", results: entries });
    notice = refused === null ? null : m.measure_paste_refused({ message: pasteRefusal(refused) });
  }

  function pasteRefusal(error: CommandError): string {
    return error.kind === "rejected" && error.reason === "invalid_result"
      ? m.measure_paste_invalid()
      : refusalDetail(error);
  }

  function onPaste(event: ClipboardEvent) {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) {
      return; // Pasting into the cell editor is plain text editing.
    }
    event.preventDefault();
    const text = event.clipboardData?.getData("text/plain") ?? "";
    focusGrid();
    void paste(text);
  }

  /**
   * Cmd/Ctrl+V on the grid: the browser pastes only into a text field, so the focus moves to a
   * hidden one for the paste; its `paste` event bubbles to the grid, which takes the focus back.
   */
  function catchPaste() {
    const field = trap;
    if (field === null) {
      return;
    }
    field.value = "";
    field.focus({ preventScroll: true });
    setTimeout(() => {
      if (document.activeElement === field) {
        focusGrid();
      }
    }, 300);
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.target !== grid || event.defaultPrevented || event.isComposing) {
      return; // Keys inside the editor or the result type picker belong to them.
    }
    let shortcut = matchShortcut(event, isMacPlatform(), "grid");
    const index = activeRow;
    const col = active?.col ?? 0;
    const r = index >= 0 ? rows[index] : undefined;
    const sample = col >= 0 ? sampleIds[col] : undefined;
    const attribute = r !== undefined && measurement.row(r.c.id)?.result_type === "attribute";
    if (
      (shortcut?.action === "grid_ok" || shortcut?.action === "grid_not_ok") &&
      !(attribute && sample !== undefined)
    ) {
      shortcut = null; // On a value cell these are typed characters.
    }
    if (shortcut === null) {
      // A printable character starts editing the active cell with that character. Either way
      // it must not reach the window shortcuts, except the cheat sheet (D-52).
      if (matchShortcut(event)?.action === "show_shortcuts") {
        return;
      }
      if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        if (index >= 0 && col >= 0) {
          startEdit(index, col, event.key);
        }
      }
      return;
    }
    if (shortcut.action === "grid_paste") {
      catchPaste(); // No preventDefault: the browser's paste must still happen.
      return;
    }
    // Tab past the last sample and Shift+Tab before the first leave the grid, so the keyboard
    // reaches the sample panel and the sample bar (NFR-UX-01, no keyboard trap).
    if (
      (shortcut.action === "grid_next_sample" && col >= samples.length - 1) ||
      (shortcut.action === "grid_previous_sample" && col <= 0)
    ) {
      return;
    }
    event.preventDefault();
    if (order.length === 0) {
      return;
    }
    if (index < 0) {
      setActive(0, Math.min(0, samples.length - 1));
      return;
    }
    switch (shortcut.action) {
      case "grid_up":
      case "grid_previous_row":
        move("up");
        break;
      case "grid_down":
      case "grid_next_row":
        move("down");
        break;
      case "grid_left":
        setActive(index, col - 1);
        break;
      case "grid_right":
      case "grid_next_sample":
        setActive(index, col + 1);
        break;
      case "grid_previous_sample":
        setActive(index, Math.max(0, col - 1));
        break;
      case "grid_first_sample":
        setActive(index, 0);
        break;
      case "grid_last_sample":
        setActive(index, samples.length - 1);
        break;
      case "grid_first_row":
        setActive(0, col);
        break;
      case "grid_last_row":
        setActive(order.length - 1, col);
        break;
      case "grid_edit":
        startEdit(index, col);
        break;
      case "grid_clear":
        if (r && sample !== undefined) {
          void clearCell(r.c.id, sample);
        }
        break;
      case "grid_cancel":
        notice = null;
        break;
      case "grid_ok":
      case "grid_not_ok":
        if (r && sample !== undefined) {
          const text = shortcut.action === "grid_ok" ? "ok" : "not ok";
          move("down"); // As Enter after a typed value: the next characteristic of this part.
          void setCell(r.c.id, sample, text, false);
        }
        break;
      case "grid_toggle":
        if (r && sample !== undefined && attribute) {
          void setCell(r.c.id, sample, toggledText(results.get(cellKey(sample, r.c.id))), false);
        }
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
    // Read only columns keep the sample column of the active cell.
    const col = cell.dataset.col === "keep" ? (active?.col ?? 0) : Number(cell.dataset.col);
    return { index: Number(row.dataset.row), col };
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) {
      return;
    }
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest("input, select")) {
      return;
    }
    const header = target?.closest<HTMLElement>("[data-sample-col]");
    if (header) {
      setActive(Math.max(0, activeRow), Number(header.dataset.sampleCol));
      return;
    }
    const hit = cellAt(target);
    if (hit !== null) {
      setActive(hit.index, hit.col);
    }
  }

  function onDoubleClick(event: MouseEvent) {
    const hit = cellAt(event.target);
    if (hit !== null && hit.col >= 0) {
      startEdit(hit.index, hit.col);
    }
  }

  // Keep the active cell valid when rows or samples change (delete, undo).
  $effect(() => {
    const ids = order;
    const count = samples.length;
    untrack(() => {
      if (active === null) {
        const primary = selected.primary;
        const index = primary === null ? -1 : ids.indexOf(primary);
        if (ids.length > 0) {
          active = { id: ids[Math.max(0, index)] ?? "", col: count > 0 ? 0 : TYPE_COL };
        }
        return;
      }
      if (!ids.includes(active.id)) {
        active = ids.length > 0 ? { id: ids[0] ?? "", col: active.col } : null;
      }
      if (active !== null && active.col >= count) {
        active = { ...active, col: count - 1 };
      }
    });
  });

  /** Shown text of a result. */
  function valueText(result: MeasurementResult | undefined): string {
    if (result === undefined) {
      return "";
    }
    if (result.value.type === "variable") {
      return result.value.value;
    }
    return result.value.value === "ok" ? m.result_ok() : m.result_not_ok();
  }

  const report = $derived(measurement.state);
</script>

<!-- The measurement grid (T3.5, FR-MEA-02): rows are measured characteristics, one column per
     sample. A virtualized ARIA grid; the focus stays on it and `aria-activedescendant` names
     the active cell. -->
<div class="flex min-h-0 flex-1 flex-col">
  <MeasureBar
    characteristic={activeChar}
    serial={samples.find((s) => s.id === activeSampleId)?.serial}
    value={activeEditing?.text ??
      (activeChar !== undefined && activeSampleId !== undefined
        ? editText(results.get(cellKey(activeSampleId, activeChar.id)))
        : "")}
    editing={activeEditing !== null}
    invalid={activeEditing?.error ?? false}
    verdict={activeChar !== undefined && activeSampleId !== undefined
      ? (measurement.verdict(activeChar.id, activeSampleId) ?? "missing")
      : undefined}
    position={activeChar !== undefined && activeSampleId !== undefined
      ? measurement.position(activeChar.id, activeSampleId)
      : null}
    onInput={onBarInput}
    onCommit={onBarCommit}
    onCancel={onBarCancel}
    onTakeOver={onBarTakeOver}
  />
  <header class="flex h-7 shrink-0 items-center gap-3 px-3 text-xs text-text-muted">
    <h2 class="font-semibold text-text">{m.measure_grid_label()}</h2>
    <span class="shrink-0"
      >{m.measure_grid_count({
        rows: String(rows.length),
        samples: String(samples.length),
      })}</span
    >
    {#if report}
      <span class="flex shrink-0 items-center gap-1" data-testid="overall-verdict">
        {m.measure_overall()}
        {#if report.verdict !== "open"}
          <VerdictMark verdict={report.verdict} size={12} />
        {/if}
        <span
          class:font-bold={report.verdict === "fail"}
          class:text-danger={report.verdict === "fail"}>{verdictLabel(report.verdict)}</span
        >
      </span>
    {/if}
    <span class="flex min-w-0 flex-1" role="alert" title={notice ?? undefined}>
      {#if notice !== null}
        <span class="flex min-w-0 items-center gap-1 text-danger">
          <svg class="size-3.5 shrink-0" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              d="M8 1.5 15 14H1L8 1.5Z"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linejoin="round"
            />
            <path d="M8 6.5v3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            <circle cx="8" cy="12" r="0.9" fill="currentColor" />
          </svg>
          <span class="truncate" data-testid="grid-notice">{notice}</span>
        </span>
      {/if}
    </span>
  </header>
  <div
    bind:this={grid}
    class="grid-scroll relative min-h-0 flex-1 overflow-auto text-sm outline-none"
    role="grid"
    tabindex="0"
    aria-label={m.measure_grid_label()}
    aria-rowcount={rows.length + 1}
    aria-colcount={FROZEN.length + samples.length}
    aria-activedescendant={activeDescendant}
    onkeydown={onKeyDown}
    onpointerdown={onPointerDown}
    ondblclick={onDoubleClick}
    onpaste={onPaste}
    onscroll={(event) => {
      scrollLeft = event.currentTarget.scrollLeft;
    }}
  >
    <textarea
      bind:this={trap}
      class="paste-trap"
      tabindex="-1"
      aria-hidden="true"
      onblur={() => {
        if (trap) {
          trap.value = "";
        }
      }}></textarea>
    <div class="sticky top-0 z-20" role="rowgroup" style:width="{totalWidth}px">
      <div
        class="relative flex border-b border-border bg-surface-raised text-xs font-semibold text-text-muted"
        role="row"
        aria-rowindex={1}
        style:height="{HEADER_HEIGHT}px"
      >
        <div class="frozen flex bg-surface-raised">
          {#each FROZEN as spec, i (spec.id)}
            <div
              class="flex shrink-0 items-end border-r border-border px-2 pb-1"
              role="columnheader"
              aria-colindex={i + 1}
              style:width="{spec.size}px"
            >
              <span class="truncate">{spec.label()}</span>
            </div>
          {/each}
        </div>
        {#each shownSamples as { sample, col } (sample.id)}
          {@const state = measurement.sample(sample.id)}
          <div
            class="sample-head absolute top-0 flex h-full flex-col justify-center gap-0.5 border-r border-border px-2"
            class:active-col={active?.col === col}
            role="columnheader"
            aria-colindex={FROZEN.length + col + 1}
            data-sample-col={col}
            style:left="{FROZEN_WIDTH + col * SAMPLE_WIDTH}px"
            style:width="{SAMPLE_WIDTH}px"
            title={m.measure_sample_title({ serial: sample.serial })}
          >
            <span class="truncate text-text">{sample.serial}</span>
            {#if state}
              <VerdictCounts counts={state.counts} />
            {/if}
          </div>
        {/each}
      </div>
    </div>
    {#if measurement.state !== null && (rows.length === 0 || samples.length === 0)}
      <div class="sticky left-0 w-fit max-w-xl">
        {#if rows.length === 0 && measurement.rows.length > 0}
          <EmptyState
            size="inline"
            title={m.empty_measure_filtered_title()}
            text={m.empty_measure_filtered_text()}
            actions={[
              ...(ui.onlyMissing ? [{ command: "only_missing" as const }] : []),
              ...(ui.onlyFailed ? [{ command: "only_failed" as const }] : []),
            ]}
          />
        {:else if rows.length === 0 && project.characteristics.length === 0}
          <EmptyState
            size="inline"
            title={m.empty_measure_no_chars_title()}
            text={m.empty_measure_no_chars_text()}
            actions={[{ command: "place_tool" }]}
          />
        {:else if rows.length === 0}
          <EmptyState
            size="inline"
            title={m.empty_measure_no_chars_title()}
            text={m.empty_measure_not_inspected()}
            actions={[{ tab: "home", label: m.empty_go_drawing() }]}
          />
        {:else}
          <EmptyState
            size="inline"
            title={m.empty_measure_no_parts_title()}
            text={m.empty_measure_no_parts_text()}
            actions={[{ command: "add_parts" }]}
          />
        {/if}
      </div>
    {/if}
    <div
      class="relative"
      role="rowgroup"
      style:height="{virtualizer.totalSize}px"
      style:width="{totalWidth}px"
    >
      {#each virtualizer.items as item (rows[item.index]?.c.id ?? item.key)}
        {@const r = rows[item.index]}
        {#if r}
          {@const c = r.c}
          {@const isSelected = selected.has(c.id)}
          <div
            class="row absolute left-0 flex border-b border-border"
            class:selected={isSelected}
            role="row"
            aria-rowindex={item.index + 2}
            aria-selected={isSelected}
            data-row={item.index}
            style:transform="translateY({virtualizer.rowTop(item)}px)"
            style:height="{ROW_HEIGHT}px"
            style:width="{totalWidth}px"
          >
            <div class="frozen flex">
              <div
                class="cell flex shrink-0 items-center gap-1 border-r border-border px-2"
                role="rowheader"
                aria-colindex={1}
                data-col="keep"
                style:width="{FROZEN[0].size}px"
              >
                {#if measurement.changed.has(c.id)}
                  <!-- Decision 16: limits or result type changed after the last issued report. -->
                  <svg
                    class="size-3 shrink-0 text-warning"
                    viewBox="0 0 16 16"
                    fill="none"
                    role="img"
                    aria-label={m.measure_changed_since_issue()}
                    data-testid="changed-hint"
                  >
                    <title>{m.measure_changed_since_issue()}</title>
                    <path
                      d="M8 2 14.5 13.5H1.5L8 2Z"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linejoin="round"
                    />
                  </svg>
                {/if}
                <span class="flex-1 text-right font-semibold tabular-nums">{c.number}</span>
              </div>
              <div
                class="cell flex shrink-0 items-center gap-1 border-r border-border px-2"
                role="gridcell"
                aria-colindex={2}
                aria-readonly="true"
                data-col="keep"
                style:width="{FROZEN[1].size}px"
                title={c.requirement_text}
              >
                <span class="truncate">{c.requirement_text}</span>
                <ToleranceBadges derivation={c.derivation} compact />
              </div>
              {#each [c.nominal, c.lower_limit, c.upper_limit] as value, i (i)}
                <div
                  class="cell flex shrink-0 items-center justify-end border-r border-border px-2 tabular-nums"
                  role="gridcell"
                  aria-colindex={3 + i}
                  aria-readonly="true"
                  data-col="keep"
                  style:width="{FROZEN[2 + i]?.size ?? 0}px"
                >
                  <span class="truncate">{value ?? ""}</span>
                </div>
              {/each}
              <div
                class="cell flex shrink-0 items-center border-r border-border px-2 text-text-muted"
                role="gridcell"
                aria-colindex={6}
                aria-readonly="true"
                data-col="keep"
                style:width="{FROZEN[5].size}px"
              >
                {c.unit === null ? "" : unitLabel(c.unit)}
              </div>
              <div
                id="{cellId(c.id, TYPE_COL)}-cell"
                class="cell flex shrink-0 items-center border-r border-border px-1"
                class:active={active?.id === c.id && active.col === TYPE_COL}
                role="gridcell"
                aria-colindex={7}
                data-col={TYPE_COL}
                style:width="{FROZEN[6].size}px"
              >
                <select
                  id={cellId(c.id, TYPE_COL)}
                  class="h-6 w-full min-w-0 rounded border border-border bg-surface px-1 text-xs text-text"
                  tabindex="-1"
                  aria-label={m.measure_type_label({ number: c.number })}
                  value={c.result_type}
                  onchange={(event) => {
                    void onResultTypeChange(c, event.currentTarget);
                    focusGrid();
                  }}
                  onkeydown={(event) => {
                    if (event.key === "Escape") {
                      event.preventDefault();
                      focusGrid();
                    }
                  }}
                >
                  <option value="auto">{resultTypeLabel("auto", r.state.result_type)}</option>
                  <option value="variable">{resultTypeLabel("variable")}</option>
                  <option value="attribute">{resultTypeLabel("attribute")}</option>
                </select>
              </div>
              <div
                class="cell flex shrink-0 items-center border-r-2 border-border px-2"
                role="gridcell"
                aria-colindex={8}
                aria-readonly="true"
                data-col="keep"
                style:width="{FROZEN[7].size}px"
              >
                <VerdictCounts counts={r.state.counts} />
              </div>
            </div>
            {#each shownSamples as { sample, col } (sample.id)}
              {@const result = results.get(cellKey(sample.id, c.id))}
              {@const verdict = measurement.verdict(c.id, sample.id) ?? "missing"}
              {@const isActive = active?.id === c.id && active.col === col}
              {@const isEditing = editing?.id === c.id && editing.sample === sample.id}
              <div
                id="{cellId(c.id, col)}-cell"
                class="cell value absolute top-0 flex h-full items-center gap-1 border-r border-border"
                class:active={isActive}
                class:fail={verdict === "fail"}
                role="gridcell"
                aria-colindex={FROZEN.length + col + 1}
                data-col={col}
                data-verdict={verdict}
                style:left="{FROZEN_WIDTH + col * SAMPLE_WIDTH}px"
                style:width="{SAMPLE_WIDTH}px"
              >
                {#if isEditing && editing && editing.source === "cell"}
                  <CellEditor
                    text={editing.text}
                    pending={false}
                    invalid={editing.error}
                    numeric={r.state.result_type === "variable"}
                    label={m.measure_cell_label({ number: c.number, serial: sample.serial })}
                    selectAll={editing.selectAll}
                    enterMoves
                    onInput={(text) => {
                      if (editing) {
                        editing = { ...editing, text, error: false };
                      }
                    }}
                    onCommit={commitEdit}
                    onCancel={cancelEdit}
                  />
                {:else if isEditing && editing}
                  <!-- Typed in the Measure bar: the cell shows the same text (T3.19). -->
                  <span
                    class="editing-text flex h-full flex-1 items-center justify-end truncate px-2 tabular-nums"
                    class:invalid={editing.error}
                    data-testid="cell-bar-edit">{editing.text}</span
                  >
                {:else}
                  <span class="flex w-4 shrink-0 justify-center pl-1">
                    <VerdictMark {verdict} />
                  </span>
                  <span
                    class="flex-1 truncate pr-2 text-right tabular-nums"
                    class:font-bold={verdict === "fail"}>{valueText(result)}</span
                  >
                  {#if verdict !== "missing"}
                    <span class="sr-only">{verdictLabel(verdict)}</span>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .frozen {
    position: sticky;
    left: 0;
    z-index: 2;
    background: var(--dimo-surface);
  }

  .row {
    background: var(--dimo-surface);
    top: 0;
  }

  .row.selected,
  .row.selected .frozen {
    background: color-mix(in srgb, var(--dimo-accent) 12%, var(--dimo-surface));
  }

  /* Shape as well as color: a bar at the start of the selected row. */
  .row.selected .frozen::before {
    content: "";
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    width: 3px;
    background: var(--dimo-accent);
    z-index: 1;
  }

  /* Fail: a bold border around the cell as well as the cross (M3 decision 9). */
  .cell.value.fail {
    box-shadow: inset 0 0 0 2px var(--dimo-danger);
  }

  .sample-head.active-col {
    box-shadow: inset 0 -3px 0 var(--dimo-accent);
  }

  .grid-scroll:focus-visible .cell.active,
  .grid-scroll:focus .cell.active {
    outline: 2px solid var(--dimo-focus);
    outline-offset: -3px;
  }

  .cell.active {
    outline: 1px dashed var(--dimo-border);
    outline-offset: -3px;
  }

  /* The edit of the Measure bar shown in its cell, outlined like the cell editor. */
  .editing-text {
    outline: 2px solid var(--dimo-focus);
    outline-offset: -2px;
  }

  .editing-text.invalid {
    outline-color: var(--dimo-danger);
  }

  /* Takes the focus for a paste (see catchPaste); invisible and out of the layout. */
  .paste-trap {
    position: fixed;
    top: 0;
    left: 0;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
    resize: none;
  }
</style>
