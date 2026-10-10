<script lang="ts">
  /**
   * Detail panel of the active characteristic (T2.8): its tolerance rule with markers and the
   * explanation Rust builds in the UI language (FR-TOL-08), the inspect flag (FR-CHR-08), the
   * change history (FR-CHR-10), and re-interpretation of the selection after a settings change.
   */
  import {
    changeSourceLabel,
    commandErrorMessage,
    fieldLabel,
    getLocale,
    m,
    ruleLabel,
    unitLabel,
  } from "$lib/i18n";
  import type { HistoryEntry } from "$lib/ipc/bindings";
  import { explainLanguage } from "$lib/stores/box-select.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection as appSelection, type SelectionStore } from "$lib/stores/selection.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import ToleranceBadges from "./ToleranceBadges.svelte";

  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    project?: ProjectStore;
    selected?: SelectionStore;
    tolerance?: ToleranceStore;
  }

  let { project = appProject, selected = appSelection, tolerance = appTolerance }: Props = $props();

  type Tab = "tolerance" | "history";
  let tab = $state<Tab>("tolerance");

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

  $effect(() => {
    const shown = c;
    void project.revision;
    const seq = ++historySeq;
    if (shown === null || tab !== "history") {
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

  function onTabKey(event: KeyboardEvent) {
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      tab = tab === "tolerance" ? "history" : "tolerance";
      document.getElementById(`detail-tab-${tab}`)?.focus();
    }
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

  const done = $derived(tolerance.lastReinterpretation);
</script>

{#if project.isOpen}
  <section class="flex flex-col gap-2 text-text" aria-label={m.detail_label()}>
    {#if c}
      <h2 class="text-sm font-semibold">{m.detail_title({ number: c.number })}</h2>
      <div class="flex gap-1 border-b border-border" role="tablist" aria-label={m.detail_label()}>
        {#each ["tolerance", "history"] as const as id (id)}
          <button
            type="button"
            id="detail-tab-{id}"
            role="tab"
            class="tab -mb-px border-b-2 px-2 py-1 text-xs"
            class:selected={tab === id}
            aria-selected={tab === id}
            aria-controls="detail-panel-{id}"
            tabindex={tab === id ? 0 : -1}
            onclick={() => {
              tab = id;
            }}
            onkeydown={onTabKey}
            >{id === "tolerance" ? m.detail_tab_tolerance() : m.detail_tab_history()}</button
          >
        {/each}
      </div>

      {#if tab === "tolerance"}
        <div
          id="detail-panel-tolerance"
          role="tabpanel"
          aria-labelledby="detail-tab-tolerance"
          class="flex flex-col gap-2"
          data-testid="detail-tolerance"
        >
          <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
            <dt class="text-text-muted">{m.detail_rule()}</dt>
            <dd class="flex flex-wrap items-center gap-1" data-field="rule">
              <span>{c.derivation ? ruleLabel(c.derivation.rule) : ruleLabel(null)}</span>
              <ToleranceBadges derivation={c.derivation} />
            </dd>
            {#if c.upper_limit !== null || c.lower_limit !== null}
              <dt class="text-text-muted">{m.detail_limits()}</dt>
              <dd class="tabular-nums" data-field="limits">
                {c.lower_limit ?? "–"} … {c.upper_limit ?? "–"}
                {c.unit === null ? "" : unitLabel(c.unit)}
              </dd>
            {/if}
          </dl>
          <div class="text-xs">
            <h3 class="text-text-muted">{m.detail_explanation()}</h3>
            <p class="whitespace-pre-line" data-field="explanation" aria-live="polite">
              {#if explanation.state === "loading"}
                {m.detail_loading()}
              {:else if explanation.state === "error"}
                <span class="text-danger">{m.detail_failed({ message: explanation.text })}</span>
              {:else}
                {explanation.text ?? m.detail_no_rule()}
              {/if}
            </p>
          </div>
          <label class="flex items-center gap-1.5 text-xs">
            <input
              type="checkbox"
              checked={c.inspect}
              onchange={(e) => void setInspect(e.currentTarget.checked)}
            />
            {m.detail_inspect()}
          </label>
        </div>
      {:else}
        <div
          id="detail-panel-history"
          role="tabpanel"
          aria-labelledby="detail-tab-history"
          class="text-xs"
          data-testid="detail-history"
        >
          {#if history.state === "loading"}
            <p>{m.detail_loading()}</p>
          {:else if history.state === "error"}
            <p class="text-danger">{m.detail_failed({ message: history.text })}</p>
          {:else if history.entries.length === 0}
            <p class="text-text-muted">{m.history_empty()}</p>
          {:else}
            <ol class="flex flex-col gap-1.5">
              {#each history.entries as entry, i (i)}
                <li class="border-l-2 border-border pl-2" data-source={entry.source}>
                  <div class="flex flex-wrap gap-x-2 text-text-muted">
                    <time datetime={entry.timestamp}>{when(entry.timestamp)}</time>
                    <span>{entry.user}</span>
                  </div>
                  <div class="flex flex-wrap items-center gap-x-1.5">
                    <span class="source font-semibold">{changeSourceLabel(entry.source)}</span>
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
      {/if}
    {/if}

    {#if selected.size > 0}
      <div class="flex flex-col gap-1 border-t border-border pt-2">
        <button
          type="button"
          class="self-start rounded border border-border px-2 py-1 text-xs hover:bg-surface-raised disabled:opacity-50"
          disabled={tolerance.busy}
          aria-describedby="reinterpret-hint"
          onclick={() => void tolerance.reinterpret(selected.ids)}
          >{m.detail_reinterpret({ count: String(selected.size) })}</button
        >
        <p id="reinterpret-hint" class="text-xs text-text-muted">{m.detail_reinterpret_hint()}</p>
        {#if done}
          <p class="text-xs" role="status">
            {m.detail_reinterpret_done({
              count: String(done.reinterpreted),
              manual: String(done.skipped_manual),
              unreadable: String(done.skipped_unreadable),
            })}
          </p>
        {/if}
        {#if tolerance.error}
          <p class="text-xs text-danger" role="alert">{commandErrorMessage(tolerance.error)}</p>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .tab {
    border-color: transparent;
    color: var(--dimo-text-muted);
  }

  /* The selected tab: an underline and bold text, not only a color. */
  .tab.selected {
    border-color: var(--dimo-accent);
    color: var(--dimo-text);
    font-weight: 600;
  }
</style>
