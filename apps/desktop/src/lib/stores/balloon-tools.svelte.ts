/**
 * Balloon tools of the drawing viewport (T1.6, FR-BAL-01, FR-BAL-02, FR-BAL-12): the active
 * tool, the inline value editor and the actions on selected balloons. View state plus thin
 * wrappers that turn user actions into document commands; Rust does the work (ADR 0001).
 */

import type {
  Balloon,
  BalloonId,
  BalloonMove,
  BalloonStyleOverride,
  CharId,
  Command,
  OrientedBox,
  Patch,
  SheetId,
} from "$lib/ipc/bindings";
import { frozenSet } from "$lib/sets";
import { UNITS_PER_MM, groupMoves, type Placement } from "$lib/viewport/balloons";
import { screenToSheet, type Rect, type ViewTransform } from "$lib/viewport/view-math";
import { documentStore } from "./document.svelte";
import { projectStore, type ProjectStore } from "./project.svelte";
import { selection, type SelectionStore } from "./selection.svelte";

/** Distance of one keyboard move of balloons, in mm on the printed sheet. */
export const NUDGE_MM = 1;

/** Select: click, drag and box select balloons. Place: click or drag on the drawing to add one. */
export type Tool = "select" | "place";

/** A style override that changes nothing; restyles set single fields on top of it. */
export const NO_OVERRIDE: BalloonStyleOverride = {
  shape: null,
  size_mm: null,
  outline_mm: null,
  outline_color: null,
  fill_color: null,
  text_color: null,
  leader: null,
};

/** The ID of the characteristic a patch inserted, if any. */
export function insertedCharacteristic(patch: Patch | undefined): CharId | null {
  for (const change of patch?.changes ?? []) {
    if (change.type === "characteristic_inserted") {
      return change.characteristic.id;
    }
  }
  return null;
}

/** The source region of a characteristic for a rectangle drawn on the sheet (FR-CHR-09). */
export function regionBox(rect: Rect): OrientedBox {
  return {
    center: { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 },
    size: { width: rect.width, height: rect.height },
    angle: 0,
  };
}

export class BalloonToolsStore {
  tool = $state<Tool>("select");
  /** Characteristic whose inline value editor is open in the viewport. */
  editing = $state<CharId | null>(null);
  /** Whether the style picker of the selection is open. */
  styleOpen = $state(false);

  readonly #project: ProjectStore;
  readonly #selection: SelectionStore;
  readonly #sheet: () => SheetId | undefined;

  /**
   * @param project the open project
   * @param chosen the shared selection
   * @param sheet ID of the sheet shown in the viewport
   */
  constructor(project: ProjectStore, chosen: SelectionStore, sheet: () => SheetId | undefined) {
    this.#project = project;
    this.#selection = chosen;
    this.#sheet = sheet;
  }

  /** Balloons on the sheet in the viewport, in drawing order. */
  get sheetBalloons(): readonly Balloon[] {
    const sheet = this.#sheet();
    return sheet === undefined ? [] : this.#project.balloonsOnSheet(sheet);
  }

