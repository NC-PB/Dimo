<script lang="ts">
  /**
   * The proposal card of box select (T2.6, FR-REC-01, FR-REC-02, ADR 0006): next to the box the
   * user drew, it shows what Rust read there, with the tolerance engine's explanation and notes. The requirement text and the kind can be edited;
   * edited text is read again by Rust. Enter accepts (one undo step), Esc discards.
   */
  import type { CharacteristicKind } from "$lib/ipc/bindings";
  import { kindLabel, m, noteText, ruleLabel, unitLabel } from "$lib/i18n";
  import type { BoxSelectStore } from "$lib/stores/box-select.svelte";
  import { KINDS } from "$lib/table/columns";
  import { sheetToScreen, type ViewTransform } from "$lib/viewport/view-math";

  interface Props {
    /** The box select store whose card this is. */
    store: BoxSelectStore;
    /** The view transform of the overlay. */
    view: ViewTransform;
    /** Width of the viewport in CSS px, to choose the side of the box. */
    width: number;
    /** Height of the viewport in CSS px, to keep the card inside it. */
    height: number;
    /** Called when the card closes, to give the keyboard focus back to the drawing. */
    onDone: () => void;
  }

  let { store, view, width, height, onDone }: Props = $props();

  /** Space kept between the card and the viewport edge, in CSS px. */
  const EDGE_PX = 4;
  /** Measured height of the card. */
  let cardHeight = $state(0);

  /** Space between the box and the card, in CSS px. */
  const GAP_PX = 10;
  /** Width of the card on screen. */
  const CARD_WIDTH_PX = 280;

  /** The box on screen; sheet rotations are multiples of 90 degrees, so it stays a rectangle. */
  const box = $derived.by(() => {
    const r = store.region;
    if (r === null) {
      return null;
    }
    const a = sheetToScreen(view, { x: r.x, y: r.y });
    const b = sheetToScreen(view, { x: r.x + r.width, y: r.y + r.height });
    return {
      left: Math.min(a.x, b.x),
      top: Math.min(a.y, b.y),
      width: Math.abs(b.x - a.x),
      height: Math.abs(b.y - a.y),
    };
  });

  const at = $derived.by(() => {
    if (box === null) {
      return null;
    }
    const right = box.left + box.width + GAP_PX + CARD_WIDTH_PX <= width;
    return {
      x: right ? box.left + box.width + GAP_PX : Math.max(0, box.left - GAP_PX - CARD_WIDTH_PX),
      // Next to the top of the box, moved up as far as needed to stay inside the viewport.
      y: Math.max(EDGE_PX, Math.min(box.top, height - cardHeight - EDGE_PX)),
    };
  });

  /** The requirement text fields, one per proposal. */
  const inputs = $state<HTMLInputElement[]>([]);
  let focusedFor: unknown = null;

  // A new card puts the cursor into the first requirement text, selected.
  $effect(() => {
    const shown = store.region;
    const first = inputs[0];
    if (shown !== null && shown !== focusedFor && first) {
      focusedFor = shown;
      first.focus({ preventScroll: true });
      first.select();
    }
    if (shown === null) {
      focusedFor = null;
    }
  });

  function value(text: string | null): string {
    return text ?? m.proposal_none();
  }

  async function accept() {
    await store.accept();
    onDone();
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.isComposing) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      void accept();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      store.discard();
      onDone();
    }
  }
</script>

