/**
 * Balloon style choices offered by the style popover and the project defaults in the settings
 * view (FR-BAL-03, D-24). Rust validates every value; these are only the offered options.
 */

import { m } from "$lib/i18n";
import type { BalloonShape, Color } from "$lib/ipc/bindings";

export const SHAPES: readonly BalloonShape[] = ["circle", "flag", "rectangle"];

/** Balloon sizes offered, in mm on the printed sheet (D-24 default 7). */
export const SIZES_MM: readonly number[] = [5, 6, 7, 8, 10, 12];

/** Outline widths offered, in mm (D-24 default 0.35). */
export const OUTLINES_MM: readonly number[] = [0.25, 0.35, 0.5, 0.7];

/**
 * Outline colors offered: the D-24 blue, black and three colors of a color blind safe palette.
 * Each one is named, so the choice never depends on seeing the color.
 */
export const COLORS: readonly { color: Color; label: () => string }[] = [
  { color: "#0057B8", label: () => m.color_blue() },
  { color: "#000000", label: () => m.color_black() },
  { color: "#009988", label: () => m.color_teal() },
  { color: "#CC3311", label: () => m.color_red() },
  { color: "#EE3377", label: () => m.color_magenta() },
];

/** Translated name of a balloon shape. */
export function shapeLabel(shape: BalloonShape): string {
  switch (shape) {
    case "circle":
      return m.shape_circle();
    case "flag":
      return m.shape_flag();
    case "rectangle":
      return m.shape_rectangle();
  }
}
