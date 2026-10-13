<script lang="ts">
  import { m } from "$lib/i18n";
  import type { Scale, TableClass, ToleranceTableInfo, Unit } from "$lib/ipc/bindings";
  import { SCALE_PRESETS, SHEET_UNITS, presetFor, scaleText } from "$lib/sheet-properties";
  import { setupStore, type SetupAnswers, type SetupStore } from "$lib/stores/setup.svelte";
  import { toleranceStore } from "$lib/stores/tolerance.svelte";

  /**
   * "Set up this drawing" (M3 decision 23, T3.17, mockup "Setup"): the right pane after a new
   * project. It asks for the general tolerance as large choices (the classes come from Rust's
   * table list, never from a fixed list here), unit and scale, and part number and customer.
   * Start ballooning sends the answers through the setup store; Skip closes it unchanged.
   */
  interface Props {
    setup?: SetupStore;
    /** The tables Rust lists; App keeps the list in step with the project. */
    tables?: { readonly generalTables: readonly ToleranceTableInfo[] };
    /** Number of sheets of the drawing, for the "all sheets" hint. */
    sheetCount?: number;
  }

  let { setup = setupStore, tables = toleranceStore, sheetCount = 1 }: Props = $props();

  const NONE = "none";

  interface Choice {
    value: string;
    label: string;
    hint: string;
    choice: TableClass | null;
  }

  /** Descriptions of the ISO 2768-1 classes (f fine, m medium, c coarse, v very coarse). */
  function classHint(table: ToleranceTableInfo, cls: string): string {
    if (!table.table.id.startsWith("iso-2768")) {
      return table.title;
    }
    switch (cls) {
      case "f":
        return m.setup_class_f();
      case "m":
        return m.setup_class_m();
      case "c":
        return m.setup_class_c();
      case "v":
        return m.setup_class_v();
      default:
        return table.title;
    }
  }

  /** Short name of a table: its title up to the first descriptive word, e.g. "ISO 2768-1". */
  function shortName(table: ToleranceTableInfo): string {
    const standard = /^(ISO|DIN|EN)\s[\d-]+/.exec(table.title);
    return standard ? standard[0] : table.title;
  }

  const choices = $derived<Choice[]>([
    { value: NONE, label: m.tolerance_none(), hint: m.setup_none_hint(), choice: null },
    ...tables.generalTables.flatMap((t) =>
      t.classes.map((cls) => ({
        value: `${t.table.id}|${String(t.table.version)}|${cls}`,
        label: `${shortName(t)} ${cls}`,
        hint: classHint(t, cls),
        choice: { table: t.table, class: cls },
      })),
    ),
  ]);
  const anyDraft = $derived(tables.generalTables.some((t) => t.draft));

  function keyOf(choice: TableClass | null): string {
    return choice === null
      ? NONE
      : `${choice.table.id}|${String(choice.table.version)}|${choice.class}`;
  }

  // The form starts with what the project holds and is filled again each time the pane opens.
  let general = $state(NONE);
  let unit = $state<Unit>("mm");
  let scale = $state("1:1");
  let partNumber = $state("");
  let customer = $state("");
  let filledFor = -1;

  $effect(() => {
    const session = setup.shownFor;
    if (session !== -1 && session !== filledFor) {
      const current = setup.current();
      if (current !== null) {
        general = keyOf(current.general);
        unit = current.unit;
        scale = scaleText(current.scale);
        partNumber = current.partNumber;
        customer = current.customer;
      }
    }
    filledFor = session;
  });

  const customScale = $derived.by(() => {
    const current = setup.current()?.scale;
    return current !== undefined && presetFor(current) === undefined ? current : undefined;
  });

  function unitLabel(value: Unit): string {
    return value === "in" ? m.sheet_unit_in() : m.sheet_unit_mm();
  }

  function answers(): SetupAnswers | null {
    const current = setup.current();
    if (current === null) {
      return null;
    }
    const picked = choices.find((c) => c.value === general);
    const chosenScale: Scale =
      SCALE_PRESETS.find((p) => scaleText(p) === scale) ?? customScale ?? current.scale;
    return {
      // A stored choice that is not listed (yet) stays as it is.
      general: picked === undefined ? current.general : picked.choice,
      unit,
      scale: chosenScale,
      partNumber,
      customer,
    };
  }

  function start(): void {
    const a = answers();
    if (a !== null) {
      void setup.apply(a);
    }
  }

  const fieldClass =
    "min-h-8 rounded-md border border-border-strong bg-surface px-2 text-[13px] text-text";
  const stepNumber =
    "flex size-6 shrink-0 items-center justify-center rounded-full bg-accent-strong font-heading text-xs font-bold text-accent-ink";
</script>

