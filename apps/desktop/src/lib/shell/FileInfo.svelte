<script lang="ts">
  /**
   * File > Info (ADR 0007, M3 decision 21, mockup "File view, project info", T3.16): what is
   * saved with the project. Part, the general tolerance and a summary of the other tolerance
   * rules with the way to the Drawing tab, the drawing (file, revision, fingerprint), progress
   * counts and the report header.
   * Edits go through undoable commands (`update_project_info`,
   * `update_report_info`, `update_revision_label`); counts come from Rust (`measurement_state`).
   */
  import type { ProjectInfo, ReportInfo, ReportKind, TableClass } from "$lib/ipc/bindings";
  import { m } from "$lib/i18n";
  import {
    editProjectInfo,
    editReportInfo,
    editRevisionLabel,
    setReportDateToday,
  } from "$lib/project-details";
  import {
    measurementStore as appMeasurement,
    type MeasurementStore,
  } from "$lib/stores/measurement.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import { view } from "$lib/stores/view.svelte";
  import { untrack } from "svelte";
  import GeneralToleranceOffer from "$lib/components/GeneralToleranceOffer.svelte";
  import CommandButton from "./CommandButton.svelte";
  import GeneralToleranceField from "./GeneralToleranceField.svelte";

  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    projectStore?: ProjectStore;
    toleranceStore?: ToleranceStore;
    measurementStore?: MeasurementStore;
  }

  let {
    projectStore = appProject,
    toleranceStore = appTolerance,
    measurementStore = appMeasurement,
  }: Props = $props();

  const uid = $props.id();
  const project = $derived(projectStore.project);
  const revision = $derived(project?.revisions.find((r) => r.id === project.current_revision));
  const tolerance = $derived(toleranceStore.settings);
  const counts = $derived(measurementStore.state);

  // Progress counts follow every change of the project while the page is shown.
  $effect(() => {
    void projectStore.session;
    void projectStore.revision;
    const p = projectStore.project;
    untrack(() => {
      if (p) {
        void measurementStore.refresh(p.issued_reports.length > 0);
      }
    });
  });

  function tableName(choice: TableClass, list: typeof toleranceStore.tables): string {
    const table = list.find((t) => t.table.id === choice.table.id);
    return `${table?.title ?? choice.table.id} ${choice.class}`;
  }

  function reportKind(kind: ReportKind): string {
    return kind === "check_sheet"
      ? m.export_check_sheet_title()
      : m.export_inspection_report_title();
  }

  const toInspect = $derived(project?.characteristics.filter((c) => c.inspect).length ?? 0);
  const complete = $derived(counts?.samples.filter((s) => s.counts.missing === 0).length ?? 0);
  const lastIssued = $derived(project?.issued_reports.at(-1));

  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  async function copyHash(): Promise<void> {
    if (!revision) {
      return;
    }
    try {
      await navigator.clipboard.writeText(revision.sha256);
    } catch {
      return; // The clipboard may be unavailable; the hash can still be selected by hand.
    }
    copied = true;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => {
      copied = false;
    }, 1500);
  }

  const INFO_FIELDS = [
    { field: "part_number", label: () => m.field_part_number() },
    { field: "part_name", label: () => m.field_part_name() },
    { field: "customer", label: () => m.field_customer() },
    { field: "order_reference", label: () => m.field_order_reference() },
  ] as const satisfies readonly { field: keyof ProjectInfo; label: () => string }[];
  const REPORT_FIELDS = [
    { field: "report_number", label: () => m.field_report_number() },
    { field: "company_name", label: () => m.field_company_name() },
    { field: "inspected_by", label: () => m.field_inspected_by() },
    { field: "approved_by", label: () => m.field_approved_by() },
  ] as const satisfies readonly { field: keyof ReportInfo; label: () => string }[];

  const boxClass = "flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 shadow-sm";
  const headingClass = "font-heading text-base font-semibold";
  const gridClass =
    "grid grid-cols-[minmax(7rem,10rem)_minmax(0,1fr)] items-center gap-x-3 gap-y-2";
  const narrowGridClass =
    "grid grid-cols-[minmax(6rem,8rem)_minmax(0,1fr)] items-center gap-x-3 gap-y-2";
  const fieldClass =
    "min-h-8 w-full rounded-md border border-border-strong bg-surface px-2 py-1 text-text";
  const linkClass = "text-sm font-medium text-accent-strong underline-offset-2 hover:underline";
  const statClass =
    "flex items-center justify-between gap-3 border-b border-border py-1.5 last:border-b-0";
