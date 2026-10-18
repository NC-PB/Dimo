/**
 * The "Adjust image" editor of photo and scan drawings (R.20, R01 decisions 8 to 11). View
 * state only: the settings being edited. Rust draws every preview (`preview_image_edit`),
 * proposes the corners (`image_corner_start`) and applies the settings as one undo step with
 * the sheet size and balloon positions it derives itself (`apply_image_edit`). Before an apply
 * that would move balloons off the sheet, Rust answers with their number and the editor asks.
 */

import {
  commands,
  type CornerStart,
  type ImageEdit,
  type ImageEditOutcome,
  type ImagePoint,
  type ImagePreview,
  type Paper,
  type PreviewStage,
  type ProjectPatched,
  type Rotation,
  type SheetCorners,
  type Trim,
} from "$lib/ipc/bindings";
import type { Result } from "$lib/ipc/result";
import { asCommandError, commandErrorMessage } from "$lib/errors";
import {
  DEFAULT_EDIT,
  MAX_ANGLE,
  decimalText,
  decimalValue,
  renameCorners,
  sameEdit,
  turnedBy,
} from "$lib/image-edit";
import { documentStore } from "./document.svelte";
import { projectStore } from "./project.svelte";

/** The tools of the editor; each shows its own stage of the edit. */
export type ImageTool = "rotate" | "corners" | "trim" | "paper";

/** Wait after the last change before a new preview is asked for, in ms. */
const PREVIEW_DELAY = 80;

/** The Rust calls of the editor. */
export interface ImageEditorApi {
  previewImageEdit(
    page: number,
    edit: ImageEdit,
    stage: PreviewStage,
  ): Promise<Result<ImagePreview>>;
  closeImagePreview(): Promise<Result<null>>;
  imageCornerStart(page: number, rotation: Rotation): Promise<Result<CornerStart>>;
  applyImageEdit(
    page: number,
    edit: ImageEdit,
    moveOffSheet: boolean,
  ): Promise<Result<ImageEditOutcome>>;
}

/** What the editor needs of the project. */
export interface ImageEditorProject {
  readonly imagePages: readonly ImageEdit[] | null;
  editWith<T>(
    send: () => Promise<Result<T>>,
    patchedOf: (answer: T) => ProjectPatched | null,
  ): Promise<T | undefined>;
}

export interface ImageEditorDeps {
  api?: ImageEditorApi;
  project?: ImageEditorProject;
  /** The sheet on screen, the page the editor opens with. */
  shownSheet?: () => number;
  /** Runs `work` after `ms`; tests run it at once. */
  later?: (work: () => void, ms: number) => void;
}

