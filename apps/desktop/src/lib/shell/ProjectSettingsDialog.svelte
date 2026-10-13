<script lang="ts">
  /**
   * Project settings of the Drawing tab that need more room than a ribbon field (M3 decision
   * 21, T3.16): decimal place rules with the rounding after unit conversion, company tables
   * with the drawing rule, and the balloon style of the project. One small dialog, opened by
   * the ribbon button of each; the ribbon stays one row and the drawing stays visible behind.
   * Every change is an undoable Rust command and applies at once, so the dialog has only Close.
   */
  import { Dialog } from "bits-ui";
  import ProjectBalloonStyle from "$lib/components/ProjectBalloonStyle.svelte";
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
    }
  }

  // A closed project closes the dialog; there is nothing left to set.
  $effect(() => {
    if (!projectStore.isOpen) {
      view.projectDialog = null;
    }
  });
</script>

<Dialog.Root
  bind:open={
    () => view.projectDialog !== null,
    (open) => {
      if (!open) {
        view.projectDialog = null;
      }
    }
  }
>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-40 bg-black/30" />
    <Dialog.Content
      class="fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[36rem] max-w-[92vw] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 overflow-y-auto rounded-xl border border-border bg-surface p-5 text-text shadow-lg"
      data-testid="project-dialog"
    >
      {#if view.projectDialog !== null}
        {@const dialog = view.projectDialog}
        <Dialog.Title class="font-heading text-lg font-semibold">{title(dialog)}</Dialog.Title>
        <Dialog.Description class="text-xs text-text-muted">
          {m.project_dialog_hint()}
        </Dialog.Description>
        {#if dialog === "balloon_style"}
          <ProjectBalloonStyle />
        {:else}
          <ToleranceSettings section={dialog} />
        {/if}
        <div class="flex justify-end">
          <Dialog.Close
            class="min-h-8 rounded border border-border px-3 py-1 text-sm hover:bg-hover"
          >
            {m.project_dialog_close()}
          </Dialog.Close>
        </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
