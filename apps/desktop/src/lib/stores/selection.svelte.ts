/**
 * Selected characteristics, shared by the viewport and the characteristic table (T1.6, T1.7).
 * View state only: never sent to Rust, keyed by stable characteristic ID (FR-BAL-09).
 *
 * API:
 * - `ids`: the selected characteristic IDs (read only set, replaced on every change).
 * - `select(ids, mode)`: `replace` the selection, `add` to it, or `toggle` each ID.
 * - `clear()`: select nothing.
 * - `has(id)`: whether a characteristic is selected.
 * - `focusRequest`: `{ id, seq } | null`, set by `focus(id)`. Asks the table to bring this
 *   characteristic into view and focus it for editing; `seq` increases with every request, so
 *   the same ID can be requested again.
 */

/* eslint-disable svelte/prefer-svelte-reactivity -- `ids` is a `$state.raw` set that is replaced
   on every change and never mutated, so plain sets are the reactive unit here. */

import type { CharId } from "$lib/ipc/bindings";

/** How {@link SelectionStore.select} combines the given IDs with the current selection. */
export type SelectMode = "replace" | "add" | "toggle";

/** A request to focus one characteristic for editing. */
export interface FocusRequest {
  id: CharId;
  seq: number;
}

export class SelectionStore {
  /** The selected characteristic IDs. Replaced, never mutated. */
  ids = $state.raw<ReadonlySet<CharId>>(new Set());
  /** The last request to focus a characteristic for editing. */
  focusRequest = $state.raw<FocusRequest | null>(null);

  /** Changes the selection: `replace` it, `add` to it, or `toggle` each ID. */
  select(ids: Iterable<CharId>, mode: SelectMode = "replace"): void {
    const next = mode === "replace" ? new Set<CharId>() : new Set(this.ids);
    for (const id of ids) {
      if (mode === "toggle" && next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
    }
    this.ids = next;
  }

  clear(): void {
    if (this.ids.size > 0) {
      this.ids = new Set();
    }
  }

  has(id: CharId): boolean {
    return this.ids.has(id);
  }

  /** Asks the table to focus this characteristic for editing. */
  focus(id: CharId): void {
    this.focusRequest = { id, seq: (this.focusRequest?.seq ?? 0) + 1 };
  }
}

/** The selection of the app window. */
export const selection = new SelectionStore();
