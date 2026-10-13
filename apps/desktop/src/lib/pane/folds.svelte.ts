/**
 * Which sections of the properties pane are folded (T3.18). View state only, remembered per user
 * in the browser storage of the window; without storage (a private window, a test) the sections
 * simply start unfolded. Nothing here is saved in the project.
 */

const KEY = "dimo.pane.folded";

/** The storage the folds are kept in, or `null` when there is none or it cannot be used. */
function browserStorage(): Storage | null {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

export class PaneFolds {
  /** IDs of the folded sections, sorted. */
  #folded = $state<readonly string[]>([]);
  readonly #storage: () => Storage | null;

  /** @param storage where the folded sections are remembered; the browser's by default */
  constructor(storage: () => Storage | null = browserStorage) {
    this.#storage = storage;
    this.#folded = this.#load();
  }

  /** Whether section `id` shows its content. Sections start unfolded. */
  isOpen(id: string): boolean {
    return !this.#folded.includes(id);
  }

  /** Folds section `id` when it is open, unfolds it when it is folded. */
  toggle(id: string): void {
    const next = this.#folded.includes(id)
      ? this.#folded.filter((f) => f !== id)
      : [...this.#folded, id].sort();
    this.#folded = next;
    try {
      this.#storage()?.setItem(KEY, JSON.stringify(next));
    } catch {
      // Storage full or blocked: the fold still applies until the window closes.
    }
  }

  #load(): readonly string[] {
    try {
      const stored: unknown = JSON.parse(this.#storage()?.getItem(KEY) ?? "[]");
      return Array.isArray(stored)
        ? stored.filter((v): v is string => typeof v === "string").sort()
        : [];
    } catch {
      return [];
    }
  }
}

/** The folds of the app window. */
export const paneFolds = new PaneFolds();
