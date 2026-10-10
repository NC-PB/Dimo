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
  FieldValue,
  OrientedBox,
  Patch,
  SheetId,
} from "$lib/ipc/bindings";
import { commands } from "$lib/ipc/bindings";
import { frozenSet } from "$lib/sets";
import { UNITS_PER_MM, groupMoves, type Placement } from "$lib/viewport/balloons";
import { screenToSheet, type Rect, type ViewTransform } from "$lib/viewport/view-math";
import {
  boxSelect,
  explainLanguage,
  type BoxSelectStore,
  type RecognitionCommands,
} from "./box-select.svelte";
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

/**
 * Keys typed on the drawing between a placing click and the moment the value editor or the
 * proposal card has the keyboard focus (FR-BAL-02, NFR-UX-01). The editor or card that opens
 * takes them over, as if they had been typed into it.
 */
export interface TypedAhead {
  /** The characters typed, Backspace applied. */
  text: string;
  /** Enter was pressed: store the value (editor) or accept the proposals (card). */
  enter: boolean;
  /** Escape was pressed: close without storing (editor) or discard the proposals (card). */
  cancel: boolean;
}

/** What `typeAhead` reads of a key event. */
export type TypedKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "isComposing">;

/**
 * Whether a key types one character. Cmd and Ctrl combinations are shortcuts; Alt is not
 * excluded because it types characters such as `Ø` on macOS.
 */
export function typesCharacter(event: TypedKey): boolean {
  return [...event.key].length === 1 && !event.ctrlKey && !event.metaKey;
}

