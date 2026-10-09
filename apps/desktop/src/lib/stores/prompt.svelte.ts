import type { UnsavedChoice } from "./project.svelte";

/**
 * The "save changes?" question. {@link UnsavedPrompt.ask} opens the dialog and resolves with the
 * user's answer; the dialog component calls {@link UnsavedPrompt.answer}.
 */
export class UnsavedPrompt {
  open = $state(false);
  #resolve: ((choice: UnsavedChoice) => void) | null = null;

  ask(): Promise<UnsavedChoice> {
    // A second question replaces an unanswered one, which counts as cancelled.
    this.#resolve?.("cancel");
    this.open = true;
    return new Promise((resolve) => {
      this.#resolve = resolve;
    });
  }

  answer(choice: UnsavedChoice): void {
    const resolve = this.#resolve;
    this.#resolve = null;
    this.open = false;
    resolve?.(choice);
  }
}

export const unsavedPrompt = new UnsavedPrompt();
