// @vitest-environment happy-dom
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import BalloonTools from "./BalloonTools.svelte";

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;

beforeEach(() => {
  documentStore.show({
    doc: "0".repeat(64),
    name: "part.pdf",
    sheets: [{ width: 842, height: 595 }],
  });
  balloonTools.setTool("select");
  target = document.createElement("div");
  document.body.append(target);
  component = mount(BalloonTools, { target });
  flushSync();
});

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target.remove();
  documentStore.show(null);
  balloonTools.setTool("select");
});

function item(text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
  if (!found) {
    throw new Error(`no button ${text}`);
  }
  return found;
}

function click(button: HTMLButtonElement): void {
  button.click();
  flushSync();
}

describe("balloon tool toggle (T3.13, M3 decision 19)", () => {
  it("clicking the active Place button returns to Select", () => {
    click(item("Place balloon"));
    expect(balloonTools.tool).toBe("place");
    expect(item("Place balloon").dataset.state).toBe("on");

    click(item("Place balloon"));
    expect(balloonTools.tool).toBe("select");
    expect(item("Place balloon").dataset.state).toBe("off");
    expect(item("Select").dataset.state).toBe("on");
  });

  it("clicking the active Select button keeps Select on", () => {
    click(item("Select"));
    expect(balloonTools.tool).toBe("select");
    expect(item("Select").dataset.state).toBe("on");
  });

  it("follows the store when the tool changes elsewhere (Esc, shortcut)", () => {
    balloonTools.setTool("place");
    flushSync();
    expect(item("Place balloon").dataset.state).toBe("on");
    balloonTools.setTool("select");
    flushSync();
    expect(item("Select").dataset.state).toBe("on");
    expect(item("Place balloon").dataset.state).toBe("off");
  });
});
