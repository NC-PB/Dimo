<script lang="ts">
  import DrawingToolbar from "$lib/components/DrawingToolbar.svelte";
  import ProjectMessages from "$lib/components/ProjectMessages.svelte";
  import Shortcuts from "$lib/components/Shortcuts.svelte";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import TablePanel from "$lib/table/TablePanel.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { onMount } from "svelte";

  let shortcutsOpen = $state(false);

  onMount(() => {
    void appInfo.load();
    let stop: (() => void) | undefined;
    let unmounted = false;
    void (async () => {
      try {
        // Listen to the project events first, then fetch the state, so nothing is missed.
        stop = await projectStore.connect();
      } catch {
        return; // Outside Tauri, for example in a plain browser.
      }
      if (unmounted) {
        stop();
        return;
      }
      if (DEV_TOOLS_ENABLED) {
        await devTools.startup(projectStore, documentStore, viewport);
      }
    })();
    return () => {
      unmounted = true;
      stop?.();
    };
  });
</script>

<div class="flex h-full flex-col">
  <Toolbar />
  <div class="flex min-h-0 flex-1">
    <div class="flex min-w-0 flex-1 flex-col">
      <DrawingToolbar
        onShowShortcuts={() => {
          shortcutsOpen = true;
        }}
      />
      <ProjectMessages />
      <Viewport />
      <TablePanel />
    </div>
    <SidePanel />
  </div>
</div>
<Shortcuts bind:open={shortcutsOpen} />
<UnsavedChangesDialog />
