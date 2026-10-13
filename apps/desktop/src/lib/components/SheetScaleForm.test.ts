// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import type { Scale, Sheet } from "$lib/ipc/bindings";
import { emptyProject } from "$lib/stores/fixtures";
import SheetScaleForm from "./SheetScaleForm.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
});

function render(sent: [keyof Scale, number][]) {
  const sheet = emptyProject().revisions[0]?.sheets[0] as Sheet;
  target = document.createElement("div");
  document.body.append(target);
  component = mount(SheetScaleForm, {
    target,
    props: {
      sheet: () => sheet,
      setPart: (part: keyof Scale, value: number) => {
        sent.push([part, value]);
        return Promise.resolve(undefined);
      },
    },
  });
  flushSync();
}

function type(index: number, text: string) {
  const input = target.querySelectorAll<HTMLInputElement>("input")[index];
  if (input) {
    input.value = text;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }
  flushSync();
}

describe("custom scale dialog (FR-DOC-05, T3.18)", () => {
  it("sends each changed part of the scale", () => {
    const sent: [keyof Scale, number][] = [];
    render(sent);
    type(0, "3");
    type(1, "7");
    expect(sent).toEqual([
      ["drawing", 3],
      ["actual", 7],
    ]);
  });

  it("refuses a part that is not a whole number and shows why", () => {
    const sent: [keyof Scale, number][] = [];
    render(sent);
    type(0, "0");
    expect(sent).toEqual([]);
    expect(target.querySelector("[role='alert']")?.textContent).toContain("whole numbers");
    expect(target.querySelectorAll<HTMLInputElement>("input")[0]?.value).toBe("1");
  });
});
