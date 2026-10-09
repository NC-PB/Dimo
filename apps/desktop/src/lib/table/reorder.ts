/**
 * Reorder commands of the characteristic table (FR-BAL-06, D-21). The table only says where the
 * selected characteristics go; Rust moves them, renumbers in the same undo step and refuses the
 * move while numbering is locked (D-23).
 */

import type { CharId, Command } from "$lib/ipc/bindings";

/**
 * The `move_characteristics` command that puts `moving` into the gap before row `dropIndex`
 * of `order` (0 is before the first row, `order.length` after the last). The moved rows keep
 * their relative order. `null` if nothing moves.
 */
export function moveToGap(
  order: readonly CharId[],
  moving: ReadonlySet<CharId>,
  dropIndex: number,
): Command | null {
  const ids = order.filter((id) => moving.has(id));
  if (ids.length === 0) {
    return null;
  }
  const gap = Math.max(0, Math.min(order.length, Math.round(dropIndex)));
  const before = order.slice(gap).find((id) => !moving.has(id)) ?? null;
  if (isUnchanged(order, moving, before)) {
    return null;
  }
  return { type: "move_characteristics", ids, before };
}

/**
 * The command that moves `moving` one row up (`-1`) or down (`1`), past the next row that is
 * not moved. Rows that are not together are gathered at the new place. `null` at the ends.
 */
export function moveStep(
  order: readonly CharId[],
  moving: ReadonlySet<CharId>,
  direction: -1 | 1,
): Command | null {
  const indexes = order.flatMap((id, i) => (moving.has(id) ? [i] : []));
  const first = indexes[0];
  const last = indexes[indexes.length - 1];
  if (first === undefined || last === undefined) {
    return null;
  }
  return moveToGap(order, moving, direction < 0 ? first - 1 : last + 2);
}

/** True if inserting `moving` before `before` gives the same order. */
function isUnchanged(
  order: readonly CharId[],
  moving: ReadonlySet<CharId>,
  before: CharId | null,
): boolean {
  const rest = order.filter((id) => !moving.has(id));
  const picked = order.filter((id) => moving.has(id));
  const at = before === null ? rest.length : rest.indexOf(before);
  const next = [...rest.slice(0, at), ...picked, ...rest.slice(at)];
  return next.every((id, i) => id === order[i]);
}
