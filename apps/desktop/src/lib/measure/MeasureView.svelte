<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import { m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import Pane from "$lib/pane/Pane.svelte";
  import { measurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import MeasureGrid from "./MeasureGrid.svelte";
  import { measureUi } from "./measure-ui.svelte";
  import SampleBar from "./SampleBar.svelte";
  import SamplePanel from "./SamplePanel.svelte";

  /**
   * The Measure view (ADR 0007, T3.5, T3.19): the parts line, the measurement grid with the
   * Measure bar above it, and a pane with the active part (fields and counts). Project and
   * report details live in File > Info. The parts and row commands are on the Measure ribbon
   * (T3.15), the totals in the status bar.
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
      <EmptyState
        icon={icons.measure}
        title={m.empty_no_project_title()}
        text={m.empty_measure_no_project()}
        actions={[{ command: "new_project" }, { command: "open_project" }]}
      />
    {:else}
      <SampleBar />
      <MeasureGrid />
    {/if}
  </section>
  <!-- The shared pane frame of T3.18, filled with the active part (T3.19). -->
  {#if projectStore.project !== null}
    <SamplePanel activeSample={measureUi.activeSample} />
  {:else}
    <Pane title={m.sample_panel_title()} label={m.measure_side_panel_label()} />
  {/if}
</div>