/** Who takes over keys typed ahead: the pending placement, the editor of a characteristic, the card. */
type AheadTarget = { kind: "pending" } | { kind: "editor"; id: CharId } | { kind: "card" };

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
  readonly #boxSelect: BoxSelectStore | null;
  readonly #recognition: Pick<RecognitionCommands, "readCalloutText"> | null;
  /** Keys typed while a placement is pending, until the editor or card takes them. */
  #ahead: { typed: TypedAhead; target: AheadTarget } | null = null;

  /**
   * @param project the open project
   * @param chosen the shared selection
   * @param sheet ID of the sheet shown in the viewport
   * @param box box select of the place tool (T2.6); without it a dragged box places a balloon
   * @param recognition reads typed values (M2 decision 5); without it they are text only
   */
  constructor(
    project: ProjectStore,
    chosen: SelectionStore,
    sheet: () => SheetId | undefined,
    box: BoxSelectStore | null = null,
    recognition: Pick<RecognitionCommands, "readCalloutText"> | null = null,
  ) {
    this.#project = project;
    this.#selection = chosen;
    this.#sheet = sheet;
    this.#boxSelect = box;
    this.#recognition = recognition;
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
   *
   * A dragged region is first read as box select (T2.6, FR-REC-01): if it holds PDF text, the
   * proposal card opens instead and nothing is added until the user accepts. Returns `null`
   * then.
   *
   * With numbering locked the new number follows the primary selected characteristic
   * (FR-BAL-11, `insert_after`). Keys typed until the editor or card opens are kept, see
   * `typeAhead`.
   */
  async place(placement: Placement, region: Rect | null): Promise<CharId | null> {
    const sheet = this.#sheet();
    if (sheet === undefined) {
      return null;
    }
    // A new click or box replaces an open proposal card.
    this.#boxSelect?.discard();
    const ahead = {
      typed: { text: "", enter: false, cancel: false },
      target: { kind: "pending" } as AheadTarget,
    };
    this.#ahead = ahead;
    try {
      if (region && this.#boxSelect) {
        const box = regionBox(region);
        if (await this.#boxSelect.select(sheet, region, box, placement)) {
          this.editing = null;
          ahead.target = { kind: "card" };
          return null;
        }
      }
      const patch = await this.#project.execute({
        type: "add_characteristic",
        sheet,
        position: placement.position,
        anchor: placement.anchor,
        region: region ? regionBox(region) : null,
        values: [],
        insert_after: this.insertAfter(),
      });
      const id = insertedCharacteristic(patch);
      if (id !== null) {
        ahead.target = { kind: "editor", id };
        this.#selection.focus(id, "viewport");
        this.editing = id;
      }
      return id;
    } finally {
      // Nothing opened (no text, refused, failed): the keys typed meanwhile have no target.
      if (this.#ahead === ahead && ahead.target.kind === "pending") {
        this.#ahead = null;
      }
    }
  }

  /**
   * The characteristic a new one follows when numbering is locked: the primary selection, if it
   * still exists (FR-BAL-11). Rust ignores it while numbering is unlocked.
   */
  insertAfter(): CharId | null {
    const id = this.#selection.primary;
    return id !== null && this.#project.characteristicById.has(id) ? id : null;
  }

  /**
   * A key pressed on the drawing. While a placement is pending (from the placing click until
   * the value editor or proposal card took over) characters, Backspace, Enter and Escape are
   * kept for the editor or card instead of acting as shortcuts, so nothing typed right after a
   * click is lost (FR-BAL-02, NFR-UX-01). Returns whether the key was kept; the caller then
   * prevents its default action and the shortcuts.
   */
  typeAhead(event: TypedKey): boolean {
    const ahead = this.#ahead;
    if (ahead === null || event.isComposing) {
      return false;
    }
    if (!this.#aheadAlive(ahead.target)) {
      this.#ahead = null;
      return false;
    }
    const typed = ahead.typed;
    if (typed.enter || typed.cancel) {
      return false; // Done typing: further keys act on the drawing again.
    }
    if (event.key === "Enter") {
      typed.enter = true;
    } else if (event.key === "Escape") {
      typed.cancel = true;
    } else if (event.key === "Backspace") {
      typed.text = [...typed.text].slice(0, -1).join("");
    } else if (typesCharacter(event)) {
      typed.text += event.key;
    } else {
      return false;
    }
    return true;
  }

  /**
   * Hands the keys typed ahead to the editor of `id` or to the proposal card (`"card"`), once.
   * Returns `null` if nothing was typed for it.
   */
  takeTyped(target: CharId | "card"): TypedAhead | null {
    const ahead = this.#ahead;
    if (ahead === null) {
      return null;
    }
    const t = ahead.target;
    const matches = target === "card" ? t.kind === "card" : t.kind === "editor" && t.id === target;
    if (!matches) {
      return null;
    }
    this.#ahead = null;
    return ahead.typed;
  }

  /** Whether the editor or card the keys are kept for is still coming. */
  #aheadAlive(target: AheadTarget): boolean {
    switch (target.kind) {
      case "pending":
        return true;
      case "editor":
        return this.editing === target.id;
      case "card":
        return this.#boxSelect?.open ?? false;
    }
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
    if (this.#ahead?.target.kind === "editor") {
      this.#ahead = null;
    }
  }

  /**
   * Stores the text typed in the value editor, if it changed. Rust parses and interprets it like
   * a box selection (M2 decision 5): kind, nominal, limits and rule are set in the same command.
   * Text that does not parse is stored as requirement text only.
   */
  async commitText(id: CharId, text: string): Promise<void> {
    const current = this.#project.characteristicById.get(id);
    if (current === undefined || current.requirement_text === text.trim()) {
      return;
    }
    let values: FieldValue[] = [{ field: "requirement_text", value: text }];
    const sheet = this.#sheet();
    if (this.#recognition && sheet !== undefined) {
      const read = await this.#recognition
        .readCalloutText(sheet, text, explainLanguage())
        .catch(() => null);
      if (read?.status === "ok") {
        values = read.data.values;
      }
    }
    await this.#project.execute({ type: "update_fields", ids: [id], values });
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
   * Escape: discards the open proposal card, else closes the style picker, else clears the
   * selection, else returns to the select tool. Returns whether it did something. Gestures and
   * the editor handle Escape themselves.
   */
  escape(): boolean {
    if (this.#boxSelect?.open) {
      this.#boxSelect.discard();
      return true;
    }
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
  boxSelect,
  commands,
);