export class ImageEditorStore {
  open = $state(false);
  page = $state(0);
  edit = $state.raw<ImageEdit>(DEFAULT_EDIT);
  tool = $state<ImageTool>("rotate");
  preview = $state.raw<ImagePreview | null>(null);
  /** Corners to start from for the current quarter turns, once asked for. */
  cornerStart = $state.raw<CornerStart | null>(null);
  /** Balloons an apply would move off the sheet; the editor asks while it is set. */
  offSheet = $state<number | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);

  readonly #api: ImageEditorApi;
  readonly #project: ImageEditorProject;
  readonly #shownSheet: () => number;
  readonly #later: (work: () => void, ms: number) => void;
  #saved: ImageEdit = DEFAULT_EDIT;
  #request = 0;
  #pending = false;
  #loading = false;
  #stale = false;

  constructor(deps: ImageEditorDeps = {}) {
    this.#api = deps.api ?? commands;
    this.#project = deps.project ?? projectStore;
    this.#shownSheet = deps.shownSheet ?? (() => documentStore.sheet);
    this.#later = deps.later ?? ((work, ms) => setTimeout(work, ms));
  }

  /** True if the current drawing is a photo or scan, so the editor can open. */
  get available(): boolean {
    return this.#project.imagePages !== null;
  }

  /** The stage the preview shows for the active tool. */
  get stage(): PreviewStage {
    switch (this.tool) {
      case "corners":
        return "corners";
      case "trim":
        return "trim";
      default:
        return "result";
    }
  }

  /** True if the settings differ from what the project holds. */
  get changed(): boolean {
    return !sameEdit(this.edit, this.#saved);
  }

  /** The fine angle in degrees, as a number. */
  get angle(): number {
    return decimalValue(this.edit.fine_angle);
  }

  /** Opens the editor on the sheet on screen with the settings the project holds. */
  show(page = this.#shownSheet()): void {
    const pages = this.#project.imagePages;
    const saved = pages?.[page];
    if (saved === undefined) {
      return;
    }
    this.page = page;
    this.#saved = saved;
    this.edit = saved;
    this.tool = "rotate";
    this.cornerStart = null;
    this.offSheet = null;
    this.error = null;
    this.preview = null;
    this.open = true;
    this.#refresh();
  }

  /** Closes without applying. */
  close(): void {
    if (!this.open) {
      return;
    }
    this.open = false;
    this.preview = null;
    this.offSheet = null;
    this.#request += 1;
    void this.#api.closeImagePreview();
  }

  setTool(tool: ImageTool): void {
    this.tool = tool;
    if (tool === "corners") {
      void this.#startCorners(this.edit.corners === null);
    }
    this.#refresh();
  }

  /** Turns by quarter turns, clockwise for a positive `delta`; corners keep their points. */
  turn(delta: number): void {
    const rotation = turnedBy(this.edit.rotation, delta);
    const corners =
      this.edit.corners === null
        ? null
        : renameCorners(this.edit.corners, this.edit.rotation, rotation);
    this.cornerStart = null;
    this.#set({ rotation, corners });
  }

  /** Sets the fine angle in degrees (typed or from the slider); out of range is clamped. */
  setAngle(degrees: number): void {
    if (!Number.isFinite(degrees)) {
      return;
    }
    const clamped = Math.min(MAX_ANGLE, Math.max(-MAX_ANGLE, degrees));
    this.#set({ fine_angle: decimalText(clamped, 2) });
  }

  /** Switches the corner correction on (from the proposal or the page corners) or off. */
  async setCornersOn(on: boolean): Promise<void> {
    if (!on) {
      this.#set({ corners: null });
      return;
    }
    await this.#startCorners(true);
  }

  /** Moves one corner to a pixel of the stored page. */
  moveCorner(name: keyof SheetCorners, point: ImagePoint): void {
    const corners = this.edit.corners;
    if (corners !== null) {
      this.#set({ corners: { ...corners, [name]: point } });
    }
  }

  setTrim(trim: Trim | null): void {
    this.#set({ trim });
  }

  setPaper(paper: Paper | null): void {
    this.#set({ paper });
  }

  setCleanUp(on: boolean): void {
    this.#set({ clean_up: on });
  }

  /** Back to the image as it is. */
  reset(): void {
    this.cornerStart = null;
    this.#set(DEFAULT_EDIT);
  }

  /**
   * Applies the settings as one undo step. If balloons would leave the sheet, nothing changes
   * and {@link offSheet} asks; `moveOffSheet` applies anyway, putting them on the sheet edge.
   */
  async apply(moveOffSheet = false): Promise<boolean> {
    if (this.busy) {
      return false;
    }
    if (!this.changed) {
      this.close();
      return true;
    }
    this.busy = true;
    this.offSheet = null;
    try {
      const page = this.page;
      const edit = this.edit;
      const outcome = await this.#project.editWith(
        () => this.#api.applyImageEdit(page, edit, moveOffSheet),
        (answer) => (answer.type === "applied" ? answer.patched : null),
      );
      if (outcome === undefined) {
        return false;
      }
      if (outcome.type === "off_sheet") {
        this.offSheet = outcome.balloons;
        return false;
      }
      this.#saved = edit;
      this.close();
      return true;
    } finally {
      this.busy = false;
    }
  }

  /** Answer to the off sheet question: `true` applies anyway, `false` keeps editing. */
  async answerOffSheet(apply: boolean): Promise<void> {
    if (apply) {
      await this.apply(true);
    } else {
      this.offSheet = null;
    }
  }

  /** Asks Rust where corners start; `place` also puts the corners there. */
  async #startCorners(place: boolean): Promise<void> {
    const rotation = this.edit.rotation;
    let start = this.cornerStart;
    if (start === null) {
      const result = await this.#api.imageCornerStart(this.page, rotation);
      if (result.status === "error") {
        this.error = commandErrorMessage(asCommandError(result.error));
        return;
      }
      start = result.data;
      this.cornerStart = start;
    }
    if (place && this.edit.rotation === rotation) {
      // Corners straighten the sheet themselves (R.20), so the fine angle goes back to 0.
      this.#set({ corners: start.proposed ?? start.page, fine_angle: "0" });
    }
  }

  #set(change: Partial<ImageEdit>): void {
    this.edit = { ...this.edit, ...change };
    this.#refresh();
  }

  /**
   * Asks for a new preview after a short pause. At most one request is in flight, so Rust
   * opens previews in the order they were asked for and never closes the one on screen; a
   * change made meanwhile asks again when the answer is in.
   */
  #refresh(): void {
    if (!this.open || this.#pending) {
      return;
    }
    if (this.#loading) {
      this.#stale = true;
      return;
    }
    this.#pending = true;
    this.#later(() => {
      this.#pending = false;
      void this.#load();
    }, PREVIEW_DELAY);
  }

  async #load(): Promise<void> {
    if (!this.open) {
      return;
    }
    this.#request += 1;
    const request = this.#request;
    this.#loading = true;
    this.#stale = false;
    let result: Result<ImagePreview>;
    try {
      result = await this.#api.previewImageEdit(this.page, this.edit, this.stage);
    } finally {
      this.#loading = false;
    }
    if (this.#stale) {
      this.#refresh();
    }
    if (request !== this.#request || !this.open) {
      return;
    }
    if (result.status === "error") {
      this.error = commandErrorMessage(asCommandError(result.error));
      return;
    }
    this.error = null;
    this.preview = result.data;
  }
}

/** The image editor of the app window. */
export const imageEditor = new ImageEditorStore();
