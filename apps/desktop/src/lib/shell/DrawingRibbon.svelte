<script lang="ts">
  import { disabledReason } from "$lib/commands.svelte";
  import { m } from "$lib/i18n";
  import type { Unit } from "$lib/ipc/bindings";
  import {
    SCALE_PRESETS,
    SHEET_UNITS,
    presetFor,
    sameScale,
    scaleText,
    shownSheet,
    updateShownSheet,
  } from "$lib/sheet-properties";
  import { view } from "$lib/stores/view.svelte";
  import { tick } from "svelte";
  import CommandButton from "./CommandButton.svelte";
  import { FIELD_CLASS, LABEL_CLASS, fieldTitle } from "./field";
  import GeneralToleranceField from "./GeneralToleranceField.svelte";
  import Ribbon from "./Ribbon.svelte";

  /**
   * Drawing tab (ADR 0007, mockup "Drawing tab"): sheet, zones and views, tolerances, project.
   * Unit and scale act on the sheet on screen with the same undoable `update_sheet` as the side
   * panel; a scale that is not in the list is typed in the side panel.
   */
  const sheet = $derived(shownSheet());
  const reason = $derived(disabledReason("rotate_left"));
  const CUSTOM = "custom";
  const isPreset = $derived(sheet === undefined || presetFor(sheet.scale) !== undefined);

  function unitLabel(unit: Unit): string {
    return unit === "in" ? m.sheet_unit_in() : m.sheet_unit_mm();
  }

  async function onScale(event: Event & { currentTarget: HTMLSelectElement }): Promise<void> {
    const value = event.currentTarget.value;
    if (value === CUSTOM) {
      // Other scales are typed in the sheet properties of the side panel.
      event.currentTarget.value = sheet ? scaleText(sheet.scale) : "";
      view.set("drawing");
      view.showSidePanel = true;
      await tick();
      document
        .querySelector('[aria-labelledby="sheet-properties-title"]')
        ?.scrollIntoView({ block: "start", behavior: "smooth" });
      return;
    }
    const chosen = SCALE_PRESETS.find((p) => scaleText(p) === value);
    if (chosen && sheet && !sameScale(chosen, sheet.scale)) {
      void updateShownSheet({ scale: chosen });
    }
  }
</script>

{#snippet sheetGroup()}
  <CommandButton id="rotate_left" size="large" />
  <CommandButton id="rotate_right" size="large" />
  <div class="flex flex-col gap-1">
    <label class={LABEL_CLASS} title={fieldTitle(m.field_unit_desc(), reason)}>
      {m.sheet_unit()}
      <select
        class="{FIELD_CLASS} min-w-24"
        data-field="unit"
        disabled={sheet === undefined}
        value={sheet?.unit ?? ""}
        onchange={(e) => {
          const unit = SHEET_UNITS.find((u) => u === e.currentTarget.value);
          if (unit && sheet && unit !== sheet.unit) {
            void updateShownSheet({ unit });
          }
        }}
      >
        {#each SHEET_UNITS as unit (unit)}
          <option value={unit}>{unitLabel(unit)}</option>
        {/each}
      </select>
    </label>
    <label class={LABEL_CLASS} title={fieldTitle(m.field_scale_desc(), reason)}>
      {m.sheet_scale()}
      <select
        class="{FIELD_CLASS} min-w-24"
        data-field="scale"
        disabled={sheet === undefined}
        value={sheet ? scaleText(sheet.scale) : ""}
        onchange={(e) => void onScale(e)}
      >
        {#if sheet && !isPreset}
          <option value={scaleText(sheet.scale)}>{scaleText(sheet.scale)}</option>
        {/if}
        {#each SCALE_PRESETS as option (scaleText(option))}
          <option value={scaleText(option)}>{scaleText(option)}</option>
        {/each}
        <option value={CUSTOM}>{m.sheet_scale_custom()}</option>
      </select>
    </label>
  </div>
{/snippet}

{#snippet zones()}
  <CommandButton id="zone_grid" size="large" />
  <CommandButton id="draw_view" size="large" />
{/snippet}

{#snippet tolerances()}
  <div class="flex flex-col gap-1">
    <GeneralToleranceField />
    <CommandButton id="decimal_rules" />
  </div>
  <CommandButton id="company_tables" size="large" />
{/snippet}

{#snippet project()}
  <CommandButton id="project_info" size="large" />
{/snippet}

<Ribbon
  groups={[
    { id: "sheet", label: m.ribbon_group_sheet(), content: sheetGroup },
    { id: "zones", label: m.ribbon_group_zones(), content: zones },
    { id: "tolerances", label: m.ribbon_group_tolerances(), content: tolerances },
    { id: "project", label: m.ribbon_group_project(), content: project },
  ]}
/>
