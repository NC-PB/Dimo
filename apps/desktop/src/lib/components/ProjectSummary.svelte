<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import { m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import PaneSection from "$lib/pane/PaneSection.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import { view } from "$lib/stores/view.svelte";

  /**
   * Nothing selected (T3.18): a read only summary of the project (part, drawing, general
   * tolerance, counts) with the way to File > Info, where these values are edited (M3 decision
   * 21: one home per setting), and the empty state of T3.17 with the next step.
   */
  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    project?: ProjectStore;
    tolerance?: ToleranceStore;
  }

  let { project = appProject, tolerance = appTolerance }: Props = $props();

  const p = $derived(project.project);
  const revision = $derived(p?.revisions.find((r) => r.id === p.current_revision));

  const general = $derived.by(() => {
    const choice = tolerance.settings?.general ?? null;
    if (choice === null) {
      return m.tolerance_none();
    }
    const title = tolerance.tables.find((t) => t.table.id === choice.table.id)?.title;
    return `${title ?? choice.table.id} ${choice.class}`;
  });

  const rows = $derived(
    p && revision
      ? [
          { key: m.field_part_number(), value: p.info.part_number, field: "part_number" },
          { key: m.field_part_name(), value: p.info.part_name, field: "part_name" },
          { key: m.field_customer(), value: p.info.customer, field: "customer" },
          { key: m.field_drawing_file(), value: revision.file_name, field: "drawing_file" },
          { key: m.field_revision_label(), value: revision.label, field: "revision" },
          { key: m.tolerance_general(), value: general, field: "general_tolerance" },
          {
            key: m.info_characteristics(),
            value: String(p.characteristics.length),
            field: "characteristics",
          },
          { key: m.pane_sheets(), value: String(revision.sheets.length), field: "sheets" },
        ]
      : [],
  );

  const linkClass =
    "min-h-8 self-start rounded-md px-1 text-[13px] font-medium text-accent-strong underline-offset-2 hover:underline";
</script>

<div class="flex flex-col" data-testid="project-properties">
  <PaneSection id="project.summary" title={m.pane_section_project()}>
    <dl class="m-0 grid grid-cols-[minmax(5.5rem,auto)_minmax(0,1fr)] gap-x-2 gap-y-1 text-[13px]">
      {#each rows as row (row.field)}
        <dt class="text-text-muted">{row.key}</dt>
        <dd class="m-0 break-words" data-field={row.field}>
          {#if row.value === ""}
            <span class="text-text-muted italic">{m.pane_not_set()}</span>
          {:else}
            {row.value}
          {/if}
        </dd>
      {/each}
    </dl>
    <button
      type="button"
      class={linkClass}
      onclick={() => {
        view.openFile("info");
      }}>{m.pane_edit_info()}</button
    >
  </PaneSection>
  <PaneSection id="project.selection" title={m.pane_section_selection()}>
    <EmptyState
      size="inline"
      icon={icons.selectHint}
      title={m.empty_selection_title()}
      text={project.characteristics.length === 0
        ? m.empty_selection_none()
        : m.empty_selection_text()}
      actions={[{ command: "place_tool" }]}
    />
  </PaneSection>
</div>
