<script lang="ts">
  import ExportView from "$lib/components/ExportView.svelte";
  import GeneralToleranceOffer from "$lib/components/GeneralToleranceOffer.svelte";
  import ProjectMessages from "$lib/components/ProjectMessages.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import Shortcuts from "$lib/components/Shortcuts.svelte";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import DeleteCharacteristicsDialog from "$lib/components/DeleteCharacteristicsDialog.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { m } from "$lib/i18n";
  import MeasureView from "$lib/measure/MeasureView.svelte";
  import FilePage from "$lib/shell/FilePage.svelte";
  import StatusBar from "$lib/shell/StatusBar.svelte";
  import TabBar from "$lib/shell/TabBar.svelte";
  import TitleBar from "$lib/shell/TitleBar.svelte";
  import TablePanel from "$lib/table/TablePanel.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { toleranceStore } from "$lib/stores/tolerance.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { onMount, untrack } from "svelte";
  import { measureUi } from "$lib/measure/measure-ui.svelte";

  // An offer to update characteristics (M3 decision 22) and the row filters of the Measure tab
  // belong to the project they were made for.
  $effect(() => {
    void projectStore.session;
    toleranceStore.declineGeneralUpdate();
    measureUi.onlyMissing = false;
    measureUi.onlyFailed = false;
  });

  // The tolerance table list (ribbon fields, status bar) follows the project: an import, undo or
  // another project. Only these two values are tracked, not every patch.
  $effect(() => {
    void projectStore.session;
    void projectStore.project?.settings.tolerance;
    untrack(() => void toleranceStore.refreshTables());
  });

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

<!-- The office-style shell of ADR 0007: title bar, tabs with one ribbon row, the work area and
     the status bar. -->
<div class="flex h-full flex-col">
  <TitleBar />
  <TabBar />
  <ProjectMessages />
  <!-- The drawing stays mounted while another area is shown, so tiles, zoom and table
       scrolling are kept. Hidden panes stay mounted for the same reason. -->
  <div class="flex min-h-0 flex-1" style:display={view.current === "drawing" ? null : "none"}>
    <div class="relative flex min-w-0 flex-1 flex-col">
      <GeneralToleranceOffer
        class="absolute top-14 left-1/2 z-30 w-[min(40rem,calc(100%-2rem))] -translate-x-1/2 shadow-lg"
        confirmation={false}
      />
      <Viewport />
      <div class="contents" style:display={view.showTable ? null : "none"}>
        <TablePanel />
      </div>
    </div>
    <div class="contents" style:display={view.showSidePanel ? null : "none"}>
      <SidePanel />
    </div>
  </div>
  {#if view.current === "measure"}
    <MeasureView />
  {:else if view.current === "export"}
    <ExportView />
  {:else if view.current === "settings"}
    <SettingsView />
  {:else if view.current === "file"}
    <FilePage />
  {:else if view.current === "review"}
    <section class="flex flex-1 items-center justify-center bg-bg text-sm text-text-muted">
      <p>{m.view_planned()}</p>
    </section>
  {/if}
  <StatusBar />
</div>
<Shortcuts />
<UnsavedChangesDialog />
<DeleteCharacteristicsDialog />
