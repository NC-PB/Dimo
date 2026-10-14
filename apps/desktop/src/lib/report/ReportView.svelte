<script lang="ts">
  import EmptyState from "$lib/components/EmptyState.svelte";
  import { commandErrorMessage, errorDetail, m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import Pane from "$lib/pane/Pane.svelte";
  import { exportStore } from "$lib/stores/export.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { untrack } from "svelte";
  import PreviewPage from "./PreviewPage.svelte";
  import ReportPane from "./ReportPane.svelte";
  import {
    currentView,
    reportView as appReportView,
    type ReportViewStore,
  } from "./report-view.svelte";

  interface Props {
    /** The Report tab state; the app's unless a test passes its own. */
    store?: ReportViewStore;
  }

  let { store = appReportView }: Props = $props();

  /**
   * The Report tab work area (T3.19, ADR 0007, mockup "Report tab"): a live preview of the
   * chosen document as the PDF that Save copy writes, and the pane with the checks before
   * issuing. The preview follows every change of the project and of the report options, a short
   * pause after the last one. The other exports are on File > Export.
   */
  const MAX_PAGE_WIDTH = 1100;
  const GUTTER = 56;

  let area = $state<HTMLElement | null>(null);
  let areaWidth = $state(0);

  $effect(() => {
    const element = area;
    if (element === null) {
      return;
    }
    const measure = () => {
      areaWidth = element.clientWidth;
    };
    measure();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
    observer?.observe(element);
    return () => {
      observer?.disconnect();
    };
  });

  // Render again after every patch, a new project, another document or other options.
  $effect(() => {
    const open = projectStore.isOpen;
    void projectStore.session;
    void projectStore.revision;
    void exportStore.document;
    void settingsStore.current;
    untrack(() => {
      if (open) {
        store.schedule(currentView());
      } else {
        store.clear();
      }
    });
  });

  const project = $derived(projectStore.project);
  const pageWidth = $derived(Math.max(0, Math.min(MAX_PAGE_WIDTH, areaWidth - GUTTER)));
  const pages = $derived(store.preview?.sheets ?? []);
  const xlsx = $derived(exportStore.reportFormat(exportStore.document).endsWith("_xlsx"));
</script>

<div class="flex min-h-0 flex-1">
  <section
    class="flex min-w-0 flex-1 flex-col bg-sunken"
    aria-label={m.report_preview_label()}
    data-testid="report-view"
  >
    {#if project === null}
      <EmptyState
        icon={icons.report}
        title={m.empty_no_project_title()}
        text={m.empty_report_no_project()}
        actions={[{ command: "new_project" }, { command: "open_project" }]}
      />
    {:else}
      <div
        class="flex min-h-9 shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-b border-border bg-surface px-4 py-1.5 text-xs text-text-muted"
      >
        <span data-testid="preview-pages">
          {pages.length === 1
            ? m.report_preview_one_page()
            : m.report_preview_pages({ count: String(pages.length) })}
        </span>
        {#if store.loading}
          <span aria-live="polite">{m.report_preview_loading()}</span>
        {/if}
        {#if xlsx}
          <span>{m.report_preview_xlsx()}</span>
        {/if}
        {#if store.previewError}
          <span
            class="text-danger"
            role="alert"
            title={errorDetail(store.previewError) ?? undefined}
            >{m.report_preview_failed({ message: commandErrorMessage(store.previewError) })}</span
          >
        {/if}
      </div>
      <div bind:this={area} class="min-h-0 flex-1 overflow-auto">
        {#if project.characteristics.length === 0}
          <div class="mx-auto mt-6 max-w-2xl rounded-xl border border-border bg-surface">
            <EmptyState
              size="inline"
              icon={icons.report}
              title={m.empty_report_no_chars_title()}
              text={m.empty_report_no_chars_text()}
              actions={[{ command: "place_tool" }]}
            />
          </div>
        {:else if project.results.length === 0 && exportStore.document === "inspection_report"}
          <div class="mx-auto mt-6 max-w-2xl rounded-xl border border-border bg-surface">
            <EmptyState
              size="inline"
              icon={icons.measure}
              title={m.empty_report_no_results_title()}
              text={m.empty_report_no_results_text()}
              actions={[{ tab: "measure", label: m.empty_go_measure() }]}
            />
          </div>
        {/if}
        {#if store.preview && pageWidth > 0}
          <div class="flex flex-col items-center gap-6 px-7 py-7">
            {#each pages as page, index (index)}
              <PreviewPage
                doc={store.preview.doc}
                {index}
                count={pages.length}
                {page}
                width={pageWidth}
              />
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </section>
  <!-- The shared pane frame of T3.18, filled with the checks (T3.19). -->
  {#if project !== null}
    <ReportPane {store} />
  {:else}
    <Pane title={m.report_pane_title()} label={m.report_pane_title()} />
  {/if}
</div>
