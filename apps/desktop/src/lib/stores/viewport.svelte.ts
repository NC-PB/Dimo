import {
  IDENTITY,
  ZOOM_STEP,
  constrainPan,
  fitSheet,
  panBy,
  zoomAt,
  type Point,
  type Size,
  type ViewTransform,
} from "$lib/viewport/view-math";

/** Distance of one keyboard pan step, as a share of the viewport size. */
const KEY_PAN_SHARE = 0.1;

/**
 * Pan and zoom of the drawing viewport (FR-DOC-04). Pure view state, never sent to Rust.
 * Toolbar buttons, shortcuts and pointer gestures all change the view through this store.
 */
export class ViewportStore {
  view = $state<ViewTransform>(IDENTITY);
  /** Size of the viewport element in CSS px. */
  size = $state<Size>({ width: 0, height: 0 });
  /** Size of the sheet on screen in sheet units, `null` without a document. */
  sheet = $state<Size | null>(null);

  /** Sets a new view, keeping part of the sheet visible. */
  set(view: ViewTransform): void {
    this.view = this.sheet ? constrainPan(view, this.sheet, this.size) : view;
  }

  fit(): void {
    if (this.sheet) {
      this.view = fitSheet(this.sheet, this.size);
    }
  }

  /** Zooms by `factor` around `at` (screen point), by default the viewport center. */
  zoomBy(factor: number, at?: Point): void {
    const center = at ?? { x: this.size.width / 2, y: this.size.height / 2 };
    this.set(zoomAt(this.view, factor, center));
  }

  zoomIn(): void {
    this.zoomBy(ZOOM_STEP);
  }

  zoomOut(): void {
    this.zoomBy(1 / ZOOM_STEP);
  }

  panBy(dx: number, dy: number): void {
    this.set(panBy(this.view, dx, dy));
  }

  /** One keyboard pan step. `dx`, `dy` are -1, 0 or 1: the direction the drawing moves. */
  panStep(dx: number, dy: number): void {
    this.panBy(dx * this.size.width * KEY_PAN_SHARE, dy * this.size.height * KEY_PAN_SHARE);
  }
}

export const viewport = new ViewportStore();
