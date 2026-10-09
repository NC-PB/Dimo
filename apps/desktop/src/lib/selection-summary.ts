import type { Characteristic } from "$lib/ipc/bindings";

/** The selected characteristics that exist, in number order (as the table lists them). */
export function selectedCharacteristics(
  ids: ReadonlySet<string>,
  byId: ReadonlyMap<string, Characteristic>,
): Characteristic[] {
  const chosen: Characteristic[] = [];
  for (const id of ids) {
    const c = byId.get(id);
    if (c) {
      chosen.push(c);
    }
  }
  return chosen.sort((a, b) => a.number - b.number);
}
