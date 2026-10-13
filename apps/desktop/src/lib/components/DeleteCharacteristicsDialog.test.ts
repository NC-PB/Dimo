// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { loaded } from "$lib/stores/fixtures";
import { twoBalloons } from "$lib/viewport/balloon-fixtures";
import DeleteCharacteristicsDialog from "./DeleteCharacteristicsDialog.svelte";

let component: ReturnType<typeof mount> | null = null;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  document.body.innerHTML = "";
});

async function render(resultCount: number) {
  const setup = twoBalloons();
  const data = setup.project.project;
  if (data === null) {
    throw new Error("no project");
  }
  setup.project.load(
    loaded(1, {
      ...data,
      results: Array.from({ length: resultCount }, (_, i) => ({
        sample: `s${String(i)}`,
        characteristic: "a",
        value: { type: "variable" as const, value: "1" },
        source: { type: "manual" as const },
        comment: "",
      })),
    }),
    true,
  );
  component = mount(DeleteCharacteristicsDialog, {
    target: document.body,
    props: { tools: setup.tools },
  });
  flushSync();
  await tick();
  setup.selection.select(["a"]);
  await setup.tools.deleteSelection();
  flushSync();
  await vi.waitFor(() => {
    expect(document.body.textContent).toContain("result");
  });
  return setup;
}

function button(text: string): HTMLButtonElement {
  const found = [...document.body.querySelectorAll("button")].find(
    (b) => b.textContent.trim() === text,
  );
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

describe("delete characteristics dialog (T3.9)", () => {
  it("says how many results go and that undo restores them, then deletes on confirm", async () => {
    const { api } = await render(3);
    expect(document.body.textContent).toContain("3 results will be removed");
    expect(document.body.textContent).toContain("Undo brings them back");
    button("Delete with results").click();
    await tick();
    expect(api.commands).toEqual([{ type: "delete_characteristics", ids: ["a"] }]);
  });

  it("uses the singular for one result and keeps everything on Keep", async () => {
    const { api, tools } = await render(1);
    expect(document.body.textContent).toContain("1 result will be removed");
    button("Keep").click();
    await tick();
    expect(api.commands).toEqual([]);
    expect(tools.pendingDelete).toBeNull();
  });
});
