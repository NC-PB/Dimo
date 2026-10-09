<script lang="ts">
  import DrawingToolbar from "$lib/components/DrawingToolbar.svelte";
  import ExportView from "$lib/components/ExportView.svelte";
  import ProjectMessages from "$lib/components/ProjectMessages.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import Shortcuts from "$lib/components/Shortcuts.svelte";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { m } from "$lib/i18n";
  import TablePanel from "$lib/table/TablePanel.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { onMount } from "svelte";

  let shortcutsOpen = $state(false);

  onMount(() => {
    void appInfo.load();
    const stops: (() => void)[] = [];
    let unmounted = false;
    void (async () => {
      try {
        // Listen to the project events first, then fetch the state, so nothing is missed.
        stops.push(await projectStore.connect());
        stops.push(await exportStore.connect());
      } catch {
        return; // Outside Tauri, for example in a plain browser.
      }
      if (unmounted) {
        stops.forEach((stop) => {
          stop();
        });
        return;
      }
      if (DEV_TOOLS_ENABLED) {
        await devTools.startup(projectStore, documentStore, viewport);
      }
    })();
    return () => {
      unmounted = true;
      stops.forEach((stop) => {
        stop();
      });
    };
  });
</script>

<div class="flex h-full flex-col">
  <Toolbar />
  <ProjectMessages />
  <!-- The drawing stays mounted while another view is shown, so tiles, zoom and table
       scrolling are kept (D-50). -->
  <div class="flex min-h-0 flex-1" style:display={view.current === "drawing" ? null : "none"}>
    <div class="flex min-w-0 flex-1 flex-col">
      <DrawingToolbar
        onShowShortcuts={() => {
          shortcutsOpen = true;
        }}
      />
      <Viewport />
      <TablePanel />
    </div>
    <SidePanel />
  </div>
  {#if view.current === "export"}
    <ExportView />
  {:else if view.current === "settings"}
    <SettingsView />
  {:else if view.current !== "drawing"}
    <section class="flex flex-1 items-center justify-center bg-bg text-sm text-text-muted">
      <p>{m.view_planned()}</p>
    </section>
  {/if}
</div>
<Shortcuts bind:open={shortcutsOpen} />
<UnsavedChangesDialog />
