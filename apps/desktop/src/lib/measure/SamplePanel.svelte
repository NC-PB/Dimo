<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import { m } from "$lib/i18n";
  import type { SampleId } from "$lib/ipc/bindings";
  import Pane from "$lib/pane/Pane.svelte";
  import PaneSection from "$lib/pane/PaneSection.svelte";
  import { measurementStore, type MeasurementStore } from "$lib/stores/measurement.svelte";
  import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
  import { refusalDetail } from "$lib/table/refusal";
  import VerdictMark from "./VerdictMark.svelte";

  interface Props {
    /** The project; the app's store unless a test passes its own. */
    project?: ProjectStore;
    /** Verdicts from Rust; the app's store unless a test passes its own. */
    measurement?: MeasurementStore;
    /** Sample of the active grid column. */
    activeSample: SampleId | null;
  }

  let { project = projectStore, measurement = measurementStore, activeSample }: Props = $props();

  /**
   * The part pane of the Measure tab (T3.19, mockup "Measure tab"): the fields of the active part
   * (FR-MEA-01, M3 decision 6: serial, date, inspector, gauges, comment) and its counts per
   * verdict from Rust. Each field change is one `update_sample`; Rust trims the serial, checks
   * that it is unique and that the date is a calendar date, and the field then shows what the
   * project holds. It fills the shared pane frame (T3.18): the header names the part, the fields
   * follow, then the counts as a section that folds.
   */
  const uid = $props.id();
  const sample = $derived(project.project?.samples.find((s) => s.id === activeSample));
  const counts = $derived(sample === undefined ? undefined : measurement.sample(sample.id)?.counts);
  let notice = $state<string | null>(null);

  type Field = "serial" | "date" | "inspector" | "gauges" | "comment";
  const TEXT_FIELDS: { field: Exclude<Field, "date">; label: () => string }[] = [
    { field: "inspector", label: () => m.sample_inspector() },
    { field: "gauges", label: () => m.sample_gauges() },
    { field: "comment", label: () => m.sample_comment() },
  ];

  async function edit(event: Event & { currentTarget: HTMLInputElement }, field: Field) {
    const current = sample;
    const input = event.currentTarget;
    if (current === undefined || input.value === current[field]) {
      return;
    }
    // Only this field: Rust applies it to the part as it is then, so a fast edit of another
    // field of the same part is not overwritten with its old value (T3.18, MF F4).
    const patch = await project.execute({
      type: "update_sample",
      sample: current.id,
      values: [{ field, value: input.value }],
    });
    if (patch === undefined) {
      const error = project.error;
      project.dismissError();
      notice = error === null ? null : refusalDetail(error);
    } else {
      notice = null;
    }
    input.value = project.project?.samples.find((s) => s.id === current.id)?.[field] ?? "";
  }

  const fieldClass =
    "min-h-8 w-full min-w-0 rounded-md border border-border-strong bg-surface px-2 py-1 text-sm text-text";
  const pillClass =
    "inline-flex items-center gap-1 rounded-full border border-border px-2.5 py-0.5 text-sm font-semibold";
</script>

<Pane
  title={sample ? m.measure_part_title({ serial: sample.serial }) : m.sample_panel_title()}
  label={m.measure_side_panel_label()}
  testid="measure-pane"
>
  {#if sample}
    {#key sample.id}
      <div
        class="grid grid-cols-[minmax(5rem,auto)_minmax(0,1fr)] items-center gap-x-2 gap-y-1.5 border-b border-border px-3.5 py-2.5 text-sm"
      >
        <label for="{uid}-serial" class="text-text-muted">{m.sample_serial()}</label>
        <input
          id="{uid}-serial"
          type="text"
          class={fieldClass}
          autocomplete="off"
          value={sample.serial}
          onchange={(e) => void edit(e, "serial")}
        />
        <label for="{uid}-date" class="text-text-muted">{m.sample_date()}</label>
        <input
          id="{uid}-date"
          type="date"
          class={fieldClass}
          value={sample.date}
          onchange={(e) => void edit(e, "date")}
        />
        {#each TEXT_FIELDS as { field, label } (field)}
          <label for="{uid}-{field}" class="text-text-muted">{label()}</label>
          <input
            id="{uid}-{field}"
            type="text"
            class={fieldClass}
            autocomplete="off"
            value={sample[field]}
            onchange={(e) => void edit(e, field)}
          />
        {/each}
      </div>
    {/key}
    {#if notice !== null}
      <p class="px-3.5 pt-2 text-xs text-danger" role="alert">{notice}</p>
    {/if}
    {#if counts}
      <!-- Counts of this part from Rust, each with the shape of its verdict (decision 9). -->
      <PaneSection id="measure.counts" title={m.measure_part_counts()}>
        <div class="flex flex-wrap gap-2" data-testid="part-counts">
          <span class="{pillClass} text-success"
            ><VerdictMark verdict="pass" size={12} />{m.measure_part_pass({
              count: String(counts.pass),
            })}</span
          >
          <span class="{pillClass} text-danger"
            ><VerdictMark verdict="fail" size={12} />{m.measure_part_fail({
              count: String(counts.fail),
            })}</span
          >
          {#if counts.not_evaluated > 0}
            <span class="{pillClass} text-text-muted"
              ><VerdictMark verdict="not_evaluated" size={12} />{m.measure_part_not_evaluated({
                count: String(counts.not_evaluated),
              })}</span
            >
          {/if}
          <span class="{pillClass} text-text-muted"
            >{m.measure_part_missing({ count: String(counts.missing) })}</span
          >
        </div>
      </PaneSection>
    {/if}
  {:else}
    <div class="px-3.5 py-2.5">
      {#if (project.project?.samples.length ?? 0) === 0}
        <EmptyState
          size="inline"
          title={m.empty_measure_no_parts_title()}
          text={m.empty_measure_no_parts_text()}
          actions={[{ command: "add_parts" }]}
        />
      {:else}
        <p class="text-xs text-text-muted">{m.sample_panel_empty()}</p>
      {/if}
    </div>
  {/if}
</Pane>
