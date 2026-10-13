// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Change, Command, CommandError, Project, ProjectPatched } from "$lib/ipc/bindings";
import { REVISION_ID, emptyProject, loaded, status } from "$lib/stores/fixtures";
import { ProjectStore, type ProjectCommands } from "$lib/stores/project.svelte";
import ProjectDetails from "./ProjectDetails.svelte";

/** The change Rust would report for a command of the panel. */
function changeFor(project: Project, command: Command): Change[] {
  switch (command.type) {
    case "update_project_info":
      return [{ type: "info_changed", before: project.info, after: command.info }];
    case "update_report_info":
      return [{ type: "report_info_changed", before: project.report_info, after: command.info }];
    case "update_revision_label":
      return [
        {
          type: "revision_label_changed",
          revision: command.revision,
          before: "",
          after: command.label.trim(),
        },
      ];
    default:
      return [];
  }
}

/** A store over a fake Rust that records commands, applies them and can refuse them. */
function setup(project: Project = emptyProject()) {
  const commands: Command[] = [];
  let revision = 0;
  const refuse: { error: CommandError | null } = { error: null };
  const patched = (changes: Change[]): ProjectPatched => {
    if (changes.length > 0) {
      revision += 1;
    }
    return { session: 1, revision, patch: { changes }, status: status() };
  };
  const api: ProjectCommands = {
    projectState: () => Promise.resolve({ status: "ok", data: loaded(1, project) }),
    newProject: () => Promise.resolve({ status: "ok", data: true }),
    openProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProject: () => Promise.resolve({ status: "ok", data: true }),
    saveProjectAs: () => Promise.resolve({ status: "ok", data: true }),
    confirmClose: () => Promise.resolve({ status: "ok", data: null }),
    closeProject: () => Promise.resolve({ status: "ok", data: null }),
    openRecentProject: () => Promise.resolve({ status: "ok", data: true }),
    execute: (command) => {
      commands.push(command);
      if (refuse.error) {
        return Promise.resolve({ status: "error", error: refuse.error });
      }
      const current = projectStore.project ?? project;
      return Promise.resolve({ status: "ok", data: patched(changeFor(current, command)) });
    },
    undo: () => Promise.reject(new Error("not used")),
    redo: () => Promise.reject(new Error("not used")),
  };
  const projectStore = new ProjectStore(api);
  projectStore.load(loaded(1, project));
  return { projectStore, commands, refuse };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

function render(stores: ReturnType<typeof setup>) {
  target = document.createElement("div");
  document.body.append(target);
  component = mount(ProjectDetails, { target, props: { projectStore: stores.projectStore } });
  flushSync();
}

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  vi.restoreAllMocks();
});

async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) {
    await tick();
    await Promise.resolve();
  }
  flushSync();
}

/** The input or textarea that belongs to the label text. */
function field(label: string): HTMLInputElement | HTMLTextAreaElement {
  const found = [...target.querySelectorAll("label")].find((l) =>
    l.textContent.trim().startsWith(label),
  );
  const control = found?.querySelector<HTMLInputElement | HTMLTextAreaElement>("input, textarea");
  if (!control) {
    throw new Error(`no field ${label}`);
  }
  return control;
}

async function type(label: string, text: string): Promise<void> {
  const control = field(label);
  control.value = text;
  control.dispatchEvent(new Event("change", { bubbles: true }));
  await settle();
}

describe("project and report details (T3.4)", () => {
  it("edits part data as update_project_info commands", async () => {
    const stores = setup();
    render(stores);
    await type("Part number", "4711");
    await type("Customer", "Muster AG");
    expect(stores.commands).toEqual([
      {
        type: "update_project_info",
        info: { part_number: "4711", part_name: "", customer: "", order_reference: "" },
      },
      {
        type: "update_project_info",
        info: { part_number: "4711", part_name: "", customer: "Muster AG", order_reference: "" },
      },
    ]);
    expect(stores.projectStore.project?.info.customer).toBe("Muster AG");
    expect(field("Part number").value).toBe("4711");
  });

  it("edits the revision label of the current drawing and shows it trimmed", async () => {
    const stores = setup();
    render(stores);
    await type("Revision", " B ");
    expect(stores.commands).toEqual([
      { type: "update_revision_label", revision: REVISION_ID, label: " B " },
    ]);
    expect(stores.projectStore.project?.revisions[0]?.label).toBe("B");
    expect(field("Revision").value).toBe("B");
  });

  it("edits the report fields, remarks included, as update_report_info commands", async () => {
    const stores = setup();
    render(stores);
    await type("Report number", "R-7");
    await type("Inspected by", "Anna");
    await type("Approved by", "Ben");
    await type("Remarks", "Two lines\nof text");
    const last = stores.commands.at(-1);
    expect(last).toEqual({
      type: "update_report_info",
      info: {
        report_number: "R-7",
        report_date: "",
        company_name: "",
        inspected_by: "Anna",
        approved_by: "Ben",
        remarks: "Two lines\nof text",
      },
    });
    expect(stores.commands).toHaveLength(4);
  });

  it("sends nothing when a value did not change", async () => {
    const stores = setup();
    render(stores);
    await type("Part number", "");
    expect(stores.commands).toEqual([]);
  });

  it("sets the report date from a date input and from the today button", async () => {
    const stores = setup();
    render(stores);
    const date = target.querySelector<HTMLInputElement>("input[type=date]");
    expect(date).not.toBeNull();
    if (date) {
      date.value = "2026-10-12";
      date.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await settle();
    expect(stores.projectStore.project?.report_info.report_date).toBe("2026-10-12");

    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(2026, 0, 5, 23, 30));
    target
      .querySelector<HTMLButtonElement>("button[aria-label='Set the report date to today']")
      ?.click();
    await settle();
    vi.useRealTimers();
    expect(stores.projectStore.project?.report_info.report_date).toBe("2026-01-05");
  });

  it("shows the stored value again when Rust refuses a change", async () => {
    const stores = setup();
    render(stores);
    stores.refuse.error = { kind: "rejected", reason: "invalid_date", message: "bad" };
    await type("Report number", "R-1");
    expect(field("Report number").value).toBe("");
    expect(stores.projectStore.error?.kind).toBe("rejected");
  });

  it("shows file name and hash read only and copies the hash", async () => {
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: (text: string) => {
          written.push(text);
          return Promise.resolve();
        },
      },
    });
    const stores = setup();
    render(stores);
    expect(target.querySelector("[data-testid=drawing-file]")?.textContent).toBe("part.pdf");
    const sha = target.querySelector("[data-testid=drawing-sha]")?.textContent;
    expect(sha).toBe("a".repeat(64));
    const copy = target.querySelector<HTMLButtonElement>("button[aria-label^='Copy']");
    copy?.click();
    await settle();
    expect(written).toEqual(["a".repeat(64)]);
    expect(copy?.textContent.trim()).toBe("Copied");
    expect(stores.commands).toEqual([]);
  });

  it("runs no command when it is shown", async () => {
    const stores = setup();
    render(stores);
    await settle();
    expect(stores.commands).toEqual([]);
    expect(stores.projectStore.project?.report_info.company_name).toBe("");
  });
});