</script>

<div class="flex max-w-5xl flex-col gap-5" data-testid="file-info">
  <div>
    <h1 class="font-heading text-3xl font-semibold tracking-tight">{m.info_title()}</h1>
    <p class="mt-1 text-text-muted">{project ? m.info_lead() : m.info_no_project()}</p>
  </div>

  {#if !project || !revision}
    <div class="flex flex-wrap gap-2">
      <CommandButton id="new_project" size="large" />
      <CommandButton id="open_project" size="large" />
    </div>
  {:else}
    <div class="flex flex-wrap items-start gap-5">
      <div class="flex min-w-[min(100%,26rem)] flex-[2_1_26rem] flex-col gap-4">
        <section class={boxClass} aria-labelledby="{uid}-part">
          <h2 id="{uid}-part" class={headingClass}>{m.project_details_part()}</h2>
          <div class={gridClass}>
            {#each INFO_FIELDS as { field, label } (field)}
              <label for="{uid}-{field}" class="text-text-muted">{label()}</label>
              <input
                id="{uid}-{field}"
                type="text"
                class={fieldClass}
                autocomplete="off"
                value={project.info[field]}
                onchange={(e) => void editProjectInfo(projectStore, e.currentTarget, field)}
              />
            {/each}
          </div>
        </section>

        <section class={boxClass} aria-labelledby="{uid}-tolerances">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h2 id="{uid}-tolerances" class={headingClass}>{m.info_tolerances()}</h2>
            <button
              type="button"
              class={linkClass}
              onclick={() => {
                view.setTab("drawing");
              }}>{m.info_change_on_drawing()}</button
            >
          </div>
          <!-- The general tolerance is set here too (M3 decision 21), with the same field and
               offer as on the Home and Drawing tabs. -->
          <GeneralToleranceField tolerance={toleranceStore} />
          <GeneralToleranceOffer tolerance={toleranceStore} />
          <dl class={gridClass} data-testid="info-tolerances">
            <dt class="text-text-muted">{m.tolerance_decimal_rules()}</dt>
            <dd>
              {#if tolerance && tolerance.decimal_rules.length > 0}
                {tolerance.decimal_rules
                  .map((r) =>
                    m.info_decimal_rule({ places: String(r.places), tolerance: r.tolerance }),
                  )
                  .join(", ")}
              {:else}
                {m.tolerance_none()}
              {/if}
            </dd>
            <dt class="text-text-muted">{m.tolerance_custom_tables()}</dt>
            <dd>
              {#if toleranceStore.customTables.length > 0}
                {toleranceStore.customTables.map((t) => t.title).join(", ")}
              {:else}
                {m.tolerance_none()}
              {/if}
            </dd>
            {#if tolerance?.drawing_rule}
              <dt class="text-text-muted">{m.tolerance_drawing_rule()}</dt>
              <dd>{tableName(tolerance.drawing_rule, toleranceStore.customTables)}</dd>
            {/if}
          </dl>
        </section>

        <section class={boxClass} aria-labelledby="{uid}-drawing">
          <h2 id="{uid}-drawing" class={headingClass}>{m.project_details_drawing()}</h2>
          <div class={gridClass}>
            <span class="text-text-muted">{m.field_drawing_file()}</span>
            <span class="break-all" data-testid="info-drawing-file"
              >{revision.sheets.length === 1
                ? m.info_drawing_file_one({ file: revision.file_name })
                : m.info_drawing_file({
                    file: revision.file_name,
                    count: String(revision.sheets.length),
                  })}</span
            >
            <label for="{uid}-revision" class="text-text-muted">{m.field_revision_label()}</label>
            <input
              id="{uid}-revision"
              type="text"
              class={fieldClass}
              autocomplete="off"
              value={revision.label}
              onchange={(e) => void editRevisionLabel(projectStore, e.currentTarget, revision.id)}
            />
            <span class="text-text-muted">{m.field_drawing_sha()}</span>
            <span class="flex min-w-0 flex-wrap items-center gap-2">
              <code class="font-mono text-xs text-text-muted" title={revision.sha256}
                >{revision.sha256.slice(0, 16)}…</code
              >
              <button
                type="button"
                class={linkClass}
                aria-label={m.copy_hash_aria()}
                onclick={() => void copyHash()}
                >{copied ? m.copy_hash_done() : m.copy_hash()}</button
              >
            </span>
          </div>
        </section>
      </div>

      <div class="flex min-w-[min(100%,18rem)] flex-[1_1_18rem] flex-col gap-4">
        <section class={boxClass} aria-labelledby="{uid}-progress">
          <h2 id="{uid}-progress" class={headingClass}>{m.info_progress()}</h2>
          <div class="text-sm" data-testid="info-progress">
            <div class={statClass}>
              <span>{m.info_characteristics()}</span>
              <strong class="font-mono">{project.characteristics.length}</strong>
            </div>
            <div class={statClass}>
              <span>{m.info_to_inspect()}</span>
              <strong class="font-mono">{toInspect}</strong>
            </div>
            <div class={statClass}>
              <span>{m.info_parts_measured()}</span>
              <strong class="font-mono"
                >{m.info_of({
                  done: String(complete),
                  total: String(counts?.samples.length ?? 0),
                })}</strong
              >
            </div>
            <div class={statClass}>
              <span>{m.info_result()}</span>
              {#if counts === null || counts.samples.length === 0}
                <strong class="text-text-muted">{m.info_none()}</strong>
              {:else if counts.verdict === "fail"}
                <strong class="text-danger"
                  ><span aria-hidden="true">✕</span>
                  {m.info_result_fail({ count: String(counts.counts.fail) })}</strong
                >
              {:else if counts.verdict === "pass"}
                <strong class="text-success"
                  ><span aria-hidden="true">✓</span> {m.info_result_pass()}</strong
                >
              {:else}
                <strong class="text-text-muted"
                  ><span aria-hidden="true">○</span> {m.info_result_open()}</strong
                >
              {/if}
            </div>
            <div class={statClass}>
              <span>{m.info_last_issued()}</span>
              <strong class="text-right">
                {#if lastIssued}
                  {reportKind(lastIssued.kind)}, {lastIssued.exported_at.slice(0, 10)}
                {:else}
                  {m.info_none()}
                {/if}
              </strong>
            </div>
          </div>
        </section>

        <section class={boxClass} aria-labelledby="{uid}-report">
          <h2 id="{uid}-report" class={headingClass}>{m.project_details_report()}</h2>
          <div class={narrowGridClass}>
            {#each REPORT_FIELDS as { field, label } (field)}
              <label for="{uid}-{field}" class="text-text-muted">{label()}</label>
              <input
                id="{uid}-{field}"
                type="text"
                class={fieldClass}
                autocomplete="off"
                value={project.report_info[field]}
                onchange={(e) => void editReportInfo(projectStore, e.currentTarget, field)}
              />
            {/each}
            <label for="{uid}-report-date" class="text-text-muted">{m.field_report_date()}</label>
            <span class="flex flex-wrap items-center gap-2">
              <input
                id="{uid}-report-date"
                type="date"
                class="{fieldClass} min-w-36 flex-1"
                value={project.report_info.report_date}
                onchange={(e) => void editReportInfo(projectStore, e.currentTarget, "report_date")}
              />
              <button
                type="button"
                class="min-h-8 rounded-md border border-border px-2 text-sm hover:bg-hover"
                aria-label={m.report_date_today_aria()}
                onclick={() => {
                  setReportDateToday(projectStore);
                }}>{m.report_date_today()}</button
              >
            </span>
            <!-- Remarks print in the report header; File > Info is their home since the side
                 panel lost the project details (T3.18). -->
            <label for="{uid}-remarks" class="self-start pt-1 text-text-muted"
              >{m.field_remarks()}</label
            >
            <textarea
              id="{uid}-remarks"
              class={fieldClass}
              rows="3"
              value={project.report_info.remarks}
              onchange={(e) => void editReportInfo(projectStore, e.currentTarget, "remarks")}
            ></textarea>
          </div>
        </section>

        <p class="text-sm leading-relaxed text-text-muted">
          {m.info_personal_hint()}
          <button
            type="button"
            class={linkClass}
            onclick={() => {
              view.openFile("options");
            }}>{m.cmd_options()}</button
          >
        </p>
      </div>
    </div>
  {/if}
</div>
