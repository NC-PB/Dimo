<script lang="ts">
  import CharacteristicDetail from "$lib/components/CharacteristicDetail.svelte";
  import ProjectSummary from "$lib/components/ProjectSummary.svelte";
  import SelectionSummary from "$lib/components/SelectionSummary.svelte";
  import ZonesAndViews from "$lib/components/ZonesAndViews.svelte";
  import { commandErrorMessage, m, strategyHint } from "$lib/i18n";
  import Pane from "$lib/pane/Pane.svelte";
  import PaneSection from "$lib/pane/PaneSection.svelte";
  import { shownSheet } from "$lib/sheet-properties";
  import {
    numberingStore as appNumbering,
    type NumberingStore,
  } from "$lib/stores/numbering.svelte";
  import { projectStore as appProject, type ProjectStore } from "$lib/stores/project.svelte";
  import { selection as appSelection, type SelectionStore } from "$lib/stores/selection.svelte";
  import { sheetRegions } from "$lib/stores/sheet-regions.svelte";
  import {
    toleranceStore as appTolerance,
    type ToleranceStore,
  } from "$lib/stores/tolerance.svelte";
  import { view as appView, type PaneTool, type ViewStore } from "$lib/stores/view.svelte";
  import { untrack } from "svelte";

  /**
   * The properties pane right of the drawing (ADR 0007, M3 decision 21, T3.18). It follows the
   * selection: one characteristic shows its properties, several a summary with the actions for
   * all of them, nothing selected a summary of the project. A ribbon command that needs room
   * (Zone grid, Draw view) opens its own section on top until it is closed; while new numbers
   * are previewed, a section says how many change. Sheet properties, project details and the
   * numbering strategy are not repeated here: they live on the ribbon and in File > Info.
   */
  interface Props {
    /** The stores; the app's by default, fakes in tests. */
    project?: ProjectStore;
    selected?: SelectionStore;
    tolerance?: ToleranceStore;
    numbering?: NumberingStore;
    view?: ViewStore;
  }

  let {
    project = appProject,
    selected = appSelection,
    tolerance = appTolerance,
    numbering = appNumbering,
    view = appView,
  }: Props = $props();

  const count = $derived(selected.size);
  const single = $derived(
    count === 1 && selected.primary !== null
      ? (project.characteristicById.get(selected.primary) ?? null)
      : null,
  );
  const sheet = $derived(view.paneTool === null ? undefined : shownSheet());
  /** The section of a ribbon command on screen; the × of the header then closes it. */
  const tool = $derived(view.paneTool !== null && sheet !== undefined ? view.paneTool : null);

  // A new selection shows its properties: the section of a ribbon command gives way.
  $effect(() => {
    if (selected.size > 0) {
      untrack(() => {
        if (view.paneTool !== null) {
          closeTool(view.paneTool);
        }
      });
    }
  });

  const title = $derived(
    tool === "zones"
      ? m.zones_title()
      : tool === "views"
        ? m.views_title()
        : single
          ? m.detail_title({ number: single.number })
          : count > 1
            ? m.selection_count({ count: String(count) })
            : m.pane_project_title(),
  );
  const label = $derived(
    tool !== null
      ? m.side_panel_label()
      : single
        ? m.pane_characteristic_label()
        : count > 1
          ? m.pane_selection_label()
          : m.pane_project_label(),
  );

  function closeTool(tool: PaneTool): void {
    view.paneTool = null;
    if (sheetRegions.tool === (tool === "zones" ? "zone_frame" : "view")) {
      sheetRegions.cancel();
    }
  }

  /** The × of the header: closes the section of a ribbon command, else hides the pane. */
  function close(): void {
    if (tool !== null) {
      closeTool(tool);
    } else {
      view.showSidePanel = false;
    }
  }

  /** The developer panel (T0.8), loaded only in development builds (MF F8). */
  const devPanel = import.meta.env.DEV ? import("$lib/dev/DevPanel.svelte") : null;
</script>

<Pane
  {title}
  {label}
  onclose={close}
  closeLabel={tool === null ? m.pane_hide() : m.pane_close_section({ section: title })}
  testid="properties-pane"
>
  {#if numbering.previewing}
    <PaneSection
      id="numbering.preview"
      title={m.pane_numbering_preview()}
      onclose={() => {
        numbering.setPreviewing(false);
      }}
      closeLabel={m.pane_close_section({ section: m.pane_numbering_preview() })}
    >
      <p class="m-0 text-sm text-text-muted">{strategyHint(numbering.strategy)}</p>
      {#if numbering.preview}
        <p class="m-0 text-sm" aria-live="polite" data-testid="numbering-changes">
          {m.numbering_changes({ count: String(numbering.preview.changed) })}
        </p>
      {/if}
      {#if numbering.error}
        <p class="m-0 text-xs text-danger" role="alert">
          {m.numbering_preview_failed({ message: commandErrorMessage(numbering.error) })}
        </p>
      {/if}
    </PaneSection>
  {/if}

  {#if tool !== null && sheet}
    <!-- The pane header names the section; its × closes it. -->
    <div class="flex flex-col gap-2 border-b border-border px-3.5 py-3" data-section="tool.{tool}">
      <ZonesAndViews {sheet} part={tool} />
    </div>
  {:else if single}
    <CharacteristicDetail {project} {selected} {tolerance} />
  {:else if count > 1}
    <SelectionSummary {project} {selected} {tolerance} />
  {:else if project.isOpen}
    <ProjectSummary {project} {tolerance} />
  {/if}

  {#if devPanel !== null}
    {#await devPanel then { default: DevPanel }}
      <div class="px-3.5 py-2 text-text-muted">
        <DevPanel />
      </div>
    {/await}
  {/if}
</Pane>
