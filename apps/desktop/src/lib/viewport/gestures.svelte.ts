/**
 * Pointer gestures on the drawing (T1.6): select, multi select, box select, group move, leader
 * anchor drag, place by click or region, and pan. The viewport forwards its pointer events;
 * this class decides what they mean, keeps the optimistic preview of a drag in view state and
 * sends one command on release (frontend.md).
 *
 * Left button, in order of precedence:
 * - on the anchor handle of a selected balloon: drag the leader anchor;
 * - on a balloon: click selects (Shift or Cmd/Ctrl toggles), drag moves the selection,
 *   double click opens the value editor;
 * - Shift on empty drawing: box select, adds to the selection;
 * - place tool on empty drawing: click places a balloon at the point, drag places one for the
 *   dragged region;
 * - select tool on empty drawing: drag pans, click clears the selection.
 *
 * The middle button and the left button while Space is held always pan.
 */

import type { Balloon, BalloonId, CharId, CharacteristicStatus } from "$lib/ipc/bindings";
import {
  balloonTools,
  NO_OVERRIDE,
  type BalloonToolsStore,
} from "$lib/stores/balloon-tools.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import { projectStore, type ProjectStore } from "$lib/stores/project.svelte";
import { selection, type SelectionStore } from "$lib/stores/selection.svelte";
import { viewport } from "$lib/stores/viewport.svelte";
import {
  UNITS_PER_MM,
  anchorMove,
  balloonGeometry,
  centerInRect,
  groupMoves,
  hitTest,
  nearPoint,
  placeBalloon,
  rectFrom,
  resolveStyle,
  type BalloonGeometry,
  type Diagonal,
  type ResolvedStyle,
} from "./balloons";
import { frozenSet } from "$lib/sets";
import { afterBox, afterClick, forDrag } from "./pick";
import { screenToSheet, type Point, type Rect, type Size, type ViewTransform } from "./view-math";

/** Screen distance a pointer must move before a press becomes a drag, in CSS px. */
export const DRAG_THRESHOLD_PX = 4;
/** Extra screen distance around a balloon that still hits it, in CSS px. */
export const HIT_TOLERANCE_PX = 3;
/** Radius of the leader anchor handle of a selected balloon, in CSS px. */
export const ANCHOR_HANDLE_PX = 5;
/** Longest time between two clicks of a double click, in ms. */
export const DOUBLE_CLICK_MS = 450;

/** One balloon as the overlay draws it, in sheet space. */
export interface RenderedBalloon {
  id: BalloonId;
  characteristic: CharId;
  /** Displayed number, from Rust. */
  text: string;
  status: CharacteristicStatus;
  style: ResolvedStyle;
  /** Shape at the stored position. */
  geometry: BalloonGeometry;
  anchor: Point;
  position: Point;
}

/** The parts of a pointer event a gesture needs. */
export interface PointerInput {
  pointerId: number;
  button: number;
  shiftKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  /** Position relative to the viewport element, in CSS px. */
  x: number;
  y: number;
  /** Event time in ms. */
  timeStamp: number;
}

type Gesture =
  | {
      kind: "press-balloon";
      pointer: number;
      start: Point;
      hit: RenderedBalloon;
      additive: boolean;
    }
  | { kind: "move"; pointer: number; start: Point; ids: ReadonlySet<BalloonId>; delta: Point }
  | { kind: "anchor"; pointer: number; balloon: RenderedBalloon; anchor: Point }
  | { kind: "box"; pointer: number; start: Point; end: Point }
  | { kind: "press-empty"; pointer: number; start: Point; screen: Point; additive: boolean }
  | { kind: "region"; pointer: number; start: Point; end: Point }
  | { kind: "pan"; pointer: number; last: Point; moved: boolean; additive: boolean };

/** What the controller reads from and does to the rest of the app. Singletons in the app. */
export interface GestureDeps {
  project: ProjectStore;
  selection: SelectionStore;
  tools: BalloonToolsStore;
  /** The view transform the overlay is drawn with. */
  view: () => ViewTransform;
  /** Size of the shown sheet in sheet units. */
  sheet: () => Size | null;
  /** Pans the view by screen pixels. */
  panBy: (dx: number, dy: number) => void;
}

function additiveOf(input: PointerInput): boolean {
  return input.shiftKey || input.metaKey || input.ctrlKey;
}

export class BalloonGestures {
  /** The gesture in progress, `null` when idle. Its preview is drawn by the overlay. */
  gesture = $state.raw<Gesture | null>(null);
  /** True while Space is held: the left button pans. */
  spaceHeld = $state(false);
  /** What is under the idle pointer, for the cursor. */
  hover = $state<"balloon" | "anchor" | null>(null);

  readonly #d: GestureDeps;
  #lastClick: { id: CharId; time: number } | null = null;

  constructor(deps: GestureDeps) {
    this.#d = deps;
  }

