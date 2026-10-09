/**
 * Multi select rules of the viewport (FR-BAL-12). Pure functions on characteristic IDs; the
 * result goes into the shared selection store.
 *
 * - Click on a balloon: select only it. With Shift or Cmd/Ctrl ("additive"): toggle it.
 * - Click on empty drawing: clear the selection, unless additive.
 * - Press on a balloon and drag: a selected balloon drags the whole selection; an unselected one
 *   is selected first (alone, or added when additive), then dragged.
 * - Box select (Shift drag on empty drawing): adds the balloons whose center is in the box.
 */

/** Selection after a click (press and release without moving). `hit` is `null` on empty space. */
export function afterClick(
  selected: ReadonlySet<string>,
  hit: string | null,
  additive: boolean,
): string[] {
  if (hit === null) {
    return additive ? [...selected] : [];
  }
  if (!additive) {
    return [hit];
  }
  return selected.has(hit) ? [...selected].filter((id) => id !== hit) : [...selected, hit];
}

/** Selection that a drag starting on balloon `hit` moves. */
export function forDrag(selected: ReadonlySet<string>, hit: string, additive: boolean): string[] {
  if (selected.has(hit)) {
    return [...selected];
  }
  return additive ? [...selected, hit] : [hit];
}

/** Selection after a box select that found `inside`. */
export function afterBox(
  selected: ReadonlySet<string>,
  inside: readonly string[],
  additive: boolean,
): string[] {
  return additive ? [...new Set([...selected, ...inside])] : [...inside];
}
