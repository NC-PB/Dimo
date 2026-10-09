/**
 * Selected characteristics, shared by the viewport (balloons) and the characteristic table.
 * View state only (frontend.md): never sent to Rust, keyed by characteristic ID.
 *
 * In M1 every characteristic has exactly one balloon, so selecting a characteristic selects its
 * balloon and the other way round. Components never keep their own copy of the selection.
 *
 * API (stable, T1.6 and T1.7 build on it):
 *
 * Reading (reactive):
 * - `ids: ReadonlySet<CharId>`: the selected characteristic IDs. Replaced on every change,
 *   never mutated, so `$derived` values that read it rerun.
 * - `size`, `isEmpty`, `has(id)`.
 * - `primary: CharId | null`: the most recently selected ID, the anchor for range selection and
 *   the one an editor shows. `null` when the selection is empty.
 * - `focusRequest: { id: CharId; seq: number; from: FocusSource } | null`: "focus this
 *   characteristic for editing". Set by {@link SelectionStore.focus}; `seq` increases with every
 *   request, so an `$effect` that reads it runs again even for the same ID. `from` names the
 *   view that asked: that view takes the keyboard focus (the viewport opens its inline value
 *   editor, the table its first editable cell); the other views only scroll the item into view.
 *
 * Changing:
 * - `select(ids, primary?)`: replaces the selection.
 * - `add(ids)`, `remove(ids)`, `toggle(id)`: additive changes (Shift or Cmd/Ctrl click).
 * - `clear()`.
 * - `focus(id, from)`: selects only `id` and raises a new `focusRequest`.
 * - `retain(exists)`: drops IDs that no longer exist, for example after delete or undo.
 */

import type { CharId } from "$lib/ipc/bindings";

const EMPTY: ReadonlySet<CharId> = new Set();

/** The view that asks to focus a characteristic. */
export type FocusSource = "viewport" | "table";

export interface FocusRequest {
  /** The characteristic to focus. */
  id: CharId;
  /** Increases with every request. */
  seq: number;
  /** The view that asked; it takes the keyboard focus, the others scroll to the item. */
  from: FocusSource;
}

export class SelectionStore {
  ids = $state.raw<ReadonlySet<CharId>>(EMPTY);
  primary = $state<CharId | null>(null);
  focusRequest = $state.raw<FocusRequest | null>(null);

  #seq = 0;

  get size(): number {
    return this.ids.size;
  }

  get isEmpty(): boolean {
    return this.ids.size === 0;
  }

  has(id: CharId): boolean {
    return this.ids.has(id);
  }

  /** Replaces the selection. `primary` defaults to the last of `ids`. */
  select(ids: Iterable<CharId>, primary?: CharId | null): void {
    const next = new Set(ids);
    this.#set(next, primary === undefined ? last(next) : primary);
  }

  /** Adds `ids`; the last one becomes primary. */
  add(ids: Iterable<CharId>): void {
    const added = [...ids];
    if (added.length === 0) {
      return;
    }
    this.#set(new Set([...this.ids, ...added]), added[added.length - 1] ?? null);
  }

  /** Removes `ids`. */
  remove(ids: Iterable<CharId>): void {
    const next = new Set(this.ids);
    let changed = false;
    for (const id of ids) {
      changed = next.delete(id) || changed;
    }
    if (changed) {
      this.#set(next, this.primary !== null && next.has(this.primary) ? this.primary : last(next));
    }
  }

  /** Adds `id` if it is not selected, otherwise removes it. */
  toggle(id: CharId): void {
    if (this.ids.has(id)) {
      this.remove([id]);
    } else {
      this.add([id]);
    }
  }

  clear(): void {
    if (this.ids.size > 0 || this.primary !== null) {
      this.#set(EMPTY, null);
    }
  }

  /** Selects only `id` and asks the views to focus it for editing. */
  focus(id: CharId, from: FocusSource): void {
    this.select([id], id);
    this.focusRequest = { id, seq: ++this.#seq, from };
  }

  /** Keeps only IDs for which `exists` is true. */
  retain(exists: (id: CharId) => boolean): void {
    const gone = [...this.ids].filter((id) => !exists(id));
    if (gone.length > 0) {
      this.remove(gone);
    }
    if (this.focusRequest !== null && !exists(this.focusRequest.id)) {
      this.focusRequest = null;
    }
  }

  #set(ids: ReadonlySet<CharId>, primary: CharId | null): void {
    this.ids = ids.size === 0 ? EMPTY : ids;
    this.primary = primary;
  }
}

function last(ids: ReadonlySet<CharId>): CharId | null {
  let result: CharId | null = null;
  for (const id of ids) {
    result = id;
  }
  return result;
}

/** The selection of the app window. */
export const selection = new SelectionStore();
