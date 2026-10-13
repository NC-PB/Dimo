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
 * - `click`, `dblclick`, `drag`: pointer events on the drawing (`shift`, `mod`). With
 *   `"pause": false` the next step starts right after the pointer is released; otherwise a
 *   dragged box has been read (its card is open) before the next step. The script's pointer
 *   has its own ID, so the real mouse moving over the window does not change its gestures.
 * - `type`: waits for a text field to take the focus, then sets its text as typing would.
 * - `keys`: types the characters one by one as key events on whatever has the focus, without
 *   waiting for a field (`\n` is Enter). In a text field each character is inserted as a real
 *   key would; on the drawing the key goes to its key handling (T2.7a, typing right after a
 *   placing click).
 * - `button`: clicks the button whose text is this label (or, for icon buttons, whose
 *   `aria-label` is, or whose `aria-labelledby` element has this text, like the start screen
 *   cards).
 * - `balloon`: clicks the balloon with this number (`shift`, `mod`).
 * - `row`: clicks the requirement cell of the table row with this balloon number (`shift`, `mod`).
 * - `cell`: `[row, sample]` clicks a cell of the measurement grid, both one based (row 1 is the
 *   first measured characteristic), and gives the grid the focus (T3.5).
 * - `paste`: sends a paste event with this text to the focused element, as Cmd/Ctrl+V with that
 *   text on the clipboard would (T3.5).
 * - `view`: zoom percent and the sheet point in the viewport center.
 * - `show`: switches to a work area (`drawing`, `measure`, `export`, `settings`, ...) or a
 *   ribbon tab (`home`, `report`, `view`, ...).
 * - `select`: `[label, value]` sets the `<select>` inside the label that starts with this text,
 *   or the `<select>` whose `aria-label` starts with it.
 * - `reveal`: scrolls the field inside the label that starts with this text into view.
 * - `fill`: `[label, text]` sets the text, number or date field inside the label that starts with this text, as
 *   typing and leaving the field would (input and change events), e.g. the shorter leg (T2.9a).
 * - `check`: `[label, checked]` sets the check box (or turns on the radio button) inside the
 *   label that starts with this text.
 * - `wait`: milliseconds; `mark` and `report` write a line (report: balloons, selection, tool
 *   and limits).
 */

import { commands, type ExportFormat } from "$lib/ipc/bindings";
import { isMacPlatform } from "$lib/shortcuts";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { boxSelect } from "$lib/stores/box-select.svelte";
import { devTools } from "./dev-tools.svelte";
import { measurementStore } from "$lib/stores/measurement.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import { EXPORT_FORMATS, REPORT_FORMATS, exportStore } from "$lib/stores/export.svelte";
import { numberingStore } from "$lib/stores/numbering.svelte";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { isView, view } from "$lib/stores/view.svelte";
import { viewport } from "$lib/stores/viewport.svelte";
import { ACTUAL_SIZE_SCALE, centerOn, sheetToScreen, type Point } from "$lib/viewport/view-math";

type Pair = [number, number];

interface Mods {
  shift?: boolean;
  mod?: boolean;
  alt?: boolean;
}

/** Pointer steps: whether to pause after the release (default true). */
interface Pause {
  pause?: boolean;
}

export type UiStep =
  | ({ key: string } & Mods)
  | ({ click: Pair } & Mods & Pause)
  | ({ dblclick: Pair } & Mods)
  | ({ drag: [Pair, Pair] } & Mods & Pause)
  | { type: string }
  | { keys: string }
  | { button: string }
  | ({ row: number } & Mods)
  | { cell: [number, number] }
  | { paste: string }
  | ({ balloon: number } & Mods)
  | { view: [number, number, number] }
  | { show: string }
  | { select: [string, string] }
  | { check: [string, boolean] }
  | { reveal: string }
  | { fill: [string, string] }
  | { wait: number }
  | { mark: string }
  | { report: string };

const STEP_PAUSE_MS = 120;
/** Time between two keys of a `keys` step, a fast typist. */
const KEY_GAP_MS = 8;
const DRAG_STEPS = 6;
/**
 * Pointer ID of the script's drawing events. Not 1, the ID of the real mouse in WebKit: the real
 * mouse over the window sends trusted moves with ID 1, and a gesture would take them as its own
 * and end a dragged box at the cursor instead of the scripted point (smoke flake, 2026-10-13).
 * A gesture follows only the pointer that started it, so these events and the real mouse never
 * mix.
 */