{#if setup.visible}
  <aside
    class="flex min-h-0 w-[22rem] shrink-0 flex-col border-l border-border bg-surface text-sm text-text"
    aria-labelledby="setup-title"
    data-testid="setup-pane"
  >
    <!-- The questions scroll, the buttons stay in view. -->
    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="border-b border-border px-4 pt-4 pb-3">
        <h2 id="setup-title" class="mb-1 font-heading text-lg font-semibold">{m.setup_title()}</h2>
        <p class="text-[13px] leading-normal text-text-muted">{m.setup_lead()}</p>
      </div>

      <fieldset class="border-b border-border px-4 py-3.5">
        <legend class="contents">
          <span class="mb-2 flex items-center gap-2.5 font-semibold">
            <span class={stepNumber} aria-hidden="true">1</span>
            {m.tolerance_general()}
            {#if anyDraft}
              <span
                class="rounded-full border border-dashed border-warning px-2 text-[11px] font-semibold text-warning"
                title={m.badge_draft_title()}>{m.badge_draft()}</span
              >
            {/if}
          </span>
        </legend>
        <div class="grid grid-cols-2 gap-1.5">
          {#each choices as option (option.value)}
            <label
              class="flex cursor-pointer flex-col gap-0.5 rounded-lg border px-2.5 py-2 {general ===
              option.value
                ? 'border-2 border-accent bg-accent-soft'
                : 'border-border hover:bg-hover'}"
            >
              <span class="flex items-center gap-1.5 text-[13px]">
                <input
                  type="radio"
                  name="setup-general"
                  class="accent-accent"
                  value={option.value}
                  bind:group={general}
                />
                {option.label}
              </span>
              <span class="text-xs text-text-muted">{option.hint}</span>
            </label>
          {/each}
        </div>
        <p class="mt-2 text-xs leading-snug text-text-muted">{m.setup_general_hint()}</p>
      </fieldset>

      <div class="border-b border-border px-4 py-3.5">
        <h3 class="mb-2 flex items-center gap-2.5 font-semibold">
          <span class={stepNumber} aria-hidden="true">2</span>
          {m.setup_sheet_title()}
        </h3>
        <div class="flex flex-wrap gap-2">
          <label class="flex flex-col gap-0.5 text-xs text-text-muted">
            {m.sheet_unit()}
            <select
              class={fieldClass}
              value={unit}
              onchange={(e) => {
                unit = SHEET_UNITS.find((u) => u === e.currentTarget.value) ?? unit;
              }}
            >
              {#each SHEET_UNITS as option (option)}
                <option value={option}>{unitLabel(option)}</option>
              {/each}
            </select>
          </label>
          <label class="flex flex-col gap-0.5 text-xs text-text-muted">
            {m.sheet_scale()}
            <select
              class={fieldClass}
              value={scale}
              onchange={(e) => {
                scale = e.currentTarget.value;
              }}
            >
              {#if customScale}
                <option value={scaleText(customScale)}>{scaleText(customScale)}</option>
              {/if}
              {#each SCALE_PRESETS as option (scaleText(option))}
                <option value={scaleText(option)}>{scaleText(option)}</option>
              {/each}
            </select>
          </label>
        </div>
        {#if sheetCount > 1}
          <p class="mt-2 text-xs leading-snug text-text-muted">
            {m.setup_sheet_all({ count: String(sheetCount) })}
          </p>
        {/if}
      </div>

      <div class="px-4 py-3.5">
        <h3 class="mb-2 flex items-center gap-2.5 font-semibold">
          <span class={stepNumber} aria-hidden="true">3</span>
          {m.setup_part_title()}
        </h3>
        <div class="flex flex-wrap gap-2">
          <label class="flex min-w-0 flex-1 flex-col gap-0.5 text-xs text-text-muted">
            {m.field_part_number()}
            <input class={fieldClass} type="text" bind:value={partNumber} />
          </label>
          <label class="flex min-w-0 flex-1 flex-col gap-0.5 text-xs text-text-muted">
            {m.field_customer()}
            <input class={fieldClass} type="text" bind:value={customer} />
          </label>
        </div>
      </div>
    </div>
    <div class="flex justify-end gap-2 border-t border-border px-4 py-3.5">
      <button
        type="button"
        class="min-h-10 rounded-lg border border-border-strong bg-surface px-4 text-text hover:bg-hover"
        onclick={() => {
          setup.close();
        }}
      >
        {m.setup_skip()}
      </button>
      <button
        type="button"
        class="min-h-10 rounded-lg bg-accent-strong px-4 font-semibold text-accent-ink hover:opacity-90 disabled:opacity-45"
        disabled={setup.busy}
        onclick={start}
      >
        {m.setup_start()}
      </button>
    </div>
  </aside>
{/if}
