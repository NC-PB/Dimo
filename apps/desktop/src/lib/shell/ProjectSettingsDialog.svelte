<script lang="ts">
  /**
   * Project settings of the Home and Drawing tabs that need more room than a ribbon field (M3
   * decision 21, T3.16, T3.18): decimal place rules with the rounding after unit conversion,
   * company tables with the drawing rule, the balloon style of the project, the numbering
   * options and a custom scale of the sheet on screen. One small dialog, opened by the ribbon
   * button or field of each; the ribbon stays one row and the drawing stays visible behind.
   * Every change is an undoable Rust command and applies at once, so the dialog has only Close.
   */
  import AppDialog from "$lib/components/AppDialog.svelte";
  import NumberingOptions from "$lib/components/NumberingOptions.svelte";
  import ProjectBalloonStyle from "$lib/components/ProjectBalloonStyle.svelte";
  import SheetScaleForm from "$lib/components/SheetScaleForm.svelte";
  import ToleranceSettings from "$lib/components/ToleranceSettings.svelte";
  import { m } from "$lib/i18n";
  import { projectStore } from "$lib/stores/project.svelte";
  import { view, type ProjectDialog } from "$lib/stores/view.svelte";

  function title(dialog: ProjectDialog): string {
    switch (dialog) {
      case "decimal_rules":
        return m.cmd_decimal_rules().replace(/…$/, "");
      case "company_tables":
        return m.cmd_company_tables();
      case "balloon_style":
        return m.cmd_project_balloon_style();
      case "numbering":
        return m.numbering_dialog_title();
      case "sheet_scale":
        return m.scale_dialog_title();
    }
  }

  // A closed project closes the dialog; there is nothing left to set.
  $effect(() => {
    if (!projectStore.isOpen) {
      view.projectDialog = null;
    }
  });
</script>

<AppDialog
  open={view.projectDialog !== null}
  onClose={() => {
    view.projectDialog = null;
  }}
  title={view.projectDialog === null ? "" : title(view.projectDialog)}
  description={m.project_dialog_hint()}
  closeLabel={m.project_dialog_close()}
  width="wide"
  testid="project-dialog"
>
  {#if view.projectDialog !== null}
    {@const dialog = view.projectDialog}
    {#if dialog === "balloon_style"}
      <ProjectBalloonStyle />
    {:else if dialog === "numbering"}
      <NumberingOptions />
    {:else if dialog === "sheet_scale"}
      <SheetScaleForm />
    {:else}
      <ToleranceSettings section={dialog} />
    {/if}
  {/if}
</AppDialog>
