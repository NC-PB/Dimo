<script lang="ts">
  import { m } from "$lib/i18n";
  import type { Patch, Scale, Sheet } from "$lib/ipc/bindings";
  import { parseScalePart, setShownScalePart, shownSheet } from "$lib/sheet-properties";

  /**
   * A scale that is not in the list of the Drawing tab (FR-DOC-05, D-20), typed as drawing : part
   * in the dialog that "Custom" of the scale field opens (T3.18). Each changed part is one
   * undoable `update_sheet` of the sheet on screen; Rust checks the value again.
   */
  interface Props {
    /** The sheet and the update; the shown sheet of the app by default, fakes in tests. */
    sheet?: () => Sheet | undefined;
    setPart?: (part: keyof Scale, value: number) => Promise<Patch | undefined>;
  }

  let { sheet: current = shownSheet, setPart = setShownScalePart }: Props = $props();

  const sheet = $derived(current());
  let invalid = $state(false);

  function onPart(event: Event & { currentTarget: HTMLInputElement }, part: keyof Scale): void {
    if (!sheet) {
      return;
    }
    const input = event.currentTarget;
    const value = parseScalePart(input.value);
    if (value === null) {
      invalid = true;
      input.value = String(sheet.scale[part]);
      return;
    }
    invalid = false;
    if (value !== sheet.scale[part]) {
      void setPart(part, value);
    }
  }

  const fieldClass =
    "min-h-8 w-24 rounded-md border border-border-strong bg-surface px-2 py-1 text-right font-mono text-sm text-text";
  const labelClass = "flex flex-col gap-1 text-[13px] text-text-muted";
</script>

{#if sheet}
  <div class="flex flex-col gap-2 text-sm text-text" data-testid="sheet-scale">
    <div class="flex items-end gap-2">
      <label class={labelClass}>
        {m.sheet_scale_drawing()}
        <input
          type="number"
          min="1"
          step="1"
          class={fieldClass}
          aria-invalid={invalid}
          value={sheet.scale.drawing}
          onchange={(e) => {
            onPart(e, "drawing");
          }}
        />
      </label>
      <span class="pb-1.5" aria-hidden="true">:</span>
      <label class={labelClass}>
        {m.sheet_scale_part()}
        <input
          type="number"
          min="1"
          step="1"
          class={fieldClass}
          aria-invalid={invalid}
          value={sheet.scale.actual}
          onchange={(e) => {
            onPart(e, "actual");
          }}
        />
      </label>
    </div>
    {#if invalid}
      <p class="m-0 text-xs text-danger" role="alert">{m.sheet_scale_invalid()}</p>
    {:else}
      <p class="m-0 text-xs text-text-muted">{m.sheet_scale_hint()}</p>
    {/if}
  </div>
{/if}
