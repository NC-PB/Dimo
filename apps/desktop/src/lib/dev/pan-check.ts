/**
 * Frame time measurement during a scripted pan (T0.8, NFR-PERF-02). Development only.
 *
 * The check moves the view along a circle for a fixed time, one step per animation frame, and
 * records the time between frames. At 60 fps a frame takes 16.7 ms.
 */

export interface FrameStats {
  frames: number;
  meanMs: number;
  p95Ms: number;
  maxMs: number;
}

/** Mean, 95th percentile (nearest rank) and maximum of frame times in milliseconds. */
export function frameStats(frameTimes: readonly number[]): FrameStats {
  if (frameTimes.length === 0) {
    return { frames: 0, meanMs: 0, p95Ms: 0, maxMs: 0 };
  }
  const sorted = [...frameTimes].sort((a, b) => a - b);
  const sum = sorted.reduce((a, b) => a + b, 0);
  const rank = Math.max(0, Math.ceil(0.95 * sorted.length) - 1);
  return {
    frames: sorted.length,
    meanMs: sum / sorted.length,
    p95Ms: sorted[rank] ?? 0,
    maxMs: sorted[sorted.length - 1] ?? 0,
  };
}

/** Frames skipped at the start, while the first moves warm up the renderer. */
const WARM_UP_FRAMES = 10;

/**
 * Pans along a circle of `radius` CSS px for `durationMs`, calling `moveTo(dx, dy)` with the
 * offset from the start once per animation frame, and resolves with the frame time statistics.
 */
export function runPanCheck(
  moveTo: (dx: number, dy: number) => void,
  durationMs = 5000,
  radius = 200,
): Promise<FrameStats> {
  return new Promise((resolve) => {
    const times: number[] = [];
    let start: number | null = null;
    let last: number | null = null;
    let frame = 0;
    const step = (now: number) => {
      start ??= now;
      if (last !== null && frame > WARM_UP_FRAMES) {
        times.push(now - last);
      }
      last = now;
      frame++;
      const t = (now - start) / durationMs;
      if (t >= 1) {
        moveTo(0, 0);
        resolve(frameStats(times));
        return;
      }
      // Two full circles, starting and ending at the start position.
      const angle = t * 4 * Math.PI;
      moveTo(radius * Math.sin(angle), radius * (1 - Math.cos(angle)) * 0.5);
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });
}

/** Share of the scroll range at time `t` (0 to 1): two sweeps from the top to the end and back. */
export function scrollSweep(t: number): number {
  const phase = (Math.min(Math.max(t, 0), 1) * 2) % 1;
  return phase < 0.5 ? phase * 2 : 2 - phase * 2;
}

/**
 * Scrolls `element` along {@link scrollSweep} for `durationMs`, one step per animation frame,
 * and resolves with the frame time statistics (T1.7 table check). Restores the scroll position.
 */
export function runScrollCheck(element: HTMLElement, durationMs = 5000): Promise<FrameStats> {
  const startTop = element.scrollTop;
  return new Promise((resolve) => {
    const times: number[] = [];
    let start: number | null = null;
    let last: number | null = null;
    let frame = 0;
    const step = (now: number) => {
      start ??= now;
      if (last !== null && frame > WARM_UP_FRAMES) {
        times.push(now - last);
      }
      last = now;
      frame++;
      const t = (now - start) / durationMs;
      if (t >= 1) {
        element.scrollTop = startTop;
        resolve(frameStats(times));
        return;
      }
      element.scrollTop = scrollSweep(t) * (element.scrollHeight - element.clientHeight);
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });
}
