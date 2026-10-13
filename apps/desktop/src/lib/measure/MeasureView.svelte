<script lang="ts">
  import ProjectDetails from "$lib/components/ProjectDetails.svelte";
  import { m } from "$lib/i18n";
  import { measurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import MeasureGrid from "./MeasureGrid.svelte";
  import { measureUi } from "./measure-ui.svelte";
  import SampleBar from "./SampleBar.svelte";
  import SamplePanel from "./SamplePanel.svelte";

  /**
   * The Measure view (ADR 0007, T3.5): the parts line, the measurement grid, and a side panel
   * with the active part and the project and report details (T3.4). The parts and row commands
   * are on the Measure ribbon (T3.15).
   */

  // Verdicts follow every change of the project: Rust computes them (M3 decision 2). The
  // decision 16 markers are asked for only when a report was issued.
  $effect(() => {
    const project = projectStore.project;
    if (project === null) {
      measurementStore.clear();
    } else {
      void measurementStore.refresh(project.issued_reports.length > 0);
    }
  });
</script>

<div class="flex min-h-0 flex-1">
  <section class="flex min-w-0 flex-1 flex-col bg-bg" aria-label={m.view_measure()}>
    {#if projectStore.project === null}
      <p class="m-auto text-sm text-text-muted">{m.measure_no_project()}</p>
    {:else}
      <SampleBar />
      <MeasureGrid />
    {/if}
  </section>
  <aside
    class="flex min-h-0 w-72 shrink-0 flex-col gap-4 overflow-y-auto border-l border-border bg-surface p-3 text-sm text-text-muted"
    aria-label={m.measure_side_panel_label()}
  >
    {#if projectStore.project !== null}
      <SamplePanel activeSample={measureUi.activeSample} />
    {/if}
    <ProjectDetails />
  </aside>
</div>
