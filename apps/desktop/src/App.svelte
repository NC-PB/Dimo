<script lang="ts">
  import ReportView from "$lib/report/ReportView.svelte";
  import GeneralToleranceOffer from "$lib/components/GeneralToleranceOffer.svelte";
  import ProjectMessages from "$lib/components/ProjectMessages.svelte";
  import Shortcuts from "$lib/components/Shortcuts.svelte";
  import PropertiesPane from "$lib/components/PropertiesPane.svelte";
  import DeleteCharacteristicsDialog from "$lib/components/DeleteCharacteristicsDialog.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import SetupPane from "$lib/components/SetupPane.svelte";
  import StartScreen from "$lib/components/StartScreen.svelte";
  import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import MeasureView from "$lib/measure/MeasureView.svelte";
  import FilePage from "$lib/shell/FilePage.svelte";
  import ProjectSettingsDialog from "$lib/shell/ProjectSettingsDialog.svelte";
  import StatusBar from "$lib/shell/StatusBar.svelte";
  import TabBar from "$lib/shell/TabBar.svelte";
  import TitleBar from "$lib/shell/TitleBar.svelte";
  import TablePanel from "$lib/table/TablePanel.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { exportStore } from "$lib/stores/export.svelte";
  import { numberingStore } from "$lib/stores/numbering.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { setupStore } from "$lib/stores/setup.svelte";
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
    view.paneTool = null;
  });

  // The numbering preview follows the project: every change (also undo and redo) and every
  // other strategy asks Rust again. It is switched on from the Home tab (T3.18).
  $effect(() => {
    void projectStore.revision;
    void projectStore.session;
    void numberingStore.strategy;
    void numberingStore.previewing;
    untrack(() => void numberingStore.refresh());
  });

  // A project opened or created from the File page shows its drawing, so work can start
  // at once (ADR 0007, T3.16). Closing a project stays on the File page, which then offers
  // New and Open.
  $effect(() => {
    void projectStore.session;
    untrack(() => {
      if (projectStore.isOpen && view.tab === "file") {
        view.setTab("home");
      }
    });
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
       scrolling are kept. Hidden panes stay mounted for the same reason. Without a project the
       start screen covers it (T3.17); the drawing keeps its size underneath. -->
  <div
    class="relative flex min-h-0 flex-1"
    style:display={view.current === "drawing" ? null : "none"}
  >
    {#if !projectStore.isOpen}
      <div class="absolute inset-0 z-40 flex">
        <StartScreen />
      </div>
    {/if}
    <div class="relative flex min-w-0 flex-1 flex-col" inert={!projectStore.isOpen}>
      <GeneralToleranceOffer
        class="absolute top-14 left-1/2 z-30 w-[min(40rem,calc(100%-2rem))] -translate-x-1/2 shadow-lg"
        confirmation={false}
      />
      <Viewport />
      <div class="contents" style:display={view.showTable ? null : "none"}>
        <TablePanel />
      </div>
    </div>
    <!-- The setup pane of a new project takes the place of the properties pane (decision 23). -->
    <SetupPane sheetCount={projectStore.sheets.length} />
    <div
      class="contents"
      style:display={view.showSidePanel && !setupStore.visible ? null : "none"}
      inert={!projectStore.isOpen}
    >
      <PropertiesPane />
    </div>
  </div>
  {#if view.current === "measure"}
    <MeasureView />
  {:else if view.current === "export"}
    <ReportView />
  {:else if view.current === "file"}
    <FilePage />
  {:else if view.current === "review"}
    <section class="flex flex-1 overflow-auto bg-bg" aria-label={m.tab_review()}>
      <EmptyState
        icon={icons.review}
        title={m.review_title()}
        text={m.review_text()}
        actions={projectStore.isOpen
          ? [{ command: "place_tool" }]
          : [{ command: "new_project" }, { command: "open_project" }]}
      />
    </section>
  {/if}
  <StatusBar />
</div>
<Shortcuts />
<UnsavedChangesDialog />
<DeleteCharacteristicsDialog />
<ProjectSettingsDialog />
