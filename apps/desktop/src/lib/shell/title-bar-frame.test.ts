import { describe, expect, it } from "vitest";
import { titleBarFrame } from "./title-bar-frame";

// T3.22: one title bar on macOS, native frame on Windows and Linux.
describe("title bar frame", () => {
  it("moves the window and leaves room for the window buttons on macOS", () => {
    expect(titleBarFrame(true, false)).toEqual({ drag: true, windowButtons: true });
  });

  it("needs no room for the window buttons in full screen", () => {
    expect(titleBarFrame(true, true)).toEqual({ drag: true, windowButtons: false });
  });

  it("is plain content inside the native frame elsewhere", () => {
    expect(titleBarFrame(false, false)).toEqual({ drag: false, windowButtons: false });
    expect(titleBarFrame(false, true)).toEqual({ drag: false, windowButtons: false });
  });
});
