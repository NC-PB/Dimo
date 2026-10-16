<script lang="ts">
  import AppDialog from "$lib/components/AppDialog.svelte";
  import { commandForShortcut, runCommand } from "$lib/commands.svelte";
  import { m, shortcutLabel } from "$lib/i18n";
  import {
    SHORTCUTS,
    formatCombo,
    isMacPlatform,
    matchShortcut,
    type ShortcutAction,
  } from "$lib/shortcuts";
  import { documentStore } from "$lib/stores/document.svelte";
  import { keyboard } from "$lib/stores/keyboard.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { balloonTools } from "$lib/stores/balloon-tools.svelte";
  import { balloonGestures } from "$lib/viewport/gestures.svelte";

  /**
   * The window's keyboard shortcuts and the cheat sheet (D-52). Shortcuts that belong to a
   * command of the registry run that command, so key, ribbon button and tooltip do the same
   * thing and share one disabled state (T3.15). The rest (panning, nudging, Escape, tabs) are
   * handled here. Which shortcut a key triggers, if any, decides `keyboard.shortcutFor`: open
   * dialogs, the scope of the area shown or focused, text fields (MF F8).
   */

  /** Runs a shortcut. Returns false if it did nothing, so the key keeps its usual meaning. */
  function run(action: ShortcutAction): boolean {
    const id = commandForShortcut(action);
    if (id !== undefined) {
      // A Cmd/Ctrl key of a disabled command still belongs to DimoMark, never to the webview.
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

  function onKeyDown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing) {
      return;
    }
    const shortcut = keyboard.shortcutFor(event, event.target);
    if (shortcut !== null && run(shortcut.action)) {
      event.preventDefault();
    }
  }

  /** `?` inside the open cheat sheet closes it again; the dialog keeps every other key. */
  function onSheetKey(event: KeyboardEvent) {
    if (matchShortcut(event, isMacPlatform(), "app")?.action === "show_shortcuts") {
      event.preventDefault();
      view.shortcutsOpen = false;
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<AppDialog
  open={view.shortcutsOpen}
  onClose={() => {
    view.shortcutsOpen = false;
  }}
  title={m.shortcuts_title()}
  closeLabel={m.close()}
  testid="shortcuts-dialog"
  onkeydown={onSheetKey}
>
  <table class="w-full text-sm">
    <thead class="sr-only">
      <tr>
        <th scope="col">{m.shortcut_column_action()}</th>
        <th scope="col">{m.shortcut_column_keys()}</th>
      </tr>
    </thead>
    <tbody>
      {#each SHORTCUTS as shortcut, i (shortcut.action)}
        {@const group =
          shortcut.scope === "table" || shortcut.scope === "grid" ? shortcut.scope : null}
        {#if group !== null && SHORTCUTS[i - 1]?.scope !== group}
          <tr>
            <th colspan="2" scope="colgroup" class="pt-3 pb-1 text-left font-semibold">
              {group === "grid" ? m.grid_shortcuts_title() : m.table_shortcuts_title()}
            </th>
          </tr>
        {/if}
        <tr class="border-t border-border" data-action={shortcut.action}>
          <td class="py-1 pr-4">{shortcutLabel(shortcut.action)}</td>
          <td class="py-1 text-right">
            {#each shortcut.keys as combo, k (k)}
              {#if k > 0}<span class="text-text-muted"> / </span>{/if}<kbd
                class="rounded border border-border bg-surface-raised px-1.5 font-mono text-xs"
                >{formatCombo(combo)}</kbd
              >
            {/each}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</AppDialog>
