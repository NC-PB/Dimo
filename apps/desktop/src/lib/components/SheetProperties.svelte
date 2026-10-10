<script lang="ts">
  import ZonesAndViews from "$lib/components/ZonesAndViews.svelte";
  import { m, shortcutLabel } from "$lib/i18n";
  import type { Scale, Unit } from "$lib/ipc/bindings";
  import { shortcutKeys, type ShortcutAction } from "$lib/shortcuts";
  import {
    SCALE_PRESETS,
    SHEET_UNITS,
    parseScalePart,
    presetFor,
    rotateShownSheet,
    sameScale,
    scaleText,
    shownSheet,
    updateShownSheet,
  } from "$lib/sheet-properties";
  import { viewRotation } from "$lib/viewport/view-math";

  /** Properties of the sheet on screen: rotation, unit and scale (FR-DOC-05, D-20). */
  const sheet = $derived(shownSheet());
  const preset = $derived(sheet ? presetFor(sheet.scale) : undefined);

  // The custom scale inputs show when the user asks for them or the scale is not a preset.
  let customRequested = $state(false);
  const showCustom = $derived(sheet !== undefined && (customRequested || preset === undefined));
  let scaleInvalid = $state(false);
  const CUSTOM = "custom";

  // Another sheet starts with a clean form.
  let formFor = "";
  $effect(() => {
    const id = sheet?.id ?? "";
    if (id !== formFor) {
      formFor = id;
      customRequested = false;
      scaleInvalid = false;
    }
  });

  function hint(action: ShortcutAction): string {
    return `${shortcutLabel(action)} (${shortcutKeys(action)})`;
  }

  function setUnit(unit: Unit): void {
    if (sheet && sheet.unit !== unit) {
      void updateShownSheet({ unit });
    }
  }

  function unitLabel(unit: Unit): string {
    return unit === "in" ? m.sheet_unit_in() : m.sheet_unit_mm();
  }

  function onPresetChange(event: Event & { currentTarget: HTMLSelectElement }): void {
    const value = event.currentTarget.value;
    scaleInvalid = false;
    if (value === CUSTOM) {
      customRequested = true;
      return;
    }
    customRequested = false;
    const chosen = SCALE_PRESETS.find((p) => scaleText(p) === value);
    if (chosen && sheet && !sameScale(chosen, sheet.scale)) {
      void updateShownSheet({ scale: chosen });
    }
  }

  function onPartChange(
    event: Event & { currentTarget: HTMLInputElement },
    part: keyof Scale,
  ): void {
    if (!sheet) {
      return;
    }
    const input = event.currentTarget;
    const value = parseScalePart(input.value);
    if (value === null) {
      scaleInvalid = true;
      input.value = String(sheet.scale[part]);
      return;
    }
    scaleInvalid = false;
    if (value !== sheet.scale[part]) {
      void updateShownSheet({ scale: { ...sheet.scale, [part]: value } });
    }
  }

  const buttonClass =
    "rounded border border-border px-2 py-1 text-sm text-text hover:bg-surface-raised";
  const fieldClass = "rounded border border-border bg-surface px-2 py-1 text-sm text-text";
</script>

{#if sheet}
  <section class="flex flex-col gap-3" aria-labelledby="sheet-properties-title">
    <h2 id="sheet-properties-title" class="text-sm font-semibold text-text">
      {m.sheet_properties_title()}
    </h2>

    <fieldset class="flex flex-col gap-1">
      <legend class="mb-1 text-xs text-text-muted">{m.sheet_rotation()}</legend>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class={buttonClass}
          title={hint("rotate_left")}
          aria-label={m.rotate_left()}
          onclick={() => void rotateShownSheet(-1)}>↺</button
        >
        <output class="w-24 text-center text-sm text-text tabular-nums" aria-live="polite">
          {m.sheet_rotation_value({ degrees: String(viewRotation(sheet.rotation)) })}
        </output>
        <button
          type="button"
          class={buttonClass}
          title={hint("rotate_right")}
          aria-label={m.rotate_right()}
          onclick={() => void rotateShownSheet(1)}>↻</button
        >
      </div>
    </fieldset>

    <fieldset class="flex flex-col gap-1">
      <legend class="mb-1 text-xs text-text-muted">{m.sheet_unit()}</legend>
      <div class="flex gap-3">
        {#each SHEET_UNITS as unit (unit)}
          <label class="flex items-center gap-1.5 text-sm text-text">
            <input
              type="radio"
              name="sheet-unit"
              value={unit}
              checked={sheet.unit === unit}
              onchange={() => {
                setUnit(unit);
              }}
            />
            {unitLabel(unit)}
          </label>
        {/each}
      </div>
      <p class="text-xs text-text-muted">{m.sheet_unit_hint()}</p>
    </fieldset>

    <fieldset class="flex flex-col gap-1">
      <legend class="mb-1 text-xs text-text-muted">{m.sheet_scale()}</legend>
      <select
        class={fieldClass}
        aria-label={m.sheet_scale()}
        value={showCustom ? CUSTOM : scaleText(sheet.scale)}
        onchange={onPresetChange}
      >
        {#each SCALE_PRESETS as option (scaleText(option))}
          <option value={scaleText(option)}>{scaleText(option)}</option>
        {/each}
        <option value={CUSTOM}>{m.sheet_scale_custom()}</option>
      </select>
      {#if showCustom}
        <div class="flex items-center gap-1.5">
          <input
            type="number"
            min="1"
            step="1"
            class="{fieldClass} w-20"
            aria-label={m.sheet_scale_drawing()}
            aria-invalid={scaleInvalid}
            value={sheet.scale.drawing}
            onchange={(e) => {
              onPartChange(e, "drawing");
            }}
          />
          <span aria-hidden="true">:</span>
          <input
            type="number"
            min="1"
            step="1"
            class="{fieldClass} w-20"
            aria-label={m.sheet_scale_part()}
            aria-invalid={scaleInvalid}
            value={sheet.scale.actual}
            onchange={(e) => {
              onPartChange(e, "actual");
            }}
          />
        </div>
      {/if}
      {#if scaleInvalid}
        <p class="text-xs text-danger" role="alert">{m.sheet_scale_invalid()}</p>
      {:else}
        <p class="text-xs text-text-muted">{m.sheet_scale_hint()}</p>
      {/if}
    </fieldset>

    <ZonesAndViews {sheet} />
  </section>
{/if}
