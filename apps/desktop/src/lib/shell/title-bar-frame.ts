import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isMacPlatform } from "$lib/shortcuts";

/**
 * Window frame of the title bar (T3.22). On macOS the window has no native title bar of its own
 * (`titleBarStyle: "Overlay"` in tauri.conf.json): the window buttons sit inside the app's title
 * bar, which leaves room for them and moves the window. Windows and Linux keep the native frame,
 * so the title bar there is plain content.
 *
 * `trafficLightPosition` in tauri.conf.json centers the window buttons in the 44 px title bar
 * (`h-11`); the 88 px left padding of TitleBar.svelte clears them with some room to spare.
 */
export interface TitleBarFrame {
  /** The empty title bar area moves the window, a double click zooms it. */
  drag: boolean;
  /** Room on the left for the window buttons. */
  windowButtons: boolean;
}

/**
 * In full screen macOS hides the window buttons together with the menu bar, so the title bar
 * needs no room for them.
 */
export function titleBarFrame(mac: boolean = isMacPlatform(), fullScreen = false): TitleBarFrame {
  return { drag: mac, windowButtons: mac && !fullScreen };
}

/**
 * Calls `onChange` with the full screen state of the window now and after every resize; returns
 * the function that stops listening. Outside the app (tests) the window is never full screen.
 * The window size cannot tell: on a display with a camera notch a full screen window is shorter
 * than the screen. The window's own resize event is used, not the page's: macOS clears the full
 * screen state only after the last page resize, and the window sends one more resize then.
 */
export function watchFullScreen(onChange: (fullScreen: boolean) => void): () => void {
  if (!isTauri()) {
    return () => {};
  }
  const win = getCurrentWindow();
  const update = () => {
    win.isFullscreen().then(onChange, () => {});
  };
  update();
  let stopped = false;
  let unlisten: (() => void) | null = null;
  win.onResized(update).then(
    (stop) => {
      if (stopped) {
        stop();
      } else {
        unlisten = stop;
      }
    },
    () => {},
  );
  return () => {
    stopped = true;
    unlisten?.();
  };
}