{#if store.open && box && at}
  <div
    class="pointer-events-none absolute z-10 border-2 border-dashed border-accent"
    style:left="{box.left}px"
    style:top="{box.top}px"
    style:width="{box.width}px"
    style:height="{box.height}px"
    aria-hidden="true"
  ></div>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <section
    class="absolute z-20 flex flex-col gap-2 overflow-y-auto rounded border border-border bg-surface p-2 text-sm text-text shadow-lg"
    style:left="{at.x}px"
    style:top="{at.y}px"
    style:width="{CARD_WIDTH_PX}px"
    style:max-height="{Math.max(0, height - 2 * EDGE_PX)}px"
    bind:clientHeight={cardHeight}
    aria-label={m.proposal_card_label()}
    onkeydown={onKeyDown}
  >
    <header class="flex items-baseline justify-between gap-2">
      <h2 class="text-xs font-semibold">{m.proposal_card_label()}</h2>
      {#if store.proposals.length > 1}
        <span class="text-xs text-text-muted">
          {m.proposal_count({ count: String(store.proposals.length) })}
        </span>
      {/if}
    </header>
    {#each store.proposals as proposal, index (index)}
      <div class="flex flex-col gap-1 border-t border-border pt-2 first-of-type:border-t-0">
        <label class="flex flex-col gap-0.5">
          <span class="text-xs text-text-muted">{m.proposal_text()}</span>
          <input
            bind:this={inputs[index]}
            type="text"
            autocomplete="off"
            spellcheck="false"
            class="rounded border border-border bg-surface px-1.5 py-0.5 text-text"
            value={proposal.requirement_text}
            oninput={(e) => {
              store.edit(index, { requirement_text: e.currentTarget.value });
            }}
            onchange={() => void store.reread(index)}
          />
        </label>
        <label class="flex items-center justify-between gap-2">
          <span class="text-xs text-text-muted">{m.proposal_kind()}</span>
          <select
            class="rounded border border-border bg-surface px-1 py-0.5 text-text"
            value={proposal.kind}
            onchange={(e) => {
              store.edit(index, { kind: e.currentTarget.value as CharacteristicKind });
            }}
          >
            {#each KINDS as kind (kind)}
              <option value={kind}>{kindLabel(kind)}</option>
            {/each}
          </select>
        </label>
        {#if proposal.parse_error}
          <p class="text-xs text-text" role="note">
            <span aria-hidden="true">⚠</span>
            {m.proposal_parse_error({
              position: String(proposal.parse_error.position + 1),
              expected: proposal.parse_error.expected,
            })}
          </p>
        {:else}
          <dl class="grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5 text-xs">
            <dt class="text-text-muted">{m.proposal_nominal()}</dt>
            <dd class="tabular-nums" data-field="nominal">
              {value(proposal.nominal)}
              {#if proposal.nominal !== null && proposal.unit !== null}{unitLabel(
                  proposal.unit,
                )}{/if}
              {#if proposal.fit}<span class="ml-1">{proposal.fit}</span>{/if}
            </dd>
            <dt class="text-text-muted">{m.proposal_upper()}</dt>
            <dd class="tabular-nums" data-field="upper_limit">{value(proposal.upper_limit)}</dd>
            <dt class="text-text-muted">{m.proposal_lower()}</dt>
            <dd class="tabular-nums" data-field="lower_limit">{value(proposal.lower_limit)}</dd>
            <dt class="text-text-muted">{m.proposal_rule()}</dt>
            <dd data-field="rule">{ruleLabel(proposal.derivation?.rule ?? null)}</dd>
            {#if store.explanations[index]}
              <dt class="text-text-muted">{m.proposal_explanation()}</dt>
              <dd data-field="explanation">{store.explanations[index]}</dd>
            {/if}
          </dl>
          {#each store.notes[index] ?? [] as note (note)}
            <p class="text-xs" role="note">
              <span aria-hidden="true">⚠</span>
              {noteText(note)}
            </p>
          {/each}
          {#if !proposal.inspect}
            <p class="text-xs" role="note">
              <span aria-hidden="true">◇</span>
              {m.proposal_not_inspected()}
            </p>
          {/if}
        {/if}
      </div>
    {/each}
    <footer class="flex items-center justify-between gap-2 pt-1">
      <span class="text-xs text-text-muted">{m.proposal_keys_hint()}</span>
      <span class="flex gap-1">
        <button
          type="button"
          class="rounded border border-border px-2 py-0.5 text-xs hover:bg-surface-raised"
          onclick={() => {
            store.discard();
            onDone();
          }}
        >
          {m.proposal_discard()}
        </button>
        <button
          type="button"
          class="rounded bg-accent px-2 py-0.5 text-xs text-accent-text"
          onclick={() => void accept()}
        >
          {m.proposal_accept()}
        </button>
      </span>
    </footer>
  </section>
{/if}
