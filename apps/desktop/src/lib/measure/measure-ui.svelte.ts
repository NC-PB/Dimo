/**
 * What the Measure ribbon needs from the measurement grid (T3.15): the active row and part, the
 * row filters, and the grid's own paste, clear and result type actions. View state only; the
 * grid sends the commands and shows Rust's refusals itself.
 */

import type { CharId, ResultType, SampleId } from "$lib/ipc/bindings";

/** Actions of the mounted grid. */
export interface GridActions {
  /** Pastes a block of text at the active cell, as Cmd/Ctrl+V does. */
  paste: (text: string) => Promise<void>;
  /** Clears the active cell. */
  clear: () => Promise<void>;
  /** Sets the result type of the active row. */
  setResultType: (value: ResultType) => Promise<void>;
  /** Gives the grid the keyboard focus. */
  focus: () => void;
  /** Shows a line above the grid. */
  notify: (text: string) => void;
}

export class MeasureUiStore {
  /** Characteristic of the active grid row. */
  activeRow = $state<CharId | null>(null);
  /** Part (sample) of the active grid column, `null` on the result type column or without one. */
  activeSample = $state<SampleId | null>(null);
  /** Show only rows with a missing value (verdicts from Rust). */
  onlyMissing = $state(false);
  /** Show only rows that are not OK (verdicts from Rust). */
  onlyFailed = $state(false);
  #grid = $state.raw<GridActions | null>(null);

  /** Whether a grid is on screen. */
  get attached(): boolean {
    return this.#grid !== null;
  }

  /** The grid registers its actions while mounted; call the returned function to detach. */
  attach(actions: GridActions): () => void {
    this.#grid = actions;
    return () => {
      if (this.#grid === actions) {
        this.#grid = null;
        this.activeRow = null;
        this.activeSample = null;
      }
    };
  }

  /**
   * Pastes the clipboard at the active cell. The webview may refuse to read the clipboard for a
   * button; then the grid takes the focus and says to press the paste keys.
   */
  async paste(hint: string, read: () => Promise<string> = readClipboard): Promise<void> {
    const grid = this.#grid;
    if (grid === null) {
      return;
    }
    let text: string;
    try {
      text = await read();
    } catch {
      grid.focus();
      grid.notify(hint);
      return;
    }
    await grid.paste(text);
    grid.focus();
  }

  async clear(): Promise<void> {
    await this.#grid?.clear();
    this.#grid?.focus();
  }

  async setResultType(value: ResultType): Promise<void> {
    await this.#grid?.setResultType(value);
  }
}

function readClipboard(): Promise<string> {
  return navigator.clipboard.readText();
}

export const measureUi = new MeasureUiStore();