  /** Balloons of the selected characteristics, on all sheets. */
  get selectedBalloons(): BalloonId[] {
    const ids = this.#selection.ids;
    return (this.#project.project?.balloons ?? [])
      .filter((b) => ids.has(b.characteristic))
      .map((b) => b.id);
  }

  setTool(tool: Tool): void {
    this.tool = tool;
  }

  /**
   * Adds a characteristic with its balloon (FR-BAL-01), selects it and opens the value editor
   * (FR-BAL-02). `region` is the rectangle the user dragged, `null` for a click.
   */
  async place(placement: Placement, region: Rect | null): Promise<CharId | null> {
    const sheet = this.#sheet();
    if (sheet === undefined) {
      return null;
    }
    const patch = await this.#project.execute({
      type: "add_characteristic",
      sheet,
      position: placement.position,
      anchor: placement.anchor,
      region: region ? regionBox(region) : null,
      values: [],
      insert_after: null,
    });
    const id = insertedCharacteristic(patch);
    if (id !== null) {
      this.#selection.focus(id, "viewport");
      this.editing = id;
    }
    return id;
  }

  /** Opens the value editor of the primary selected characteristic. */
  editPrimary(): boolean {
    const id = this.#selection.primary;
    if (id === null || !this.#project.characteristicById.has(id)) {
      return false;
    }
    this.#selection.focus(id, "viewport");
    this.editing = id;
    return true;
  }

  /** Closes the value editor. */
  stopEditing(): void {
    this.editing = null;
  }

  /** Stores the text typed in the value editor as requirement text, if it changed. */
  async commitText(id: CharId, text: string): Promise<void> {
    const current = this.#project.characteristicById.get(id);
    if (current === undefined || current.requirement_text === text.trim()) {
      return;
    }
    await this.#project.execute({
      type: "update_fields",
      ids: [id],
      values: [{ field: "requirement_text", value: text }],
    });
  }

  /** Moves balloons or leader anchors as one undoable command (FR-BAL-12). */
  async move(moves: BalloonMove[]): Promise<void> {
    if (moves.length > 0) {
      await this.#project.execute({ type: "move_balloons", moves });
    }
  }

  /**
   * Moves the selected balloons on the shown sheet one step in a screen direction (`dx`, `dy`
   * are -1, 0 or 1), the keyboard way to move balloons (NFR-UX-01). One undo step per press.
   * Returns false if nothing is selected on this sheet.
   */
  nudge(dx: number, dy: number, view: ViewTransform): boolean {
    const chars = this.#selection.ids;
    const placed = this.sheetBalloons
      .filter((b) => chars.has(b.characteristic))
      .map((b) => ({
        id: b.id,
        position: { x: b.position.x ?? 0, y: b.position.y ?? 0 },
        anchor: { x: b.anchor.x ?? 0, y: b.anchor.y ?? 0 },
      }));
    if (placed.length === 0) {
      return false;
    }
    // The screen direction in sheet space, also on a rotated sheet.
    const origin = screenToSheet(view, { x: 0, y: 0 });
    const to = screenToSheet(view, { x: dx, y: dy });
    const step = NUDGE_MM * UNITS_PER_MM * view.scale;
    const delta = { x: (to.x - origin.x) * step, y: (to.y - origin.y) * step };
    void this.move(groupMoves(placed, frozenSet(placed.map((b) => b.id)), delta));
    return true;
  }

  /** Deletes the selected characteristics with their balloons (FR-BAL-12). */
  async deleteSelection(): Promise<void> {
    const ids = [...this.#selection.ids];
    if (ids.length === 0) {
      return;
    }
    this.editing = null;
    await this.#project.execute({ type: "delete_characteristics", ids });
    this.#selection.retain((id) => this.#project.characteristicById.has(id));
  }

  /** Selects every balloon on the sheet in the viewport. */
  selectAll(): void {
    this.#selection.select(this.sheetBalloons.map((b) => b.characteristic));
  }

  /** Sets style fields on the selected balloons (FR-BAL-03, FR-BAL-12). */
  async restyle(style: Partial<BalloonStyleOverride>): Promise<void> {
    await this.#onSelected((ids) => ({
      type: "restyle_balloons",
      ids,
      style: { ...NO_OVERRIDE, ...style },
    }));
  }

  /** The selected balloons use the project default style again. */
  async resetStyle(): Promise<void> {
    await this.#onSelected((ids) => ({ type: "reset_balloon_style", ids }));
  }

  async #onSelected(command: (ids: BalloonId[]) => Command): Promise<void> {
    const ids = this.selectedBalloons;
    if (ids.length > 0) {
      await this.#project.execute(command(ids));
    }
  }

  /**
   * Escape: closes the style picker, else clears the selection, else returns to the select
   * tool. Returns whether it did something. Gestures and the editor handle Escape themselves.
   */
  escape(): boolean {
    if (this.styleOpen) {
      this.styleOpen = false;
      return true;
    }
    if (!this.#selection.isEmpty) {
      this.#selection.clear();
      return true;
    }
    if (this.tool !== "select") {
      this.tool = "select";
      return true;
    }
    return false;
  }
}

/** The balloon tools of the app window. */
export const balloonTools = new BalloonToolsStore(
  projectStore,
  selection,
  () => projectStore.sheets[documentStore.sheet]?.id,
);
