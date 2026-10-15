<script lang="ts">
  /**
   * The properties of one characteristic in the right pane (T2.8, T3.18, mockup "Main"):
   * Requirement (text, kind, quantity), Tolerance (rule with markers, limits, the shorter leg of
   * an angle (T2.9a), the explanation Rust builds in the UI language (FR-TOL-08), Update limits),
   * Inspection (inspect flag FR-CHR-08, result type FR-MEA-06, method and gauge) and History
   * (FR-CHR-10). Every edit is one undoable Rust command (`update_fields`, `set_result_type`);
   * Rust reads and validates the values (rule 5), the field then shows what the project holds.
   */
  import { fieldClass } from "$lib/ui/fields";
  import {
    changeSourceLabel,
    commandErrorMessage,
    fieldLabel,
    getLocale,
    kindLabel,
    m,
    ruleLabel,
    unitLabel,
  } from "$lib/i18n";
  import type { CharId, FieldValue, HistoryEntry, ResultType, Unit } from "$lib/ipc/bindings";
  import { resultTypeLabel } from "$lib/measure/labels";
  import PaneSection from "$lib/pane/PaneSection.svelte";
  import { icons } from "$lib/icons";
  import { explainLanguage } from "$lib/stores/box-select.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection as appSelection, type SelectionStore } from "$lib/stores/selection.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import {
    COLUMNS,
    KINDS,
    displayText,
    fieldValue,
    rawText,
    type ColumnId,
  } from "$lib/table/columns";
  import { paneFolds } from "$lib/pane/folds.svelte";
  import ToleranceBadges from "./ToleranceBadges.svelte";

  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    project?: ProjectStore;
    selected?: SelectionStore;
    tolerance?: ToleranceStore;
  }

  let { project = appProject, selected = appSelection, tolerance = appTolerance }: Props = $props();

  const HISTORY = "characteristic.history";
  const uid = $props.id();

  const c = $derived(
    selected.primary === null ? null : (project.characteristicById.get(selected.primary) ?? null),
  );

  /** The explanation for the shown characteristic: loading, text (or `null`) or an error. */
  let explanation = $state<
    { state: "loading" } | { state: "ok"; text: string | null } | { state: "error"; text: string }
  >({ state: "loading" });
  let history = $state<
    | { state: "loading" }
    | { state: "ok"; entries: HistoryEntry[] }
    | { state: "error"; text: string }
  >({ state: "loading" });
  let explainSeq = 0;
  let historySeq = 0;

  // A patch gives the characteristic a new object, so both follow every change, undo and redo.
  $effect(() => {
    const shown = c;
    const language = explainLanguage();
    const seq = ++explainSeq;
    if (shown === null) {
      return;
    }
    explanation = { state: "loading" };
    void tolerance.explanation(shown.id, language).then((result) => {
      if (seq !== explainSeq) {
        return;
      }
      explanation =
        result.status === "ok"
          ? { state: "ok", text: result.data }
          : { state: "error", text: commandErrorMessage(result.error) };
    });
  });

  // The history is asked for only while its section is open.
  $effect(() => {
    const shown = c;
    void project.revision;
    const seq = ++historySeq;
    if (shown === null || !paneFolds.isOpen(HISTORY)) {
      return;
    }
    history = { state: "loading" };
    void tolerance.history(shown.id).then((result) => {
      if (seq !== historySeq) {
        return;
      }
      history =
        result.status === "ok"
          ? { state: "ok", entries: [...result.data].reverse() }
          : { state: "error", text: commandErrorMessage(result.error) };
    });
  });

  function when(timestamp: string): string {
    const date = new Date(timestamp);
    return Number.isNaN(date.getTime()) ? timestamp : date.toLocaleString(getLocale());
  }

  function what(entry: HistoryEntry): string {
    if (entry.before === null) {
      return m.history_created();
    }
    if (entry.after === null) {
      return m.history_removed();
    }
    return m.history_fields({ fields: entry.fields.map(fieldLabel).join(", ") });
  }

  /**
   * Sends one typed field with `update_fields`. Afterwards the field shows what the project
   * holds: Rust's reading, or the old value when Rust refused (the reason appears in the project
   * messages).
   */
  async function edit(
    field: HTMLInputElement | HTMLSelectElement,
    id: CharId,
    column: ColumnId,
  ): Promise<void> {
    const current = project.characteristicById.get(id);
    if (current === undefined || field.value === rawText(current, column)) {
      return;
    }
    const value: FieldValue | null = fieldValue(column, field.value);
    if (value !== null) {
      await project.execute({ type: "update_fields", ids: [id], values: [value] });
    }
    const after = project.characteristicById.get(id);
    field.value = after === undefined ? "" : rawText(after, column);
  }

  /**
   * Overrides the result type (M3 decision 4). Rust refuses it while results of the other type
   * exist; the refusal shows in the project messages and the picker goes back.
   */
  async function setResultType(select: HTMLSelectElement) {
    if (c === null) {
      return;
    }
    const id = c.id;
    await project.execute({
      type: "set_result_type",
      ids: [id],
      result_type: select.value as ResultType,
    });
    select.value = project.characteristicById.get(id)?.result_type ?? select.value;
  }

  async function setInspect(inspect: boolean) {
    if (c !== null) {
      await project.execute({
        type: "update_fields",
        ids: [c.id],
        values: [{ field: "inspect", value: inspect }],
      });
    }
  }

  /**
   * Unit of the characteristic's sheet (D-20): Rust reads a newly typed leg in it (T2.9a). Rust
   * says which sheet that is (MF F8); asked again after every change, as a balloon can move.
   */
  let sheetUnit = $state<Unit | null>(null);
  let unitSeq = 0;
  $effect(() => {
    const shown = c;
    void project.revision;
    const seq = ++unitSeq;
    if (shown === null) {
      sheetUnit = null;
      return;
    }
    void tolerance.legUnit(shown.id).then((result) => {
      if (seq === unitSeq) {
        sheetUnit = result.status === "ok" ? result.data : null;
      }
    });
  });
  /** A stored leg in another unit than the sheet's, shown beside the field instead of in it. */
  const legElsewhere = $derived(
    c?.shorter_leg && c.shorter_leg.unit !== sheetUnit ? c.shorter_leg : null,
  );
  const legRefused = $derived(c !== null && tolerance.legError?.id === c.id);
  const legOutcome = $derived(
    c !== null && tolerance.legOutcome?.id === c.id ? tolerance.legOutcome.outcome : null,
  );

  async function setLeg(text: string) {
    if (c !== null) {
      await tolerance.setShorterLeg(c.id, text);
    }
  }

  const done = $derived(tolerance.lastReinterpretation);

  /** Values shown read only above the limits, as Rust stores them (rule 5); edited in the table. */
  const VALUE_FIELDS: readonly ColumnId[] = ["nominal", "upper_dev", "lower_dev", "fit"];

  function columnLabel(id: ColumnId): string {
    return COLUMNS.find((col) => col.id === id)?.label() ?? id;
  }
  const UpdateIcon = icons.updateLimits;

  const gridClass =
    "grid grid-cols-[minmax(5.5rem,auto)_minmax(0,1fr)] items-center gap-x-2 gap-y-1.5";
  const keyClass = "text-sm text-text-muted";
  const inputClass = fieldClass("w-full min-w-0 text-sm");
