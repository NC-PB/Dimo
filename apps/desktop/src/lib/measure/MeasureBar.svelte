<script lang="ts">
  import { m, ruleLabel, unitLabel } from "$lib/i18n";
  import type { Characteristic, LimitPosition, Verdict } from "$lib/ipc/bindings";
  import { cellVerdictText } from "./labels";
  import VerdictMark from "./VerdictMark.svelte";

  /** Where the active cell goes after Enter in the value field, as in the grid. */
  export type BarMove = "up" | "down" | "blur";

  interface Props {
    /** Characteristic of the active row, `undefined` without one. */
    characteristic: Characteristic | undefined;
    /** Serial of the part (sample) of the active column, `undefined` on the result type column. */
    serial: string | undefined;
    /** Text of the value field: the edit in progress, else the stored value. */
    value: string;
    /** True while the active cell is being edited, here or in the cell. */
    editing: boolean;
    /** True if Rust refused the last attempt for this cell. */
    invalid: boolean;
    /** Verdict of the active cell from Rust, `undefined` without a cell. */
    verdict: Verdict | undefined;
    /** Where the stored value lies against the limits, from Rust. */
    position: LimitPosition | null;
    /** The text in the field changed. */
    onInput: (text: string) => void;
    /** Enter (down), Shift+Enter (up) or leaving the field (blur). */
    onCommit: (move: BarMove) => void;
    /** Escape. */
    onCancel: () => void;
    /** A pointer went down on the field: an edit in the cell continues here. */
    onTakeOver: () => void;
  }

  let {
    characteristic,
    serial,
    value,
    editing,
    invalid,
    verdict,
    position,
    onInput,
    onCommit,
    onCancel,
    onTakeOver,
  }: Props = $props();

  /**
   * The bar above the measurement grid (T3.19, mockup "Measure tab"), like the formula bar of a
   * spreadsheet: the active row and part, the requirement with its limits and tolerance rule,
   * the value of the active cell and its verdict in words. The value field and the cell editor
   * show the same edit; Enter commits and moves down as in the grid. Limits and the verdict are
   * Rust's (rule 2); this only puts them into words.
   */
  const uid = $props.id();
  const hasCell = $derived(characteristic !== undefined && serial !== undefined);
  const unit = $derived(characteristic?.unit == null ? "" : unitLabel(characteristic.unit));

  const limits = $derived.by(() => {
    const c = characteristic;
    if (c === undefined) {
      return "";
    }
    const lower = c.lower_limit;
    const upper = c.upper_limit;
    if (lower !== null && upper !== null) {
      return m.measure_bar_limits({ lower, upper, unit }).trim();
    }
    if (upper !== null) {
      return m.measure_bar_limit_max({ value: upper, unit }).trim();
    }
    if (lower !== null) {
      return m.measure_bar_limit_min({ value: lower, unit }).trim();
    }
    return m.measure_bar_no_limits();
  });

  function onKeyDown(event: KeyboardEvent) {
    if (event.isComposing) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      onCommit(event.shiftKey ? "up" : "down");
    } else if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }
</script>

<div
  class="flex min-h-10 shrink-0 flex-wrap items-stretch border-b border-border bg-surface text-sm"
  role="group"
  aria-label={m.measure_bar_label()}
  data-testid="measure-bar"
>
  <div
    class="flex min-w-36 items-center border-r border-border bg-bg px-3 py-1.5 font-semibold"
    data-testid="measure-bar-cell"
  >
    {#if characteristic !== undefined}
      {serial === undefined
        ? m.measure_bar_row({ number: characteristic.number })
        : m.measure_bar_cell({ number: characteristic.number, serial })}
    {:else}
      <span class="font-normal text-text-muted">{m.measure_bar_none()}</span>
    {/if}
  </div>
  {#if characteristic !== undefined}
    <div
      class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-0.5 border-r border-border px-3 py-1.5 text-text-muted"
      data-testid="measure-bar-context"
    >
      <strong
        class="max-w-64 truncate font-semibold text-text"
        title={characteristic.requirement_text}>{characteristic.requirement_text}</strong
      >
      <span class="font-mono text-xs">{limits}</span>
      {#if characteristic.derivation !== null}
        <span class="text-xs">({ruleLabel(characteristic.derivation.rule)})</span>
      {/if}
    </div>
  {/if}
  <div class="flex min-w-60 flex-1 flex-wrap items-center gap-2 px-3 py-1">
    <label for="{uid}-value" class="text-text-muted">{m.measure_bar_value()}</label>
    <input
      id="{uid}-value"
      type="text"
      class="min-h-8 w-36 rounded-md border-2 bg-surface px-2 font-mono text-sm text-text outline-none disabled:border-border disabled:opacity-60 {invalid
        ? 'border-danger'
        : 'border-accent'}"
      autocomplete="off"
      spellcheck="false"
      disabled={!hasCell}
      aria-invalid={invalid}
      title={hasCell ? m.measure_bar_value_desc() : m.reason_no_part()}
      {value}
      data-testid="measure-bar-value"
      onpointerdown={() => {
        onTakeOver();
      }}
      oninput={(event) => {
        onInput(event.currentTarget.value);
      }}
      onkeydown={onKeyDown}
      onblur={() => {
        onCommit("blur");
      }}
    />
    {#if hasCell && verdict !== undefined && !editing}
      <span
        class="flex items-center gap-1 {verdict === 'fail'
          ? 'font-semibold text-danger'
          : verdict === 'pass'
            ? 'text-success'
            : 'text-text-muted'}"
        data-testid="measure-bar-verdict"
        data-verdict={verdict}
      >
        <VerdictMark {verdict} size={13} />
        {cellVerdictText(verdict, position)}
      </span>
    {:else if editing}
      <span class="text-xs text-text-muted">{m.measure_bar_editing()}</span>
    {/if}
  </div>
</div>
