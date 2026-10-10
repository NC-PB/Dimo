import type { Characteristic } from "$lib/ipc/bindings";

/**
 * The selected characteristics that exist, in the order of `byId`. The project store builds it
 * in placement order, which Rust keeps equal to the number order (D-21, D-23), as the table
 * lists them.
 */
export function selectedCharacteristics(
  ids: ReadonlySet<string>,
  byId: ReadonlyMap<string, Characteristic>,
): Characteristic[] {
  const chosen: Characteristic[] = [];
  if (ids.size === 0) {
    return chosen;
  }
  for (const c of byId.values()) {
    if (ids.has(c.id)) {
      chosen.push(c);
    }
  }
  return chosen;
}