const SCRIPT_POINTER_ID = 41;
/** Longest wait for a text field to take the focus before a `type` step. */
const FIELD_WAIT_MS = 3000;
/** Longest wait for Rust to read a dragged box before the next step. */
const BOX_WAIT_MS = 10000;

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
      pointerId: SCRIPT_POINTER_ID,
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

async function press(from: Pair, to: Pair, m: Mods & Pause): Promise<void> {
  const a = client(from);
  const b = client(to);
  pointer("pointerdown", a, m, 1);
  for (let i = 1; i <= DRAG_STEPS && (a.x !== b.x || a.y !== b.y); i++) {
    const t = i / DRAG_STEPS;
    pointer("pointermove", { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }, m, 1);
    await sleep(16);
  }
  pointer("pointerup", b, m, 0);
  if (m.pause === false) {
    return;
  }
  // A dragged box is read by Rust before the card opens (box select, T2.6); the release starts
  // that read synchronously. Wait for it, so the next step sees the card on a slow machine.
  for (let waited = 0; boxSelect.busy && waited < BOX_WAIT_MS; waited += 20) {
    await sleep(20);
  }
  await sleep(STEP_PAUSE_MS);
}

/**
 * Types `text` key by key on the focused element. Synthetic key events do not insert text, so a
 * character that reaches a text field unhandled is inserted here, as the browser would.
 */