  /** Balloons on the shown sheet with their style and shape, in drawing order. */
  readonly rendered = $derived.by((): RenderedBalloon[] => {
    const project = this.#d.project.project;
    if (project === null) {
      return [];
    }
    const base = project.settings.balloon_style;
    const byId = this.#d.project.characteristicById;
    return this.#d.tools.sheetBalloons.map((b: Balloon) => {
      const char = byId.get(b.characteristic);
      const style = resolveStyle(base, b.style);
      const text = char?.number ?? "";
      const position = { x: b.position.x ?? 0, y: b.position.y ?? 0 };
      return {
        id: b.id,
        characteristic: b.characteristic,
        text,
        status: char?.status ?? "accepted",
        style,
        geometry: balloonGeometry(style, text, position),
        anchor: { x: b.anchor.x ?? 0, y: b.anchor.y ?? 0 },
        position,
      };
    });
  });

  /** Balloons as drawn right now: moved or re-anchored by the gesture in progress. */
  readonly shown = $derived.by((): RenderedBalloon[] => {
    const g = this.gesture;
    if (g?.kind === "move") {
      const { x: dx, y: dy } = g.delta;
      return this.rendered.map((b) => {
        if (!g.ids.has(b.id)) {
          return b;
        }
        const position = { x: b.position.x + dx, y: b.position.y + dy };
        return { ...b, position, geometry: { ...b.geometry, center: position } };
      });
    }
    if (g?.kind === "anchor") {
      return this.rendered.map((b) => (b.id === g.balloon.id ? { ...b, anchor: g.anchor } : b));
    }
    return this.rendered;
  });

  /** The box select or place region being dragged, in sheet space. */
  get dragRect(): { kind: "box" | "region"; rect: Rect } | null {
    const g = this.gesture;
    if (g?.kind === "box" || g?.kind === "region") {
      return { kind: g.kind, rect: rectFrom(g.start, g.end) };
    }
    return null;
  }

  get active(): boolean {
    return this.gesture !== null;
  }

  get panning(): boolean {
    return this.gesture?.kind === "pan";
  }

  #toSheet(input: PointerInput): Point {
    return screenToSheet(this.#d.view(), { x: input.x, y: input.y });
  }

  /** Sheet units per CSS pixel at the current zoom. */
  #perPx(): number {
    return 1 / this.#d.view().scale;
  }

  /** The selected balloon whose anchor handle is at `p`, if its leader is drawn. */
  #anchorAt(p: Point): RenderedBalloon | null {
    const radius = (ANCHOR_HANDLE_PX + HIT_TOLERANCE_PX) * this.#perPx();
    const ids = this.#d.selection.ids;
    for (let i = this.rendered.length - 1; i >= 0; i--) {
      const b = this.rendered[i];
      if (b && ids.has(b.characteristic) && b.style.leader && nearPoint(p, b.anchor, radius)) {
        return b;
      }
    }
    return null;
  }

  #balloonAt(p: Point): RenderedBalloon | null {
    const index = hitTest(
      this.rendered.map((b) => b.geometry),
      p,
      HIT_TOLERANCE_PX * this.#perPx(),
    );
    return this.rendered[index] ?? null;
  }

  /** Pointer pressed. Returns whether the press starts a gesture (then capture the pointer). */
  down(input: PointerInput): boolean {
    if (this.gesture !== null || this.#d.sheet() === null) {
      return false;
    }
    const screen = { x: input.x, y: input.y };
    const additive = additiveOf(input);
    if (input.button === 1 || (input.button === 0 && this.spaceHeld)) {
      this.gesture = { kind: "pan", pointer: input.pointerId, last: screen, moved: true, additive };
      return true;
    }
    if (input.button !== 0) {
      return false;
    }
    const p = this.#toSheet(input);
    const pointer = input.pointerId;
    const anchored = this.#anchorAt(p);
    if (anchored) {
      this.gesture = { kind: "anchor", pointer, balloon: anchored, anchor: anchored.anchor };
      return true;
    }
    const hit = this.#balloonAt(p);
    if (hit) {
      this.gesture = { kind: "press-balloon", pointer, start: p, hit, additive };
      return true;
    }
    if (input.shiftKey) {
      this.gesture = { kind: "box", pointer, start: p, end: p };
      return true;
    }
    if (this.#d.tools.tool === "place") {
      this.gesture = { kind: "press-empty", pointer, start: p, screen, additive };
      return true;
    }
    this.gesture = { kind: "pan", pointer, last: screen, moved: false, additive };
    return true;
  }

  /** Pointer moved. Updates the preview or the hover state. */
  move(input: PointerInput): void {
    const g = this.gesture;
    if (g === null) {
      this.#updateHover(input);
      return;
    }
    if (g.pointer !== input.pointerId) {
      return;
    }
    const p = this.#toSheet(input);
    const threshold = DRAG_THRESHOLD_PX * this.#perPx();
    switch (g.kind) {
      case "press-balloon": {
        if (!nearPoint(p, g.start, threshold)) {
          const chars = forDrag(this.#d.selection.ids, g.hit.characteristic, g.additive);
          this.#d.selection.select(chars, g.hit.characteristic);
          const selected = this.#d.selection.ids;
          const ids = frozenSet(
            this.rendered.filter((b) => selected.has(b.characteristic)).map((b) => b.id),
          );
          this.gesture = {
            kind: "move",
            pointer: g.pointer,
            start: g.start,
            ids,
            delta: sub(p, g.start),
          };
        }
        break;
      }
      case "move":
        this.gesture = { ...g, delta: sub(p, g.start) };
        break;
      case "anchor":
        this.gesture = { ...g, anchor: p };
        break;
      case "box":
      case "region":
        this.gesture = { ...g, end: p };
        break;
      case "press-empty":
        if (!nearPoint(p, g.start, threshold)) {
          this.gesture = { kind: "region", pointer: g.pointer, start: g.start, end: p };
        }
        break;
      case "pan": {
        const dx = input.x - g.last.x;
        const dy = input.y - g.last.y;
        if (!g.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) {
          break;
        }
        this.#d.panBy(dx, dy);
        this.gesture = { ...g, last: { x: input.x, y: input.y }, moved: true };
        break;
      }
    }
  }

  /** Pointer released: finishes the gesture, sending at most one command. */
  async up(input: PointerInput): Promise<void> {
    const g = this.gesture;
    if (g === null || g.pointer !== input.pointerId) {
      return;
    }
    const d = this.#d;
    switch (g.kind) {
      case "press-balloon": {
        this.gesture = null;
        const id = g.hit.characteristic;
        const last = this.#lastClick;
        const double =
          !g.additive && last?.id === id && input.timeStamp - last.time < DOUBLE_CLICK_MS;
        this.#lastClick = double ? null : { id, time: input.timeStamp };
        d.selection.select(afterClick(d.selection.ids, id, g.additive));
        if (double) {
          d.tools.editPrimary();
        }
        return;
      }
      case "move":
        // The preview stays until the store shows the moved balloons, so nothing jumps back.
        await d.tools.move(groupMoves(this.rendered, g.ids, g.delta));
        break;
      case "anchor":
        await d.tools.move(anchorMove(g.balloon, g.anchor));
        break;
      case "box": {
        const rect = rectFrom(g.start, g.end);
        const inside = this.rendered
          .filter((b) => centerInRect(b.geometry, rect))
          .map((b) => b.characteristic);
        d.selection.select(afterBox(d.selection.ids, inside, true));
        break;
      }
      case "press-empty":
      case "region": {
        this.gesture = null;
        const sheet = d.sheet();
        const project = d.project.project;
        if (sheet === null || project === null) {
          return;
        }
        const size =
          resolveStyle(project.settings.balloon_style, NO_OVERRIDE).sizeMm * UNITS_PER_MM;
        // A drag along one axis has no area: Rust refuses an empty region, so it is a click.
        const minSide = DRAG_THRESHOLD_PX * this.#perPx();
        const dragged = g.kind === "region" ? rectFrom(g.start, g.end) : null;
        const region =
          dragged && dragged.width >= minSide && dragged.height >= minSide ? dragged : null;
        const placement = placeBalloon(region ?? g.start, sheet, size, upRight(d.view()));
        await d.tools.place(placement, region);
        return;
      }
      case "pan":
        if (!g.moved) {
          d.selection.select(afterClick(d.selection.ids, null, g.additive));
        }
        break;
    }
    if (this.gesture === g) {
      this.gesture = null;
    }
  }

  /** Escape or pointer cancel: drops the gesture and its preview without a command. */
  cancel(): boolean {
    if (this.gesture === null) {
      return false;
    }
    this.gesture = null;
    return true;
  }

  #updateHover(input: PointerInput): void {
    if (this.#d.sheet() === null) {
      this.hover = null;
      return;
    }
    const p = this.#toSheet(input);
    this.hover = this.#anchorAt(p) ? "anchor" : this.#balloonAt(p) ? "balloon" : null;
  }
}

/** The sheet direction that is up and to the right on screen, for any view rotation. */
export function upRight(view: ViewTransform): Diagonal {
  const origin = screenToSheet(view, { x: 0, y: 0 });
  const corner = screenToSheet(view, { x: 1, y: -1 });
  return { x: corner.x >= origin.x ? 1 : -1, y: corner.y >= origin.y ? 1 : -1 };
}

function sub(a: Point, b: Point): Point {
  return { x: a.x - b.x, y: a.y - b.y };
}

/** The gestures of the app window's viewport. */
export const balloonGestures = new BalloonGestures({
  project: projectStore,
  selection,
  tools: balloonTools,
  view: () => viewport.view,
  sheet: () => documentStore.sheetSize,
  panBy: (dx, dy) => {
    viewport.panBy(dx, dy);
  },
});
