<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import { m } from "$lib/i18n";
  import type { Sample, SampleId } from "$lib/ipc/bindings";
  import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
  import { refusalDetail } from "$lib/table/refusal";

  interface Props {
    /** The project; the app's store unless a test passes its own. */
    project?: ProjectStore;
    /** Sample of the active grid column. */
    activeSample: SampleId | null;
  }

  let { project = projectStore, activeSample }: Props = $props();

  /**
   * Fields of the active sample (FR-MEA-01, M3 decision 6): serial, date, inspector, gauges,
   * comment. Each change is one `update_sample`; Rust trims the serial, checks that it is unique
   * and that the date is a calendar date, and the field then shows what the project holds.
   */
  const sample = $derived(project.project?.samples.find((s) => s.id === activeSample));
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
    const next: Sample = { ...current, [field]: input.value };
    const patch = await project.execute({ type: "update_sample", sample: next });
    if (patch === undefined) {
      const error = project.error;
      project.dismissError();
      notice = error === null ? null : refusalDetail(error);
    } else {
      notice = null;
    }
    input.value = project.project?.samples.find((s) => s.id === current.id)?.[field] ?? "";
  }

  const fieldClass = "w-full rounded border border-border bg-surface px-2 py-1 text-sm text-text";
  const labelClass = "flex flex-col gap-0.5 text-xs text-text-muted";
</script>

<section class="flex flex-col gap-2" aria-label={m.sample_panel_title()}>
  <h2 class="text-sm font-semibold text-text">{m.sample_panel_title()}</h2>
  {#if sample}
    {#key sample.id}
      <label class={labelClass}>
        {m.sample_serial()}
        <input
          type="text"
          class={fieldClass}
          autocomplete="off"
          value={sample.serial}
          onchange={(e) => void edit(e, "serial")}
        />
      </label>
      <label class={labelClass}>
        {m.sample_date()}
        <input
          type="date"
          class={fieldClass}
          value={sample.date}
          onchange={(e) => void edit(e, "date")}
        />
      </label>
      {#each TEXT_FIELDS as { field, label } (field)}
        <label class={labelClass}>
          {label()}
          <input
            type="text"
            class={fieldClass}
            autocomplete="off"
            value={sample[field]}
            onchange={(e) => void edit(e, field)}
          />
        </label>
      {/each}
    {/key}
    {#if notice !== null}
      <p class="text-xs text-danger" role="alert">{notice}</p>
    {/if}
  {:else}
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
  {/if}
</section>
