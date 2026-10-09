/**
 * Scripted UI steps for `tauri dev` (T1.6, `DIMO_DEV_UI_SCRIPT`). Development only, not a
 * feature: lets an agent drive the drawing with real DOM pointer and key events and time
 * window captures by the marks it writes to the terminal.
 *
 * The script is a JSON array of steps. Points are in sheet units and are converted to screen
 * positions with the current view, so the steps work at any zoom and rotation:
 *
 * ```json
 * [
 *   { "key": "b" },
 *   { "click": [420, 300] },
 *   { "type": "Ø8 f7" }, { "key": "Enter" },
 *   { "drag": [[400, 280], [480, 320]], "shift": true },
 *   { "button": "Flag" },
 *   { "view": [400, 420, 300] },
 *   { "wait": 500 }, { "mark": "placed" }, { "report": "after place" }
 * ]
 * ```
 *
 * - `key`: key down and up on the focused element (`mod`, `shift` for modifiers).
 * - `click`, `dblclick`, `drag`: pointer events on the drawing (`shift`, `mod`).
 * - `type`: sets the text of the focused field as typing would.
 * - `button`: clicks the button whose text is this label.
 * - `view`: zoom percent and the sheet point in the viewport center.
 * - `wait`: milliseconds; `mark` and `report` write a line (report: balloons and selection).
 */

import { commands } from "$lib/ipc/bindings";
import { isMacPlatform } from "$lib/shortcuts";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { viewport } from "$lib/stores/viewport.svelte";
import { ACTUAL_SIZE_SCALE, centerOn, sheetToScreen, type Point } from "$lib/viewport/view-math";

type Pair = [number, number];

interface Mods {
  shift?: boolean;
  mod?: boolean;
}

export type UiStep =
  | ({ key: string } & Mods)
  | ({ click: Pair } & Mods)
  | ({ dblclick: Pair } & Mods)
  | ({ drag: [Pair, Pair] } & Mods)
  | { type: string }
  | { button: string }
  | { view: [number, number, number] }
  | { wait: number }
  | { mark: string }
  | { report: string };

const STEP_PAUSE_MS = 120;
const DRAG_STEPS = 6;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function log(message: string): Promise<void> {
  return commands.devLog(message).catch(() => undefined);
}

function surface(): HTMLElement | null {
  return document.querySelector<HTMLElement>("main [role=application]");
}

function client(p: Pair): Point {
  const el = surface();
  const rect = el?.getBoundingClientRect();
  const screen = sheetToScreen(viewport.view, { x: p[0], y: p[1] });
  return { x: screen.x + (rect?.left ?? 0), y: screen.y + (rect?.top ?? 0) };
}

function modifiers(m: Mods) {
  const mac = isMacPlatform();
  return {
    shiftKey: m.shift ?? false,
    metaKey: mac && (m.mod ?? false),
    ctrlKey: !mac && (m.mod ?? false),
  };
}

function pointer(type: string, at: Point, m: Mods, buttons: number): void {
  surface()?.dispatchEvent(
    new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      button: 0,
      buttons,
      clientX: at.x,
      clientY: at.y,
      ...modifiers(m),
    }),
  );
}

async function press(from: Pair, to: Pair, m: Mods): Promise<void> {
  const a = client(from);
  const b = client(to);
  pointer("pointerdown", a, m, 1);
  for (let i = 1; i <= DRAG_STEPS && (a.x !== b.x || a.y !== b.y); i++) {
    const t = i / DRAG_STEPS;
    pointer("pointermove", { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }, m, 1);
    await sleep(16);
  }
  pointer("pointerup", b, m, 0);
  await sleep(STEP_PAUSE_MS);
}

function report(label: string): string {
  const project = projectStore.project;
  const balloons = (project?.balloons ?? []).map((b) => {
    const c = projectStore.characteristicById.get(b.characteristic);
    const shape = b.style.shape ?? "default";
    const at = `${b.position.x?.toFixed(1) ?? "?"},${b.position.y?.toFixed(1) ?? "?"}`;
    return `#${String(c?.number ?? "?")} ${shape} at ${at} "${c?.requirement_text ?? ""}"`;
  });
  const field = document.activeElement;
  const box = field?.getBoundingClientRect();
  const focus = field
    ? `${field.tagName.toLowerCase()}#${field.id} at ${String(Math.round(box?.x ?? 0))},${String(Math.round(box?.y ?? 0))}`
    : "none";
  const editor = document.getElementById("balloon-value") !== null;
  return `${label}: ${String(balloons.length)} balloons [${balloons.join("; ")}], selected ${String(selection.size)}, undo ${String(projectStore.canUndo)}, focus ${focus}, editor ${String(editor)} ${balloonTools.editing ?? "-"}, ${document.visibilityState}`;
}

async function run(step: UiStep): Promise<void> {
  if ("key" in step) {
    const target = document.activeElement ?? document.body;
    const init = { key: step.key, bubbles: true, cancelable: true, ...modifiers(step) };
    target.dispatchEvent(new KeyboardEvent("keydown", init));
    target.dispatchEvent(new KeyboardEvent("keyup", init));
    await sleep(STEP_PAUSE_MS);
  } else if ("click" in step) {
    await press(step.click, step.click, step);
  } else if ("dblclick" in step) {
    await press(step.dblclick, step.dblclick, step);
    await press(step.dblclick, step.dblclick, step);
  } else if ("drag" in step) {
    await press(step.drag[0], step.drag[1], step);
  } else if ("type" in step) {
    const field = document.activeElement;
    if (field instanceof HTMLInputElement) {
      field.value = step.type;
      field.dispatchEvent(new Event("input", { bubbles: true }));
    }
    await sleep(STEP_PAUSE_MS);
  } else if ("button" in step) {
    const button = [...document.querySelectorAll<HTMLElement>("button")].find(
      (b) => b.textContent.trim() === step.button,
    );
    button?.click();
    await sleep(STEP_PAUSE_MS);
  } else if ("view" in step) {
    const [percent, x, y] = step.view;
    viewport.set(
      centerOn(
        { x, y },
        (percent / 100) * ACTUAL_SIZE_SCALE,
        viewport.size,
        viewport.view.rotation,
      ),
    );
    await sleep(STEP_PAUSE_MS);
  } else if ("wait" in step) {
    await sleep(step.wait);
  } else if ("mark" in step) {
    await log(`mark ${step.mark}`);
  } else {
    await log(report(step.report));
  }
}

/** Plays a UI script; every step waits for the previous one. */
export async function runUiScript(text: string): Promise<void> {
  let steps: UiStep[];
  try {
    steps = JSON.parse(text) as UiStep[];
  } catch (e) {
    await log(`script is not valid JSON: ${String(e)}`);
    return;
  }
  await log(`script start, ${String(steps.length)} steps`);
  for (const [index, step] of steps.entries()) {
    try {
      await run(step);
    } catch (e) {
      await log(`step ${String(index + 1)} failed: ${String(e)}`);
    }
  }
  await log("script done");
}
