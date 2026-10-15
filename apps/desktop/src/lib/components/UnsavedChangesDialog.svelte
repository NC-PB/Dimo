<script lang="ts">
  import AppDialog from "$lib/components/AppDialog.svelte";
  import { m } from "$lib/i18n";
  import { unsavedPrompt } from "$lib/stores/prompt.svelte";
  import { buttonClass } from "$lib/ui/fields";
</script>

<!-- Asked before unsaved changes would be lost: new, open, window close (NFR-REL-01). -->
<AppDialog
  alert
  open={unsavedPrompt.open}
  onClose={() => {
    unsavedPrompt.answer("cancel");
  }}
  title={m.unsaved_title()}
  description={m.unsaved_body()}
  closeLabel={m.unsaved_cancel()}
  testid="unsaved-dialog"
>
  {#snippet actions()}
    <!-- Plain buttons: they answer first, which also closes the dialog. -->
    <button
      type="button"
      class={buttonClass.secondary}
      data-answer="discard"
      onclick={() => {
        unsavedPrompt.answer("discard");
      }}
    >
      {m.unsaved_discard()}
    </button>
    <button
      type="button"
      class={buttonClass.primary}
      data-answer="save"
      onclick={() => {
        unsavedPrompt.answer("save");
      }}
    >
      {m.unsaved_save()}
    </button>
  {/snippet}
</AppDialog>
