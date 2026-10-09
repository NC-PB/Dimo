<script lang="ts">
  import { m, shortcutLabel } from "$lib/i18n";
  import { shortcutKeys, type ShortcutAction } from "$lib/shortcuts";
  import { projectStore } from "$lib/stores/project.svelte";

  /** Tooltip with the shortcut, for example "Save (⌘S)". */
  function hint(action: ShortcutAction): string {
    return `${shortcutLabel(action)} (${shortcutKeys(action)})`;
  }

  const buttonClass =
    "rounded px-2 py-1 text-sm text-text hover:bg-surface-raised disabled:opacity-40 disabled:hover:bg-transparent";
</script>

<div class="flex items-center gap-1" role="toolbar" aria-label={m.project_tools_label()}>
  <button
    type="button"
    class={buttonClass}
    title={hint("new_project")}
    disabled={projectStore.busy}
    onclick={() => void projectStore.newProject()}
  >
    {m.new_project()}
  </button>
  <button
    type="button"
    class={buttonClass}
    title={hint("open_project")}
    disabled={projectStore.busy}
    onclick={() => void projectStore.open()}
  >
    {m.open_project()}
  </button>
  <button
    type="button"
    class={buttonClass}
    title={hint("save_project")}
    disabled={projectStore.busy || !projectStore.isOpen}
    onclick={() => void projectStore.save()}
  >
    {m.save_project()}
  </button>
  <button
    type="button"
    class={buttonClass}
    title={hint("save_project_as")}
    disabled={projectStore.busy || !projectStore.isOpen}
    onclick={() => void projectStore.saveAs()}
  >
    {m.save_project_as()}
  </button>
  <span class="mx-1 h-5 border-l border-border" aria-hidden="true"></span>
  <button
    type="button"
    class={buttonClass}
    title={hint("undo")}
    aria-label={m.undo()}
    disabled={!projectStore.canUndo}
    onclick={() => void projectStore.undo()}>↶</button
  >
  <button
    type="button"
    class={buttonClass}
    title={hint("redo")}
    aria-label={m.redo()}
    disabled={!projectStore.canRedo}
    onclick={() => void projectStore.redo()}>↷</button
  >
</div>
