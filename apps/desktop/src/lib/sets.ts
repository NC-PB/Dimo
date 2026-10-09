/**
 * Immutable sets for `$state.raw` values: a new set replaces the old one on every change, so
 * reactive readers rerun without the per entry tracking of `SvelteSet`.
 */

/** A new set of `items` that callers must not mutate. */
export function frozenSet<T>(items: Iterable<T>): ReadonlySet<T> {
  return new Set(items);
}

/** `items` without the entries in `drop`, and whether anything was removed. */
export function without<T>(
  items: ReadonlySet<T>,
  drop: Iterable<T>,
): { set: ReadonlySet<T>; changed: boolean } {
  const next = new Set(items);
  let changed = false;
  for (const item of drop) {
    changed = next.delete(item) || changed;
  }
  return { set: next, changed };
}