async function typeKeys(text: string): Promise<void> {
  for (const ch of text) {
    const key = ch === "\n" ? "Enter" : ch;
    const target = document.activeElement ?? document.body;
    const init = { key, bubbles: true, cancelable: true };
    const unhandled = target.dispatchEvent(new KeyboardEvent("keydown", init));
    if (unhandled && key !== "Enter" && target instanceof HTMLInputElement) {
      const end = target.value.length;
      target.setRangeText(ch, target.selectionStart ?? end, target.selectionEnd ?? end, "end");
      target.dispatchEvent(new Event("input", { bubbles: true }));
    }
    target.dispatchEvent(new KeyboardEvent("keyup", init));
    await sleep(KEY_GAP_MS);
  }
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
      return `${format} done ${job.fileName}${job.issued ? ` issued ${job.issued.state}` : ""}`;
    case "failed":
      return `${format} failed ${job.error.kind}`;
    case "cancelled":
      return `${format} cancelled`;
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
  // Report exports (T3.8) are listed only once they ran, so the list of the M1 flow stays as is.
  const exports = [...EXPORT_FORMATS, ...REPORT_FORMATS.filter((f) => exportStore.job(f) !== null)]
    .map(exportState)
    .join(", ");
  const theme = document.documentElement.dataset.theme ?? "-";
  // The open proposal card of box select (T2.6): text, limits and rule of each proposal.
  const card = boxSelect.proposals
    .map(
      (p) =>
        `"${p.requirement_text}" ${p.upper_limit ?? "-"}/${p.lower_limit ?? "-"} ${p.derivation?.rule.rule ?? "none"}${p.shorter_leg ? ` leg ${p.shorter_leg.value}` : ""}`,
    )
    .join("; ");
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
  // Measurement grid (T3.5): verdicts per measured row as one letter per sample.
  const letter = { pass: "p", fail: "f", not_evaluated: "n", missing: "-" } as const;
  const verdicts = measurementStore.rows
    .map(
      (r) =>
        `${projectStore.characteristicById.get(r.characteristic)?.number ?? "?"}:${r.cells.map((v) => letter[v]).join("")}`,
    )
    .join(" ");
  const times = devTools.commitTimes;
  const commits =
    times.length === 0
      ? "-"
      : `${String(times.length)} mean ${(times.reduce((a, b) => a + b, 0) / times.length).toFixed(1)} ms max ${Math.max(...times).toFixed(1)} ms`;
  // The active tool and the tool button shown as on (T3.13), and the limits and rule of each
  // characteristic in number order (M3 decision 22).
  const toolOn = [...document.querySelectorAll('[data-state="on"]')]
    .filter((e) => e.getAttribute("role") === "radio" || e.hasAttribute("data-toggle-group-item"))
    .map((e) => e.textContent.trim())
    .join(",");
  const limits = [...(project?.characteristics ?? [])]
    .map(
      (c) =>
        `#${String(c.number)} ${c.upper_limit ?? "-"}/${c.lower_limit ?? "-"} ${c.derivation?.rule.rule ?? "none"}`,
    )
    .join("; ");
  const measure = `samples ${String(project?.samples.length ?? 0)} results ${String(project?.results.length ?? 0)} verdicts [${verdicts}] overall ${measurementStore.state?.verdict ?? "-"} commits ${commits}`;
  // The box the open card was read from, in sheet space: shows where a box select landed.
  const r = boxSelect.region;
  const region = r
    ? ` region ${r.x.toFixed(1)},${r.y.toFixed(1)} ${r.width.toFixed(1)}x${r.height.toFixed(1)}`
    : "";
  return `${label}: ${measure}, card [${card}]${region}, view ${view.current}, theme ${theme}, lang ${document.documentElement.lang}, exports [${exports}], locked ${String(project?.numbering.lock?.reason ?? "no")}, ${String(balloons.length)} balloons [${balloons.join("; ")}], selected ${String(selection.size)} [${chosen}], table rows [${rows}], rotation ${String(viewport.view.rotation)}, undo ${String(projectStore.canUndo)}, restored ${String(projectStore.notice?.restored_unsaved ?? false)}, focus ${focus}, editor ${String(editor)} ${balloonTools.editing ?? "-"}, ${document.visibilityState}, numbering ${projectStore.project?.settings.numbering.strategy ?? "-"} ghosts [${ghosts}] zones ${zones} views ${String(sheet?.views.length ?? 0)}, tool ${balloonTools.tool} on [${toolOn}], limits [${limits}]`;
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
  } else if ("keys" in step) {
    await typeKeys(step.keys);
    await sleep(STEP_PAUSE_MS);
  } else if ("button" in step) {
    // Icon only buttons of the title bar are found by their accessible name (T3.15), the cards
    // of the start screen by the title they are labelled by (T3.17).
    const buttons = [...document.querySelectorAll<HTMLElement>("button")];
    const labelledBy = (b: HTMLElement): string | undefined => {
      const id = b.getAttribute("aria-labelledby");
      return id === null ? undefined : document.getElementById(id)?.textContent.trim();
    };
    const button =
      buttons.find((b) => b.textContent.trim() === step.button) ??
      buttons.find((b) => b.getAttribute("aria-label") === step.button) ??
      buttons.find((b) => labelledBy(b) === step.button);
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
  } else if ("cell" in step) {
    const [row, col] = step.cell;
    const grid = [...document.querySelectorAll<HTMLElement>('[role="grid"]')].find(
      (g) => g.offsetParent !== null && g.querySelector("[data-verdict]") !== null,
    );
    const cell = grid?.querySelector<HTMLElement>(
      `[data-row="${String(row - 1)}"] [data-col="${String(col - 1)}"]`,
    );
    cell?.dispatchEvent(
      new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0, buttons: 1 }),
    );
    grid?.focus({ preventScroll: true });
    await sleep(STEP_PAUSE_MS);
  } else if ("paste" in step) {
    const target = document.activeElement ?? document.body;
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", {
      value: { getData: (type: string) => (type === "text/plain" ? step.paste : "") },
    });
    target.dispatchEvent(event);
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
    // A work area (`drawing`, `export`, `settings`, ...) or else a tab (`home`, `report`, ...).
    if (isView(step.show)) {
      view.set(step.show);
    } else {
      view.setTab(step.show);
    }
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
  } else if ("fill" in step) {
    const field = labelled(step.fill[0])?.querySelector<HTMLInputElement>(
      "input[type=text], input[type=number], input[type=date]",
    );
    if (field) {
      field.focus({ preventScroll: true });
      field.value = step.fill[1];
      field.dispatchEvent(new Event("input", { bubbles: true }));
      field.dispatchEvent(new Event("change", { bubbles: true }));
    }
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
  // Real mouse input while the script plays. The script's own pointer ID keeps it out of the
  // scripted gestures; the count is reported to explain a run that still looks odd.
  let real = 0;
  const countReal = (e: PointerEvent) => {
    if (e.isTrusted) {
      real += 1;
    }
  };
  window.addEventListener("pointermove", countReal, true);
  window.addEventListener("pointerdown", countReal, true);
  try {
    for (const [index, step] of steps.entries()) {
      try {
        await run(step);
      } catch (e) {
        await log(`step ${String(index + 1)} failed: ${String(e)}`);
      }
    }
  } finally {
    window.removeEventListener("pointermove", countReal, true);
    window.removeEventListener("pointerdown", countReal, true);
  }
  await log(`real pointer events during the script: ${String(real)}`);
  await log("script done");
}
