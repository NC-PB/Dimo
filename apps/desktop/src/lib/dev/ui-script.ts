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
 * - `key`: key down and up on the focused element (`mod`, `shift`, `alt` for modifiers).
 * - `click`, `dblclick`, `drag`: pointer events on the drawing (`shift`, `mod`).
 * - `type`: sets the text of the focused field as typing would.
 * - `button`: clicks the button whose text is this label.
 * - `balloon`: clicks the balloon with this number (`shift`, `mod`).
 * - `row`: clicks the requirement cell of the table row with this balloon number (`shift`, `mod`).
 * - `view`: zoom percent and the sheet point in the viewport center.
 * - `show`: switches to a view (`drawing`, `export`, `settings`, ...).
 * - `select`: `[label, value]` sets the `<select>` inside the label that starts with this text,
 *   or the `<select>` whose `aria-label` starts with it.
 * - `reveal`: scrolls the field inside the label that starts with this text into view.
 * - `check`: `[label, checked]` sets the check box (or turns on the radio button) inside the
 *   label that starts with this text.
 * - `wait`: milliseconds; `mark` and `report` write a line (report: balloons and selection).
 */

import { commands, type ExportFormat } from "$lib/ipc/bindings";
import { isMacPlatform } from "$lib/shortcuts";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import { EXPORT_FORMATS, exportStore } from "$lib/stores/export.svelte";
import { numberingStore } from "$lib/stores/numbering.svelte";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { view } from "$lib/stores/view.svelte";
import { viewport } from "$lib/stores/viewport.svelte";
import { ACTUAL_SIZE_SCALE, centerOn, sheetToScreen, type Point } from "$lib/viewport/view-math";

type Pair = [number, number];

interface Mods {
  shift?: boolean;
  mod?: boolean;
  alt?: boolean;
}

export type UiStep =
  | ({ key: string } & Mods)
  | ({ click: Pair } & Mods)
  | ({ dblclick: Pair } & Mods)
  | ({ drag: [Pair, Pair] } & Mods)
  | { type: string }
  | { button: string }
  | ({ row: number } & Mods)
  | ({ balloon: number } & Mods)
  | { view: [number, number, number] }
  | { show: string }
  | { select: [string, string] }
  | { check: [string, boolean] }
  | { reveal: string }
  | { wait: number }
  | { mark: string }
  | { report: string };

const STEP_PAUSE_MS = 120;
const DRAG_STEPS = 6;
/** Longest wait for a text field to take the focus before a `type` step. */
const FIELD_WAIT_MS = 3000;

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
    altKey: m.alt ?? false,
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

/** The label element whose text starts with `text`. */
function labelled(text: string): HTMLLabelElement | undefined {
  return [...document.querySelectorAll("label")].find((l) => l.textContent.trim().startsWith(text));
}

function exportState(format: ExportFormat): string {
  const job = exportStore.job(format);
  switch (job?.state) {
    case undefined:
      return `${format} -`;
    case "running":
      return `${format} running ${String(job.fraction)}`;
    case "done":
      return `${format} done ${job.fileName}`;
    case "failed":
      return `${format} failed ${job.error.kind}`;
  }
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
  const chosen = [...selection.ids]
    .map((id) => String(projectStore.characteristicById.get(id)?.number ?? "?"))
    .join(",");
  const rows = [...document.querySelectorAll('[role="row"][aria-selected="true"]')]
    .map((r) => String(Number(r.getAttribute("aria-rowindex")) - 1))
    .join(",");
  const exports = EXPORT_FORMATS.map(exportState).join(", ");
  const theme = document.documentElement.dataset.theme ?? "-";
  const ghosts = Object.entries(numberingStore.ghosts)
    .map(
      ([id, n]) => `${String(projectStore.characteristicById.get(id)?.number ?? "?")}>${n ?? "?"}`,
    )
    .join(" ");
  const sheet = projectStore.sheets[documentStore.sheet];
  const grid = sheet?.zone_grid;
  const zones = grid
    ? `${String(grid.column_labels.length)}x${String(grid.row_labels.length)}`
    : "none";
  return `${label}: view ${view.current}, theme ${theme}, lang ${document.documentElement.lang}, exports [${exports}], locked ${String(project?.numbering.lock?.reason ?? "no")}, ${String(balloons.length)} balloons [${balloons.join("; ")}], selected ${String(selection.size)} [${chosen}], table rows [${rows}], rotation ${String(viewport.view.rotation)}, undo ${String(projectStore.canUndo)}, restored ${String(projectStore.notice?.restored_unsaved ?? false)}, focus ${focus}, editor ${String(editor)} ${balloonTools.editing ?? "-"}, ${document.visibilityState}, numbering ${projectStore.project?.settings.numbering.strategy ?? "-"} ghosts [${ghosts}] zones ${zones} views ${String(sheet?.views.length ?? 0)}`;
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
    // The field opens after the command came back from Rust; wait for it instead of typing
    // into nothing on a slow machine.
    for (let waited = 0; waited < FIELD_WAIT_MS; waited += 20) {
      if (document.activeElement instanceof HTMLInputElement) {
        break;
      }
      await sleep(20);
    }
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
  } else if ("balloon" in step) {
    const b = projectStore.project?.balloons.find(
      (x) => projectStore.characteristicById.get(x.characteristic)?.number === String(step.balloon),
    );
    if (b) {
      const at: Pair = [b.position.x ?? 0, b.position.y ?? 0];
      await press(at, at, step);
    }
  } else if ("row" in step) {
    // Rows are in number order: number n is row index n + 1 (the header is row 1).
    const cell = document.querySelector<HTMLElement>(
      `[role="row"][aria-rowindex="${String(step.row + 1)}"] [data-col="2"]`,
    );
    const box = cell?.getBoundingClientRect();
    cell?.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        pointerId: 2,
        pointerType: "mouse",
        isPrimary: true,
        button: 0,
        buttons: 1,
        clientX: (box?.left ?? 0) + 4,
        clientY: (box?.top ?? 0) + 4,
        ...modifiers(step),
      }),
    );
    // A real click focuses the grid; a synthetic event does not, and the table reads keys only
    // from the focused grid.
    document.querySelector<HTMLElement>('[role="grid"]')?.focus({ preventScroll: true });
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
  } else if ("show" in step) {
    view.set(step.show);
    await sleep(STEP_PAUSE_MS);
  } else if ("select" in step) {
    const field =
      labelled(step.select[0])?.querySelector("select") ??
      [...document.querySelectorAll("select")].find((f) =>
        (f.getAttribute("aria-label") ?? "").startsWith(step.select[0]),
      );
    if (field) {
      field.value = step.select[1];
      field.dispatchEvent(new Event("change", { bubbles: true }));
    }
    await sleep(STEP_PAUSE_MS);
  } else if ("reveal" in step) {
    labelled(step.reveal)?.scrollIntoView({ block: "center" });
    await sleep(STEP_PAUSE_MS);
  } else if ("check" in step) {
    const box = labelled(step.check[0])?.querySelector<HTMLInputElement>(
      "input[type=checkbox], input[type=radio]",
    );
    if (box && box.checked !== step.check[1]) {
      box.click();
    }
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
