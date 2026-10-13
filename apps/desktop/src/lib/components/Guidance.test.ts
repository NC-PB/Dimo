// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { characteristic, emptyProject, loaded } from "$lib/stores/fixtures";
import { projectStore } from "$lib/stores/project.svelte";
import { recentStore } from "$lib/stores/recent.svelte";
import { view } from "$lib/stores/view.svelte";
import EmptyState, { type EmptyAction } from "./EmptyState.svelte";
import { ReportViewStore } from "$lib/report/report-view.svelte";
import ReportView from "$lib/report/ReportView.svelte";
import ExportView from "./ExportView.svelte";
import SelectionSummary from "./SelectionSummary.svelte";
import StartScreen from "./StartScreen.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;
let session = 5000;

function render(create: (target: HTMLElement) => ReturnType<typeof mount>) {
  target = document.createElement("div");
  document.body.append(target);
  component = create(target);
  flushSync();
}

function start(now?: Date) {
  render((t) => mount(StartScreen, { target: t, props: now ? { now } : {} }));
}

function emptyState(actions: EmptyAction[]) {
  render((t) => mount(EmptyState, { target: t, props: { title: "Title", text: "Text", actions } }));
}

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  vi.restoreAllMocks();
  projectStore.busy = false;
  recentStore.items = [];
  projectStore.load(loaded(++session, null));
  balloonTools.setTool("select");
  view.setTab("home");
});

function card(title: string): HTMLButtonElement {
  const found = [...target.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
    b.textContent.includes(title),
  );
  if (!found) {
    throw new Error(`no card ${title}`);
  }
  return found;
}