</script>

{#if c}
  {@const id = c.id}
  <div class="flex flex-col" data-testid="characteristic-properties" data-characteristic={id}>
    <PaneSection id="characteristic.requirement" title={m.pane_section_requirement()}>
      {#key id}
        <div class={gridClass}>
          <label class={keyClass} for="{uid}-requirement-text">{m.pane_text()}</label>
          <input
            id="{uid}-requirement-text"
            type="text"
            autocomplete="off"
            spellcheck="false"
            class={inputClass}
            data-field="requirement_text"
            value={c.requirement_text}
            onchange={(e) => void edit(e.currentTarget, id, "requirement_text")}
          />
          <label class={keyClass} for="{uid}-kind">{m.col_kind()}</label>
          <select
            id="{uid}-kind"
            class={inputClass}
            data-field="kind"
            value={c.kind}
            onchange={(e) => void edit(e.currentTarget, id, "kind")}
          >
            {#each KINDS as kind (kind)}
              <option value={kind}>{kindLabel(kind)}</option>
            {/each}
          </select>
          <label class={keyClass} for="{uid}-quantity">{m.col_quantity()}</label>
          <input
            id="{uid}-quantity"
            type="text"
            inputmode="numeric"
            autocomplete="off"
            class="{inputClass} max-w-24 text-right font-mono"
            data-field="quantity"
            value={String(c.quantity)}
            onchange={(e) => void edit(e.currentTarget, id, "quantity")}
          />
        </div>
      {/key}
    </PaneSection>

    <PaneSection id="characteristic.tolerance" title={m.pane_section_tolerance()}>
      <dl class={gridClass} data-testid="detail-tolerance">
        <dt class={keyClass}>{m.detail_rule()}</dt>
        <dd class="m-0 flex flex-wrap items-center gap-1 text-sm" data-field="rule">
          <span>{c.derivation ? ruleLabel(c.derivation.rule) : ruleLabel(null)}</span>
          <ToleranceBadges derivation={c.derivation} />
        </dd>
        {#each VALUE_FIELDS as field (field)}
          {#if displayText(c, field) !== ""}
            <dt class={keyClass}>{columnLabel(field)}</dt>
            <dd class="m-0 font-mono text-sm" data-field={field}>{displayText(c, field)}</dd>
          {/if}
        {/each}
        {#if c.upper_limit !== null || c.lower_limit !== null}
          <dt class={keyClass}>{m.detail_limits()}</dt>
          <dd class="m-0 font-mono text-sm" data-field="limits">
            {c.lower_limit ?? "–"} … {c.upper_limit ?? "–"}
            {c.unit === null ? "" : unitLabel(c.unit)}
          </dd>
        {/if}
      </dl>
      {#if c.kind === "angle"}
        <label class="flex flex-wrap items-center gap-2 text-sm">
          <span class="text-text-muted">
            {m.shorter_leg_label({ unit: unitLabel(sheetUnit) })}
          </span>
          <input
            type="text"
            inputmode="decimal"
            autocomplete="off"
            spellcheck="false"
            class="min-h-8 w-24 rounded-md border border-border-strong bg-surface px-2 py-1 text-right font-mono text-sm text-text"
            data-field="shorter_leg"
            title={m.shorter_leg_hint()}
            aria-invalid={legRefused ? "true" : undefined}
            value={legElsewhere === null ? (c.shorter_leg?.value ?? "") : ""}
            onchange={(e) => void setLeg(e.currentTarget.value)}
          />
        </label>
        {#if legElsewhere}
          <p class="m-0 text-xs text-text-muted" data-field="shorter_leg_stored">
            {m.shorter_leg_stored({
              value: legElsewhere.value,
              unit: unitLabel(legElsewhere.unit),
            })}
          </p>
        {/if}
        {#if legRefused}
          <p class="m-0 text-xs text-danger" role="alert">
            <span aria-hidden="true">⚠</span>
            {m.shorter_leg_invalid()}
          </p>
        {:else if legOutcome === "manual_kept"}
          <p class="m-0 text-xs" role="status">
            <span aria-hidden="true">◇</span>
            {m.shorter_leg_manual_kept()}
          </p>
        {:else if legOutcome === "unreadable"}
          <p class="m-0 text-xs" role="status">
            <span aria-hidden="true">◇</span>
            {m.shorter_leg_unreadable()}
          </p>
        {/if}
      {/if}
      <div>
        <h4 class="m-0 mb-1 text-xs font-normal text-text-muted">{m.detail_explanation()}</h4>
        <p
          class="m-0 rounded-lg bg-bg px-2.5 py-2 text-sm leading-normal whitespace-pre-line"
          data-field="explanation"
          aria-live="polite"
        >
          {#if explanation.state === "loading"}
            {m.detail_loading()}
          {:else if explanation.state === "error"}
            <span class="text-danger">{m.detail_failed({ message: explanation.text })}</span>
          {:else}
            {explanation.text ?? m.detail_no_rule()}
          {/if}
        </p>
      </div>
      <button
        type="button"
        class="flex min-h-8 items-center gap-1.5 self-start rounded-md border border-border-strong px-2.5 text-sm text-text hover:bg-hover disabled:opacity-45"
        title={m.cmd_update_limits_desc()}
        disabled={tolerance.busy}
        data-action="update_limits"
        onclick={() => void tolerance.reinterpret(selected.ids)}
      >
        <UpdateIcon size={16} strokeWidth={1.75} aria-hidden="true" class="text-text-muted" />
        {m.cmd_update_limits()}
      </button>
      {#if done}
        <p class="m-0 text-xs" role="status">
          {m.detail_reinterpret_done({
            count: String(done.reinterpreted),
            manual: String(done.skipped_manual),
            unreadable: String(done.skipped_unreadable),
          })}
        </p>
      {/if}
      {#if tolerance.error}
        <p class="m-0 text-xs text-danger" role="alert">{commandErrorMessage(tolerance.error)}</p>
      {/if}
    </PaneSection>

    <PaneSection id="characteristic.inspection" title={m.pane_section_inspection()}>
      {#key id}
        <div class={gridClass}>
          <span class={keyClass} id="{uid}-inspect-label">{m.pane_inspect()}</span>
          <label class="flex min-h-8 items-center gap-1.5 text-sm">
            <input
              type="checkbox"
              class="size-4"
              checked={c.inspect}
              aria-describedby="{uid}-inspect-label"
              onchange={(e) => void setInspect(e.currentTarget.checked)}
            />
            {m.detail_inspect()}
          </label>
          <label class={keyClass} for="{uid}-result-type">{m.field_result_type()}</label>
          <select
            id="{uid}-result-type"
            class={inputClass}
            data-field="result_type"
            value={c.result_type}
            onchange={(e) => void setResultType(e.currentTarget)}
          >
            {#each ["auto", "variable", "attribute"] as const as type (type)}
              <option value={type}>{resultTypeLabel(type)}</option>
            {/each}
          </select>
          <label class={keyClass} for="{uid}-method">{m.col_inspection_method()}</label>
          <input
            id="{uid}-method"
            type="text"
            autocomplete="off"
            class={inputClass}
            data-field="inspection_method"
            value={c.inspection.method}
            onchange={(e) => void edit(e.currentTarget, id, "inspection_method")}
          />
          <label class={keyClass} for="{uid}-gauge">{m.col_gauge()}</label>
          <input
            id="{uid}-gauge"
            type="text"
            autocomplete="off"
            class={inputClass}
            data-field="gauge"
            value={c.inspection.gauge}
            onchange={(e) => void edit(e.currentTarget, id, "gauge")}
          />
        </div>
      {/key}
    </PaneSection>

    <PaneSection id={HISTORY} title={m.pane_section_history()}>
      <div class="text-sm" data-testid="detail-history">
        {#if history.state === "loading"}
          <p class="m-0">{m.detail_loading()}</p>
        {:else if history.state === "error"}
          <p class="m-0 text-danger">{m.detail_failed({ message: history.text })}</p>
        {:else if history.entries.length === 0}
          <p class="m-0 text-text-muted">{m.history_empty()}</p>
        {:else}
          <ol class="m-0 flex list-none flex-col gap-1.5 p-0">
            {#each history.entries as entry, i (i)}
              <li class="border-l-2 border-border pl-2" data-source={entry.source}>
                <div class="flex flex-wrap gap-x-2 text-text-muted">
                  <time datetime={entry.timestamp}>{when(entry.timestamp)}</time>
                  <span>{entry.user}</span>
                </div>
                <div class="flex flex-wrap items-center gap-x-1.5">
                  <span class="font-semibold">{changeSourceLabel(entry.source)}</span>
                  {#if entry.action === "undo"}
                    <span>({m.history_action_undo()})</span>
                  {:else if entry.action === "redo"}
                    <span>({m.history_action_redo()})</span>
                  {/if}
                </div>
                <div>{what(entry)}</div>
              </li>
            {/each}
          </ol>
        {/if}
      </div>
    </PaneSection>
  </div>
{/if}
