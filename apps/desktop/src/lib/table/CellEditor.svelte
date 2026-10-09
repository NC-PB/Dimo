<script lang="ts" module>
  /**
   * Where the active cell goes after a confirmed edit. `blur`: focus went elsewhere, the edit is
   * confirmed without taking the focus back.
   */
  export type EditMove = "none" | "up" | "down" | "next" | "previous" | "blur";
</script>

<script lang="ts">
  import { untrack } from "svelte";

  interface Props {
    /** Text in the editor. */
    text: string;
    /** True while Rust handles the change. */
    pending: boolean;
    /** True if Rust refused the last attempt. */
    invalid: boolean;
    numeric: boolean;
    /** Accessible name, the column header. */
    label: string;
    /** Select the whole text when the editor opens (else the caret goes to the end). */
    selectAll: boolean;
    onInput: (text: string) => void;
    onCommit: (move: EditMove) => void;
    onCancel: () => void;
  }

  let { text, pending, invalid, numeric, label, selectAll, onInput, onCommit, onCancel }: Props =
    $props();

  function focusOnMount(element: HTMLInputElement) {
    untrack(() => {
      element.focus({ preventScroll: true });
      if (selectAll) {
        element.select();
      } else {
        element.setSelectionRange(element.value.length, element.value.length);
      }
    });
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.isComposing) {
      return;
    }
    const plain = !event.altKey && !event.ctrlKey && !event.metaKey;
    let move: EditMove | null = null;
    switch (event.key) {
      case "Enter":
        move = "none";
        break;
      case "Tab":
        move = event.shiftKey ? "previous" : "next";
        break;
      case "ArrowUp":
        move = plain ? "up" : null;
        break;
      case "ArrowDown":
        move = plain ? "down" : null;
        break;
      case "Escape":
        event.preventDefault();
        onCancel();
        return;
    }
    if (move !== null) {
      event.preventDefault();
      onCommit(move);
    }
  }
</script>

<input
  {@attach focusOnMount}
  class="editor h-full w-full border-0 bg-surface px-2 text-text outline-2 -outline-offset-2"
  class:text-right={numeric}
  class:tabular-nums={numeric}
  class:outline-danger={invalid}
  class:outline-focus={!invalid}
  value={text}
  readonly={pending}
  aria-label={label}
  aria-invalid={invalid}
  spellcheck="false"
  autocomplete="off"
  oninput={(event) => {
    onInput(event.currentTarget.value);
  }}
  onkeydown={onKeyDown}
  onblur={() => {
    onCommit("blur");
  }}
/>
