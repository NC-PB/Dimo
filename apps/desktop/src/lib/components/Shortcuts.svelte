<script lang="ts">
  import { Dialog } from "bits-ui";
  import { m, shortcutLabel } from "$lib/i18n";
  import { SHORTCUTS, formatCombo, matchShortcut, type ShortcutAction } from "$lib/shortcuts";
  import { documentStore } from "$lib/stores/document.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { selection } from "$lib/stores/selection.svelte";
  import { unsavedPrompt } from "$lib/stores/prompt.svelte";
  import { rotateShownSheet } from "$lib/sheet-properties";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";

  interface Props {
    /** Whether the cheat sheet is open. */
    open?: boolean;
  }

  let { open = $bindable(false) }: Props = $props();

  /** Runs a shortcut. Returns false if it did nothing, so the key keeps its usual meaning. */
  function run(action: ShortcutAction): boolean {
    switch (action) {
      case "new_project":
        void projectStore.newProject();
        break;
      case "open_project":
        void projectStore.open();
        break;
      case "save_project":
        void projectStore.save();
        break;
      case "save_project_as":
        void projectStore.saveAs();
        break;
      case "undo":
        void projectStore.undo();
        break;
      case "redo":
        void projectStore.redo();
        break;
      case "zoom_in":
        viewport.zoomIn();
        break;
      case "zoom_out":
        viewport.zoomOut();
        break;
      case "fit":
        viewport.fit();
        break;
      case "rotate_left":
        void rotateShownSheet(-1);
        break;
      case "rotate_right":
        void rotateShownSheet(1);
        break;
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
      case "select_tool":
        balloonTools.setTool("select");
        break;
      case "place_tool":
        balloonTools.setTool("place");
        break;
      case "edit_value":
        return balloonTools.editPrimary();
      case "select_all":
        balloonTools.selectAll();
        break;
      case "delete_selection":
        void balloonTools.deleteSelection();
        break;
      case "restyle":
        balloonTools.styleOpen = !selection.isEmpty;
        break;
      case "cancel":
        return balloonGestures.cancel() || balloonTools.escape();
      case "show_shortcuts":
        open = !open;
        break;
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
    if (open && shortcut.action !== "show_shortcuts") {
      return; // The dialog has focus; Escape closes it.
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
    if (run(shortcut.action)) {
      event.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-40 bg-black/40" />
    <Dialog.Content
      class="fixed top-1/2 left-1/2 z-50 w-96 max-w-[90vw] -translate-x-1/2 -translate-y-1/2 rounded border border-border bg-surface p-4 text-text shadow-lg"
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
          {#each SHORTCUTS as shortcut (shortcut.action)}
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
        <Dialog.Close class="rounded bg-accent px-3 py-1 text-sm text-accent-text">
          {m.close()}
        </Dialog.Close>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
