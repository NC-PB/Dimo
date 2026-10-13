<script lang="ts">
  import { Dialog } from "bits-ui";
  import { commandForShortcut, runCommand } from "$lib/commands.svelte";
  import { m, shortcutLabel } from "$lib/i18n";
  import {
    SHORTCUTS,
    formatCombo,
    matchShortcut,
    worksInTable,
    type ShortcutAction,
  } from "$lib/shortcuts";
  import { documentStore } from "$lib/stores/document.svelte";
  import { unsavedPrompt } from "$lib/stores/prompt.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";

  /**
   * The window's keyboard shortcuts and the cheat sheet (D-52). Shortcuts that belong to a
   * command of the registry run that command, so key, ribbon button and tooltip do the same
   * thing and share one disabled state (T3.15). The rest (panning, nudging, Escape, tabs) are
   * handled here.
   */

  /** Runs a shortcut. Returns false if it did nothing, so the key keeps its usual meaning. */
  function run(action: ShortcutAction): boolean {
    const id = commandForShortcut(action);
    if (id !== undefined) {
      // A Cmd/Ctrl key of a disabled command still belongs to Dimo, never to the webview.
      const withMod = SHORTCUTS.find((s) => s.action === action)?.keys.every((k) => k.mod);
      return runCommand(id) || withMod === true;
    }
    switch (action) {
      // Arrow keys move the view over the drawing, so the drawing moves the other way.
      case "pan_left":
        viewport.panStep(1, 0);
        break;
      case "pan_right":
        viewport.panStep(-1, 0);
        break;
      case "pan_up":
        viewport.panStep(0, 1);
        break;
      case "pan_down":
        viewport.panStep(0, -1);
        break;
      case "previous_sheet":
        documentStore.previousSheet();
        break;
      case "next_sheet":
        documentStore.nextSheet();
        break;
      case "nudge_left":
        return balloonTools.nudge(-1, 0, viewport.view);
      case "nudge_right":
        return balloonTools.nudge(1, 0, viewport.view);
      case "nudge_up":
        return balloonTools.nudge(0, -1, viewport.view);
      case "nudge_down":
        return balloonTools.nudge(0, 1, viewport.view);
      case "select_all":
        balloonTools.selectAll();
        break;
      case "cancel":
        return balloonGestures.cancel() || balloonTools.escape();
      case "show_drawing":
        view.set("drawing");
        break;
      case "show_review":
        view.set("review");
        break;
      case "show_measure":
        view.set("measure");
        break;
      case "show_export":
        view.set("export");
        break;
      default:
        return false;
    }
    return true;
  }

  /** Text entry and choice controls keep their own keys. */
  function isEditable(target: EventTarget | null): boolean {
    return (
      target instanceof HTMLElement &&
      (target.isContentEditable || target.closest("input, select, textarea") !== null)
    );
  }

  /** Arrow keys belong to the focused control unless the drawing or nothing is focused. */
  function viewportOrNothingFocused(target: EventTarget | null): boolean {
    return (
      !(target instanceof HTMLElement) ||
      target === document.body ||
      target.closest("main") !== null
    );
  }

  /** Whether the characteristic table or the measurement grid (ARIA grids) has the focus. */
  function tableFocused(target: EventTarget | null): boolean {
    return target instanceof HTMLElement && target.closest('[role="grid"]') !== null;
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing) {
      return;
    }
    const shortcut = matchShortcut(event);
    if (!shortcut) {
      return;
    }
    if (unsavedPrompt.open) {
      return; // The question about unsaved changes has focus.
    }
    if (balloonTools.pendingDelete !== null) {
      return; // The question about deleting results has focus.
    }
    if (view.shortcutsOpen && shortcut.action !== "show_shortcuts") {
      return; // The dialog has focus; Escape closes it.
    }
    if (view.current !== "drawing" && shortcut.anyView !== true) {
      return; // Drawing and table keys act only while the drawing is shown.
    }
    if (
      isEditable(event.target) &&
      (shortcut.textEditing === true || shortcut.keys.every((k) => !k.mod))
    ) {
      return;
    }
    if (shortcut.viewportOnly && !viewportOrNothingFocused(event.target)) {
      return;
    }
    if (tableFocused(event.target) && !worksInTable(shortcut)) {
      return; // Single keys in the table type or move there, they never act on the drawing.
    }
    if (run(shortcut.action)) {
      event.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<Dialog.Root bind:open={view.shortcutsOpen}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-40 bg-black/40" />
    <Dialog.Content
      class="fixed top-1/2 left-1/2 z-50 max-h-[85vh] w-96 max-w-[90vw] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded border border-border bg-surface p-4 text-text shadow-lg"
    >
      <Dialog.Title class="mb-3 text-base font-semibold">{m.shortcuts_title()}</Dialog.Title>
      <table class="w-full text-sm">
        <thead class="sr-only">
          <tr>
            <th scope="col">{m.shortcut_column_action()}</th>
            <th scope="col">{m.shortcut_column_keys()}</th>
          </tr>
        </thead>
        <tbody>
          {#each SHORTCUTS as shortcut, i (shortcut.action)}
            {#if shortcut.scope !== undefined && SHORTCUTS[i - 1]?.scope !== shortcut.scope}
              <tr>
                <th colspan="2" scope="colgroup" class="pt-3 pb-1 text-left font-semibold">
                  {shortcut.scope === "grid" ? m.grid_shortcuts_title() : m.table_shortcuts_title()}
                </th>
              </tr>
            {/if}
            <tr class="border-t border-border">
              <td class="py-1 pr-4">{shortcutLabel(shortcut.action)}</td>
              <td class="py-1 text-right">
                {#each shortcut.keys as combo, i (i)}
                  {#if i > 0}<span class="text-text-muted"> / </span>{/if}<kbd
                    class="rounded border border-border bg-surface-raised px-1.5 font-mono text-xs"
                    >{formatCombo(combo)}</kbd
                  >
                {/each}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div class="mt-4 flex justify-end">
        <Dialog.Close class="rounded bg-accent-strong px-3 py-1 text-sm text-accent-ink">
          {m.close()}
        </Dialog.Close>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
