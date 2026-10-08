/** Main window views (D-50). Switched by this store, no router. */
export const VIEWS = ["drawing", "review", "measure", "export", "settings"] as const;

export type View = (typeof VIEWS)[number];

export function isView(value: string): value is View {
  return (VIEWS as readonly string[]).includes(value);
}

export class ViewStore {
  current = $state<View>("drawing");

  /** Switches to `value` if it names a view, otherwise keeps the current view. */
  set(value: string): void {
    if (isView(value)) {
      this.current = value;
    }
  }
}

export const view = new ViewStore();
