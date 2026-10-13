/**
 * Tabs and work areas of the main window (ADR 0007, replaces the D-50 view list). Switched by
 * this store, no router.
 *
 * A tab picks the ribbon; the work area below follows it: Home and Drawing both show the
 * drawing, Report shows the export view, File a full page. The View tab keeps the work area that
 * was shown, so its zoom and pane commands act on it. The settings page is reached from File
 * until File > Options replaces it (T3.16).
 */
export const TABS = ["file", "home", "drawing", "review", "measure", "report", "view"] as const;

export type Tab = (typeof TABS)[number];

/** Work areas below the ribbon. */
export const VIEWS = ["drawing", "review", "measure", "export", "settings", "file"] as const;

export type View = (typeof VIEWS)[number];

export function isTab(value: string): value is Tab {
  return (TABS as readonly string[]).includes(value);
}

export function isView(value: string): value is View {
  return (VIEWS as readonly string[]).includes(value);
}

/** The work area a tab shows; `null` keeps the current one (View tab). */
function areaOf(tab: Tab): View | null {
  switch (tab) {
    case "file":
      return "file";
    case "home":
    case "drawing":
      return "drawing";
    case "review":
      return "review";
    case "measure":
      return "measure";
    case "report":
      return "export";
    case "view":
      return null;
  }
}

/** The tab that belongs to a work area. */
function tabOf(area: View): Tab {
  switch (area) {
    case "drawing":
      return "home";
    case "review":
      return "review";
    case "measure":
      return "measure";
    case "export":
      return "report";
    case "settings":
    case "file":
      return "file";
  }
}

export class ViewStore {
  tab = $state<Tab>("home");
  #area = $state<View>("drawing");
  /** View tab: the characteristic table under the drawing. */
  showTable = $state(true);
  /** View tab: the side panel right of the drawing. */
  showSidePanel = $state(true);
  /** Whether the keyboard shortcut cheat sheet (help) is open. */
  shortcutsOpen = $state(false);

  /** The work area on screen. */
  get current(): View {
    return this.#area;
  }

  /** Switches to tab `value` if it names one. */
  setTab(value: string): void {
    if (!isTab(value)) {
      return;
    }
    this.tab = value;
    const area = areaOf(value);
    if (area !== null) {
      this.#area = area;
    } else if (this.#area === "file" || this.#area === "settings") {
      this.#area = "drawing"; // The View tab acts on the drawing, never on a page.
    }
  }

  /**
   * Shows work area `value` if it names one, otherwise keeps the current one. The tab follows,
   * except that the drawing keeps Home, Drawing or View when one of them is open.
   */
  set(value: string): void {
    if (!isView(value)) {
      return;
    }
    this.#area = value;
    const keep = value === "drawing" && areaOf(this.tab) === "drawing";
    if (!keep && !(this.tab === "view" && value !== "file" && value !== "settings")) {
      this.tab = tabOf(value);
    }
  }
}

export const view = new ViewStore();
