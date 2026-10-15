// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { m } from "$lib/i18n";
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
  setup.api.deleteResults = resultCount;
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
    expect(document.querySelector("[data-testid=delete-dialog]")).not.toBeNull();
  });
  return setup;
}

function answer(name: "delete"): HTMLButtonElement {
  const found = document.body.querySelector<HTMLButtonElement>(`button[data-answer=${name}]`);
  if (!found) {
    throw new Error(`no button ${name}`);
  }
  return found;
}

/** The cancel button of the dialog, labelled with the message. */
function keep(): HTMLButtonElement {
  const found = [...document.body.querySelectorAll("button")].find(
    (b) => b.textContent.trim() === m.delete_results_no(),
  );
  if (!found) {
    throw new Error("no keep button");
  }
  return found;
}

describe("delete characteristics dialog (T3.9)", () => {
  it("says how many results go and that undo restores them, then deletes on confirm", async () => {
    const { api } = await render(3);
    expect(document.body.textContent).toContain(m.delete_results_body_other({ count: "3" }));
    answer("delete").click();
    await tick();
    expect(api.commands).toEqual([{ type: "delete_characteristics", ids: ["a"] }]);
  });

  it("uses the singular for one result and keeps everything on Keep", async () => {
    const { api, tools } = await render(1);
    expect(document.body.textContent).toContain(m.delete_results_body_one());
    keep().click();
    await tick();
    expect(api.commands).toEqual([]);
    expect(tools.pendingDelete).toBeNull();
  });
});
