<script lang="ts">
  import { m } from "$lib/i18n";
  import type { ProjectInfo, ReportInfo } from "$lib/ipc/bindings";
  import { localDate } from "$lib/project-details";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";

  interface Props {
    /** The project store; the app's by default, a fake in tests. */
    projectStore?: ProjectStore;
  }

  let { projectStore = appProject }: Props = $props();

  /**
   * Project and report details (T3.4, FR-RVC-04, FR-DOC-07, M3 decisions 1 and 15): part data,
   * revision label, the drawing file and its hash, and the report header and approval fields.
   * Every edit is one undoable Rust command; the form shows what the project holds.
   */
  const project = $derived(projectStore.project);
  const revision = $derived(project?.revisions.find((r) => r.id === project.current_revision));

  // M3 decision 15: the company name of the app settings goes into an empty report info once
  // per opened project, through a command. Undoing it does not bring it back.
  let filledFor = -1;
  $effect(() => {
    if (projectStore.isOpen && filledFor !== projectStore.session) {
      filledFor = projectStore.session;
      void projectStore.fillReportCompany();
    }
  });

  /**
   * Sends a changed field. Afterwards the input shows what the project holds (`read`): Rust's
   * trimmed text, or the old value when Rust refused (the reason appears in the project
   * messages).
   */
  async function send(
    input: HTMLInputElement | HTMLTextAreaElement,
    read: () => string | undefined,
    run: () => Promise<unknown>,
  ): Promise<void> {
    if (input.value === read()) {
      return;
    }
    await run();
    input.value = read() ?? "";
  }

  function editInfo(
    event: Event & { currentTarget: HTMLInputElement },
    field: keyof ProjectInfo,
  ): void {
    if (!project) {
      return;
    }
    const info = { ...project.info, [field]: event.currentTarget.value };
    void send(
      event.currentTarget,
      () => projectStore.project?.info[field],
      () => projectStore.execute({ type: "update_project_info", info }),
    );
  }

  function editReport(
    event: Event & { currentTarget: HTMLInputElement | HTMLTextAreaElement },
    field: keyof ReportInfo,
  ): void {
    if (!project) {
      return;
    }
    const info = { ...project.report_info, [field]: event.currentTarget.value };
    void send(
      event.currentTarget,
      () => projectStore.project?.report_info[field],
      () => projectStore.execute({ type: "update_report_info", info }),
    );
  }

  function editRevision(event: Event & { currentTarget: HTMLInputElement }): void {
    if (!revision) {
      return;
    }
    const label = event.currentTarget.value;
    const id = revision.id;
    void send(
      event.currentTarget,
      () => projectStore.project?.revisions.find((r) => r.id === id)?.label,
      () => projectStore.execute({ type: "update_revision_label", revision: id, label }),
    );
  }

  function setToday(): void {
    if (project) {
      const info = { ...project.report_info, report_date: localDate(new Date()) };
      void projectStore.execute({ type: "update_report_info", info });
    }
  }

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

  const fieldClass = "w-full rounded border border-border bg-surface px-2 py-1 text-sm text-text";
  const labelClass = "flex flex-col gap-0.5 text-xs text-text-muted";
  const buttonClass =
    "rounded border border-border px-2 py-1 text-sm text-text hover:bg-surface-raised";

  const INFO_FIELDS = [
    { field: "part_number", label: () => m.field_part_number() },
    { field: "part_name", label: () => m.field_part_name() },
    { field: "customer", label: () => m.field_customer() },
    { field: "order_reference", label: () => m.field_order_reference() },
  ] as const;
  const REPORT_FIELDS = [
    { field: "report_number", label: () => m.field_report_number() },
    { field: "company_name", label: () => m.field_company_name() },
    { field: "inspected_by", label: () => m.field_inspected_by() },
    { field: "approved_by", label: () => m.field_approved_by() },
  ] as const;
</script>

{#if project && revision}
  <section class="flex flex-col gap-3" aria-labelledby="project-details-title">
    <h2 id="project-details-title" class="text-sm font-semibold text-text">
      {m.project_details_title()}
    </h2>

    <fieldset class="flex flex-col gap-2">
      <legend class="mb-1 text-xs font-semibold text-text-muted">
        {m.project_details_part()}
      </legend>
      {#each INFO_FIELDS as { field, label } (field)}
        <label class={labelClass}>
          {label()}
          <input
            type="text"
            class={fieldClass}
            autocomplete="off"
            value={project.info[field]}
            onchange={(e) => {
              editInfo(e, field);
            }}
          />
        </label>
      {/each}
    </fieldset>

    <fieldset class="flex flex-col gap-2">
      <legend class="mb-1 text-xs font-semibold text-text-muted">
        {m.project_details_drawing()}
      </legend>
      <label class={labelClass}>
        {m.field_revision_label()}
        <input
          type="text"
          class={fieldClass}
          autocomplete="off"
          value={revision.label}
          onchange={editRevision}
        />
      </label>
      <div class={labelClass}>
        <span id="drawing-file-label">{m.field_drawing_file()}</span>
        <span
          class="break-all text-sm text-text"
          aria-labelledby="drawing-file-label"
          data-testid="drawing-file">{revision.file_name}</span
        >
      </div>
      <div class={labelClass}>
        <span id="drawing-sha-label">{m.field_drawing_sha()}</span>
        <div class="flex items-start gap-2">
          <code
            class="min-w-0 flex-1 break-all text-xs text-text"
            aria-labelledby="drawing-sha-label"
            data-testid="drawing-sha">{revision.sha256}</code
          >
          <button
            type="button"
            class={buttonClass}
            aria-label={m.copy_hash_aria()}
            onclick={() => void copyHash()}
          >
            {copied ? m.copy_hash_done() : m.copy_hash()}
          </button>
        </div>
      </div>
    </fieldset>

    <fieldset class="flex flex-col gap-2">
      <legend class="mb-1 text-xs font-semibold text-text-muted">
        {m.project_details_report()}
      </legend>
      {#each REPORT_FIELDS.slice(0, 1) as { field, label } (field)}
        <label class={labelClass}>
          {label()}
          <input
            type="text"
            class={fieldClass}
            autocomplete="off"
            value={project.report_info[field]}
            onchange={(e) => {
              editReport(e, field);
            }}
          />
        </label>
      {/each}
      <div class={labelClass}>
        <label for="report-date">{m.field_report_date()}</label>
        <div class="flex items-center gap-2">
          <input
            id="report-date"
            type="date"
            class={fieldClass}
            value={project.report_info.report_date}
            onchange={(e) => {
              editReport(e, "report_date");
            }}
          />
          <button
            type="button"
            class={buttonClass}
            aria-label={m.report_date_today_aria()}
            onclick={setToday}>{m.report_date_today()}</button
          >
        </div>
      </div>
      {#each REPORT_FIELDS.slice(1) as { field, label } (field)}
        <label class={labelClass}>
          {label()}
          <input
            type="text"
            class={fieldClass}
            autocomplete="off"
            value={project.report_info[field]}
            onchange={(e) => {
              editReport(e, field);
            }}
          />
        </label>
      {/each}
      <label class={labelClass}>
        {m.field_remarks()}
        <textarea
          class={fieldClass}
          rows="3"
          value={project.report_info.remarks}
          onchange={(e) => {
            editReport(e, "remarks");
          }}></textarea>
      </label>
    </fieldset>
  </section>
{/if}
