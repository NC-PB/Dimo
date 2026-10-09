<script lang="ts">
  import { AlertDialog } from "bits-ui";
  import { m } from "$lib/i18n";
  import { unsavedPrompt } from "$lib/stores/prompt.svelte";

  const buttonClass = "rounded px-3 py-1 text-sm";
</script>

<!-- Asked before unsaved changes would be lost: new, open, window close (NFR-REL-01). -->
<AlertDialog.Root
  bind:open={
    () => unsavedPrompt.open,
    (open) => {
      if (!open) {
        unsavedPrompt.answer("cancel");
      }
    }
  }
>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="fixed inset-0 z-40 bg-black/40" />
    <AlertDialog.Content
      class="fixed top-1/2 left-1/2 z-50 w-96 max-w-[90vw] -translate-x-1/2 -translate-y-1/2 rounded border border-border bg-surface p-4 text-text shadow-lg"
    >
      <AlertDialog.Title class="mb-2 text-base font-semibold">{m.unsaved_title()}</AlertDialog.Title
      >
      <AlertDialog.Description class="text-sm text-text-muted">
        {m.unsaved_body()}
      </AlertDialog.Description>
      <div class="mt-4 flex justify-end gap-2">
        <AlertDialog.Cancel class="{buttonClass} hover:bg-surface-raised">
          {m.unsaved_cancel()}
        </AlertDialog.Cancel>
        <button
          type="button"
          class="{buttonClass} border border-border hover:bg-surface-raised"
          onclick={() => {
            unsavedPrompt.answer("discard");
          }}
        >
          {m.unsaved_discard()}
        </button>
        <!-- Plain buttons: they answer first, which also closes the dialog. -->
        <button
          type="button"
          class="{buttonClass} bg-accent text-accent-text"
          onclick={() => {
            unsavedPrompt.answer("save");
          }}
        >
          {m.unsaved_save()}
        </button>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
