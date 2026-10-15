// @vitest-environment happy-dom
import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import DeleteCharacteristicsDialog from "$lib/components/DeleteCharacteristicsDialog.svelte";
import Shortcuts from "$lib/components/Shortcuts.svelte";
import UnsavedChangesDialog from "$lib/components/UnsavedChangesDialog.svelte";
import ProjectSettingsDialog from "$lib/shell/ProjectSettingsDialog.svelte";
import { balloonTools } from "./balloon-tools.svelte";
import { characteristic, emptyProject, loaded } from "./fixtures";
import { KeyboardStore, keyboard } from "./keyboard.svelte";
import { projectStore } from "./project.svelte";
import { unsavedPrompt } from "./prompt.svelte";
import { view } from "./view.svelte";

const key = (k: string, mods: Partial<{ meta: boolean; shift: boolean }> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: mods.meta ?? false,
  altKey: false,
  shiftKey: mods.shift ?? false,
});

/** A small window: a drawing area with a surface, a table, a ribbon button and a text field. */
function layout() {
  const root = document.createElement("div");
  root.innerHTML = `
    <main data-area="drawing"><div tabindex="0" data-surface></div></main>
    <div role="grid" tabindex="0" data-area="table"><span data-cell></span></div>
    <button type="button" data-ribbon>Zoom</button>
    <input type="text" data-field />`;
  document.body.append(root);
  const find = (selector: string) => {
    const found = root.querySelector<HTMLElement>(selector);
    if (found === null) {
      throw new Error(selector);
    }
    return found;
  };
  return { root, find };
}

describe("keyboard scopes and dialogs (MF F8, D-52)", () => {
  let store: KeyboardStore;
  let dom: ReturnType<typeof layout>;
  let drawingShown: boolean;

  beforeEach(() => {
    store = new KeyboardStore();
    dom = layout();
    drawingShown = true;
    store.register({
      scope: "drawing",
      element: () => dom.find("[data-area=drawing]"),
      shown: () => drawingShown,
    });
    store.register({ scope: "table", element: () => dom.find("[data-area=table]") });
  });

  afterEach(() => {
    dom.root.remove();
  });

  const action = (k: ReturnType<typeof key>, target: EventTarget | null) =>
    store.shortcutFor(k, target, true)?.action ?? null;

  it("runs drawing keys while the drawing is shown, from anywhere but the table", () => {
    expect(action(key("r"), document.body)).toBe("rotate_right");
    expect(action(key("r"), dom.find("[data-surface]"))).toBe("rotate_right");
    expect(action(key("r"), dom.find("[data-ribbon]"))).toBe("rotate_right");
    expect(action(key("r"), dom.find("[data-cell]"))).toBeNull();
    expect(action(key("r"), dom.find("[data-field]"))).toBeNull();
  });

  it("switches scope with the shown area: app keys stay, drawing keys stop", () => {
    drawingShown = false;
    expect(action(key("r"), document.body)).toBeNull();
    expect(action(key("b"), document.body)).toBeNull();
    expect(action(key("s", { meta: true }), document.body)).toBe("save_project");
    expect(action(key("?", { shift: true }), document.body)).toBe("show_shortcuts");
    drawingShown = true;
    expect(action(key("b"), document.body)).toBe("place_tool");
  });

  it("lets a focused table or grid keep its keys; only app keys act from it", () => {
    expect(store.scopesAt(dom.find("[data-cell]")).scopes).toEqual(["app"]);
    expect(action(key("s", { meta: true }), dom.find("[data-cell]"))).toBe("save_project");
    expect(action(key("ArrowUp"), dom.find("[data-cell]"))).toBeNull();
    expect(action(key("?", { shift: true }), dom.find("[data-cell]"))).toBe("show_shortcuts");
  });

  it("keeps focus only keys for the drawing or no focus (was viewportOnly)", () => {
    expect(action(key("ArrowLeft"), dom.find("[data-surface]"))).toBe("pan_left");
    expect(action(key("ArrowLeft"), document.body)).toBe("pan_left");
    expect(action(key("ArrowLeft"), dom.find("[data-ribbon]"))).toBeNull();
    expect(action(key("Delete"), dom.find("[data-ribbon]"))).toBeNull();
  });

  it("an unregistered area no longer has a scope", () => {
    const other = new KeyboardStore();
    const unregister = other.register({ scope: "imageEditor", element: () => null });
    expect(other.scopesAt(document.body).scopes).toEqual(["app", "imageEditor"]);
    unregister();
    expect(other.scopesAt(document.body).scopes).toEqual(["app"]);
  });

  it("blocks every window shortcut while a dialog is open", () => {
    const close = store.openModal();
    expect(action(key("r"), document.body)).toBeNull();
    expect(action(key("s", { meta: true }), document.body)).toBeNull();
    const second = store.openModal();
    close();
    close(); // A second call changes nothing.
    expect(store.modals).toBe(1);
    expect(action(key("r"), document.body)).toBeNull();
    second();
    expect(action(key("r"), document.body)).toBe("rotate_right");
  });
});

