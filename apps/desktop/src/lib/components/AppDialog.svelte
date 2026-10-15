<script lang="ts">
  /**
   * The one modal dialog of the app (MF F8): overlay, frame, title, optional description, the
   * content and a row of buttons with the close or cancel button first. While it is open it
   * counts itself in `keyboard.modals`, so no window shortcut acts behind it (D-52); Escape and
   * the close button call `onClose`.
   *
   * `alert` is for a question that needs an answer (unsaved changes, deleting results): it does
   * not close by a click outside.
   */
  import { AlertDialog, Dialog } from "bits-ui";
  import { untrack, type Snippet } from "svelte";
  import { keyboard as appKeyboard, type KeyboardStore } from "$lib/stores/keyboard.svelte";
  import { buttonClass } from "$lib/ui/fields";

  interface Props {
    open: boolean;
    /** Called when the dialog asks to close: Escape, the close button, a click outside. */
    onClose: () => void;
    title: string;
    description?: string;
    /** Label of the close or cancel button. */
    closeLabel: string;
    /** A question that must be answered: no closing by a click outside. */
    alert?: boolean;
    /** Width of the frame. */
    width?: "narrow" | "wide";
    testid?: string;
    /** Keys pressed inside the dialog, for example `?` closing the cheat sheet. */
    onkeydown?: (event: KeyboardEvent) => void;
    children?: Snippet;
    /** Buttons after the close button, the main action last. */
    actions?: Snippet;
    /** The shortcut state; the app's unless a test passes its own. */
    keyboard?: KeyboardStore;
  }

  let {
    open,
    onClose,
    title,
    description,
    closeLabel,
    alert = false,
    width = "narrow",
    testid,
    onkeydown,
    children,
    actions,
    keyboard = appKeyboard,
  }: Props = $props();

  // Counted while open; untracked, so the count it changes does not run the effect again.
  $effect(() => {
    if (open) {
      return untrack(() => keyboard.openModal());
    }
  });

  const contentClass = $derived(
    `fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] ${width === "wide" ? "w-[36rem] max-w-[92vw]" : "w-96 max-w-[90vw]"} -translate-x-1/2 -translate-y-1/2 flex-col gap-3 overflow-y-auto rounded-xl border border-border bg-surface p-5 text-text shadow-lg`,
  );
  const overlayClass = "fixed inset-0 z-40 bg-black/40";
  const titleClass = "font-heading text-lg font-semibold";
  const descriptionClass = "text-sm text-text-muted";
  const footerClass = "mt-1 flex justify-end gap-2";

  function onOpenChange(next: boolean) {
    if (!next) {
      onClose();
    }
  }
</script>

{#if alert}
  <AlertDialog.Root bind:open={() => open, onOpenChange}>
    <AlertDialog.Portal>
      <AlertDialog.Overlay class={overlayClass} />
      <AlertDialog.Content class={contentClass} data-testid={testid} {onkeydown}>
        <AlertDialog.Title class={titleClass}>{title}</AlertDialog.Title>
        {#if description !== undefined}
          <AlertDialog.Description class={descriptionClass}>{description}</AlertDialog.Description>
        {/if}
        {@render children?.()}
        <div class={footerClass}>
          <AlertDialog.Cancel class={buttonClass.secondary}>{closeLabel}</AlertDialog.Cancel>
          {@render actions?.()}
        </div>
      </AlertDialog.Content>
    </AlertDialog.Portal>
  </AlertDialog.Root>
{:else}
  <Dialog.Root bind:open={() => open, onOpenChange}>
    <Dialog.Portal>
      <Dialog.Overlay class={overlayClass} />
      <Dialog.Content class={contentClass} data-testid={testid} {onkeydown}>
        <Dialog.Title class={titleClass}>{title}</Dialog.Title>
        {#if description !== undefined}
          <Dialog.Description class={descriptionClass}>{description}</Dialog.Description>
        {/if}
        {@render children?.()}
        <div class={footerClass}>
          <Dialog.Close class={buttonClass.secondary}>{closeLabel}</Dialog.Close>
          {@render actions?.()}
        </div>
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
{/if}