describe("start screen (T3.17)", () => {
  it("greets by the time of day and lists the two ways in and the three steps", () => {
    start(new Date(2026, 9, 13, 8, 30));
    expect(target.querySelector("h1")?.textContent).toBe("Good morning");
    expect(card("New project from drawing").getAttribute("aria-disabled")).toBeNull();
    expect(card("Open project").getAttribute("aria-disabled")).toBeNull();
    const steps = [...target.querySelectorAll("ol li strong")].map((s) => s.textContent);
    expect(steps).toEqual(["Balloon the drawing", "Measure the parts", "Print the report"]);
    // Without recent projects the slot says so instead of listing anything.
    const slot = target.querySelector('[data-slot="recent-projects"]');
    expect(slot?.querySelector('[data-testid="recent-empty"]')).not.toBeNull();
  });

  it("lists the five most recent projects compact under a heading (T3.16 in T3.17)", () => {
    const items = ["a", "b", "c", "d", "e", "f"].map((name) => ({
      path: `/work/${name}.dimo`,
      name: `${name}.dimo`,
      folder: "/work",
      exists: true,
    }));
    vi.spyOn(recentStore, "load").mockResolvedValue();
    recentStore.items = items;
    start();
    const slot = target.querySelector('[data-slot="recent-projects"]');
    expect(slot?.querySelector("h2")?.textContent).toBe("Recent projects");
    const rows = [...(slot?.querySelectorAll("[data-recent]") ?? [])];
    expect(rows.map((r) => r.getAttribute("data-recent"))).toEqual(
      items.slice(0, 5).map((i) => i.path),
    );
    // The recent list comes before the three steps, as in the mockup.
    const steps = target.querySelector("ol");
    expect(steps).not.toBeNull();
    expect(slot?.compareDocumentPosition(steps as Node)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });

  it("says good afternoon and good evening later in the day", () => {
    start(new Date(2026, 9, 13, 14, 0));
    expect(target.querySelector("h1")?.textContent).toBe("Good afternoon");
    void unmount(component ?? {});
    component = null;
    target.remove();
    start(new Date(2026, 9, 13, 19, 0));
    expect(target.querySelector("h1")?.textContent).toBe("Good evening");
  });

  it("runs New and Open through the command registry", () => {
    const create = vi.spyOn(projectStore, "newProject").mockResolvedValue(false);
    const open = vi.spyOn(projectStore, "open").mockResolvedValue();
    start();
    card("New project from drawing").click();
    card("Open project").click();
    expect(create).toHaveBeenCalledOnce();
    expect(open).toHaveBeenCalledOnce();
  });

  it("disables the cards with the reason while Dimo is busy", () => {
    const create = vi.spyOn(projectStore, "newProject").mockResolvedValue(false);
    projectStore.busy = true;
    start();
    const button = card("New project from drawing");
    expect(button.getAttribute("aria-disabled")).toBe("true");
    expect(button.title).toContain("Dimo is working, wait a moment.");
    button.click();
    expect(create).not.toHaveBeenCalled();
  });
});

describe("empty states (T3.17)", () => {
  it("shows what is missing and runs the command of its button", () => {
    projectStore.load(loaded(++session, emptyProject()));
    emptyState([{ command: "place_tool" }, { tab: "measure", label: "Go to Measure" }]);
    expect(target.querySelector("h2")?.textContent.trim()).toBe("Title");
    const [place, measure] = [...target.querySelectorAll<HTMLButtonElement>("button")];
    expect(place?.textContent.trim()).toBe("Place balloon");
    expect(place?.getAttribute("aria-pressed")).toBe("false");
    place?.click();
    flushSync();
    expect(balloonTools.tool).toBe("place");
    expect(place?.getAttribute("aria-pressed")).toBe("true");
    measure?.click();
    expect(view.tab).toBe("measure");
  });

  it("keeps a disabled command visible with its reason", () => {
    emptyState([{ command: "place_tool" }]);
    const button = target.querySelector<HTMLButtonElement>("button");
    expect(button?.getAttribute("aria-disabled")).toBe("true");
    expect(button?.title).toContain("Open or start a project first.");
    button?.click();
    expect(balloonTools.tool).toBe("select");
  });

  it("explains an empty selection in the side panel and offers Place balloon", () => {
    projectStore.load(loaded(++session, emptyProject()));
    render((t) => mount(SelectionSummary, { target: t }));
    expect(target.querySelector("h2")?.textContent.trim()).toBe("Nothing selected");
    expect(target.textContent).toContain("There are no balloons yet.");
    expect(target.querySelector<HTMLElement>("button")?.dataset.action).toBe("place_tool");
  });

  function reportEmpty(): { title: string; actions: string[] } | null {
    const box = target.querySelector('[data-testid="empty-state"]');
    return box
      ? {
          title: box.querySelector("h2")?.textContent.trim() ?? "",
          actions: [...box.querySelectorAll<HTMLElement>("button")].map(
            (b) => b.dataset.action ?? "",
          ),
        }
      : null;
  }

  it("guides File > Export from no project to the list exports (T3.19)", () => {
    render((t) => mount(ExportView, { target: t }));
    expect(reportEmpty()).toEqual({
      title: "No project open",
      actions: ["new_project", "open_project"],
    });
    projectStore.load(loaded(++session, emptyProject()));
    flushSync();
    expect(reportEmpty()).toEqual({ title: "Nothing to report yet", actions: ["place_tool"] });
  });

  it("guides the Report view from no project to measured values", () => {
    const store = new ReportViewStore(
      {
        reportPreview: () =>
          Promise.resolve({ status: "ok", data: { doc: "0", name: "p.pdf", sheets: [] } }),
        reportChecks: () => Promise.resolve({ status: "error", error: { kind: "no_project" } }),
        printReport: () => Promise.resolve({ status: "ok", data: "p.pdf" }),
      },
      0,
    );
    render((t) => mount(ReportView, { target: t, props: { store } }));
    expect(reportEmpty()).toEqual({
      title: "No project open",
      actions: ["new_project", "open_project"],
    });

    const project = emptyProject();
    projectStore.load(loaded(++session, project));
    flushSync();
    expect(reportEmpty()).toEqual({ title: "Nothing to report yet", actions: ["place_tool"] });

    project.characteristics = [characteristic("a", 1)];
    projectStore.load(loaded(++session, { ...project }));
    flushSync();
    expect(reportEmpty()).toEqual({ title: "No measured values yet", actions: ["tab-measure"] });

    projectStore.load(
      loaded(++session, {
        ...project,
        samples: [{ id: "s1", serial: "1", date: "", inspector: "", gauges: "", comment: "" }],
        results: [
          {
            sample: "s1",
            characteristic: "a",
            value: { type: "variable", value: "1" },
            source: { type: "manual" },
            comment: "",
          },
        ],
      }),
    );
    flushSync();
    expect(reportEmpty()).toBeNull();
  });
});
