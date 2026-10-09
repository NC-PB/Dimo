/**
 * Developer tools of `tauri dev` (T0.8): dummy balloons and the pan frame time check.
 * Only active when Vite runs in development mode; production builds never show them.
 */

import { commands, type DevStartup } from "$lib/ipc/bindings";
import type { DocumentStore } from "$lib/stores/document.svelte";
import type { ProjectStore } from "$lib/stores/project.svelte";
import type { ViewportStore } from "$lib/stores/viewport.svelte";
import { ACTUAL_SIZE_SCALE, centerOn, type Point } from "$lib/viewport/view-math";
import { runPanCheck, type FrameStats } from "./pan-check";

/** Resolves after two animation frames, when effects and layout of a change have run. */
function afterLayout(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        resolve();
      });
    });
  });
}

export const DEV_TOOLS_ENABLED = import.meta.env.DEV;

/** Balloon count of NFR-PERF-02. */
export const DEFAULT_BALLOON_COUNT = 500;

export class DevToolsStore {
  balloons = $state(false);
  count = $state(DEFAULT_BALLOON_COUNT);
  /** Fixed balloon anchors from `DIMO_DEV_ANCHORS`; empty for random positions. */
  anchors = $state<Point[]>([]);
  running = $state(false);
  result = $state<FrameStats | null>(null);

  /** Number of dummy balloons on each sheet: one per anchor when anchors are set. */
  get shownCount(): number {
    if (!this.balloons) {
      return 0;
    }
    return this.anchors.length > 0 ? Math.min(this.count, this.anchors.length) : this.count;
  }

  /**
   * Applies the startup settings from the `DIMO_DEV_*` environment variables of the Rust
   * process: open a drawing or project (and run the dev script), show balloons, run the pan
   * check.
   */
  async startup(
    projects: ProjectStore,
    documents: DocumentStore,
    viewport: ViewportStore,
  ): Promise<void> {
    let startup: DevStartup;
    try {
      const result = await commands.devStartup();
      if (result.status === "error") {
        projects.error = result.error;
        return;
      }
      startup = result.data;
    } catch {
      return; // Outside Tauri, for example in a plain browser.
    }
    if (startup.balloons > 0) {
      this.count = startup.balloons;
      this.balloons = true;
    }
    this.anchors = startup.anchors.map((p) => ({ x: p.x ?? 0, y: p.y ?? 0 }));
    const opened = startup.project?.snapshot ?? null;
    if (startup.project) {
      // The events of the open and the script may still be on their way; this is the result.
      projects.load(startup.project);
      documents.setSheet(startup.sheet);
    }
    const view = startup.view;
    if (view && opened) {
      // Wait until the viewport has fitted the new sheet, then replace the fit.
      await afterLayout();
      viewport.set(
        centerOn(
          { x: view.center.x ?? 0, y: view.center.y ?? 0 },
          ((view.percent ?? 100) / 100) * ACTUAL_SIZE_SCALE,
          viewport.size,
          viewport.rotation,
        ),
      );
    }
    if (startup.pan_check && opened) {
      // Let the first tiles arrive before measuring.
      await new Promise((resolve) => setTimeout(resolve, 2000));
      await this.measurePan(viewport);
    }
  }

  /** Runs the scripted pan and reports the frame times to the `tauri dev` terminal. */
  async measurePan(viewport: ViewportStore): Promise<void> {
    if (this.running) {
      return;
    }
    this.running = true;
    this.result = null;
    const start = viewport.view;
    try {
      const stats = await runPanCheck((dx, dy) => {
        viewport.view = { ...start, tx: start.tx + dx, ty: start.ty + dy };
      });
      this.result = stats;
      await commands
        .devReportFrameTimes({
          balloons: this.shownCount,
          frames: stats.frames,
          mean_ms: stats.meanMs,
          p95_ms: stats.p95Ms,
          max_ms: stats.maxMs,
          device_pixel_ratio: window.devicePixelRatio,
          viewport: [viewport.size.width, viewport.size.height],
        })
        .catch(() => undefined);
    } finally {
      viewport.view = start;
      this.running = false;
    }
  }
}

export const devTools = new DevToolsStore();