describe("dialogs block drawing shortcuts in the app (MF F8)", () => {
  let target: HTMLElement;
  const mounted: ReturnType<typeof mount>[] = [];
  let unregister: () => void = () => undefined;
  let session = 5000;

  beforeEach(() => {
    const project = emptyProject();
    project.characteristics = [characteristic("a", 1)];
    projectStore.load(loaded(++session, project));
    view.setTab("home");
    balloonTools.setTool("select");
    target = document.createElement("div");
    document.body.append(target);
    unregister = keyboard.register({ scope: "drawing", element: () => null, shown: () => true });
    for (const component of [
      Shortcuts,
      ProjectSettingsDialog,
      UnsavedChangesDialog,
      DeleteCharacteristicsDialog,
    ]) {
      mounted.push(mount(component, { target }));
    }
    flushSync();
  });

  afterEach(() => {
    for (const component of mounted.splice(0)) {
      void unmount(component);
    }
    unregister();
    target.remove();
    view.projectDialog = null;
    view.shortcutsOpen = false;
    balloonTools.cancelDelete();
    if (unsavedPrompt.open) {
      unsavedPrompt.answer("cancel");
    }
    projectStore.load(loaded(++session, null));
    balloonTools.setTool("select");
  });

  /** Presses B (place balloon) on the page and returns the tool afterwards. */
  async function pressB(): Promise<string> {
    document.body.dispatchEvent(
      new KeyboardEvent("keydown", { key: "b", bubbles: true, cancelable: true }),
    );
    await tick();
    const tool = balloonTools.tool;
    balloonTools.setTool("select");
    return tool;
  }

  const dialogs: [string, () => void][] = [
    [
      "project settings",
      () => {
        view.projectDialog = "numbering";
      },
    ],
    [
      "cheat sheet",
      () => {
        view.shortcutsOpen = true;
      },
    ],
    [
      "unsaved changes",
      () => {
        void unsavedPrompt.ask();
      },
    ],
    [
      "delete with results",
      () => {
        balloonTools.pendingDelete = { ids: ["a"], results: 2 };
      },
    ],
  ];

  it.each(dialogs)("the %s dialog stops B from switching the tool", async (_, open) => {
    expect(await pressB()).toBe("place");
    open();
    flushSync();
    await tick();
    expect(keyboard.modals).toBe(1);
    expect(await pressB()).toBe("select");
  });

  it("gives the keys back when the dialog closes", async () => {
    view.projectDialog = "numbering";
    flushSync();
    await tick();
    expect(await pressB()).toBe("select");
    view.projectDialog = null;
    flushSync();
    await tick();
    expect(keyboard.modals).toBe(0);
    expect(await pressB()).toBe("place");
  });
});
