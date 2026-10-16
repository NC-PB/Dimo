// @vitest-environment happy-dom
import { mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import BrandMark from "./BrandMark.svelte";

let component: ReturnType<typeof mount> | null = null;

afterEach(() => {
  if (component !== null) void unmount(component);
  component = null;
  document.body.innerHTML = "";
});

describe("BrandMark (M3 decision 24)", () => {
  it("shows the wordmark next to a decorative mark in the text color (R.11)", () => {
    component = mount(BrandMark, { target: document.body, props: { size: 24 } });
    expect(document.body.textContent?.trim()).toBe("DimoMark");
    expect(document.querySelector("[role='img']")?.getAttribute("aria-label")).toBe("DimoMark");
    expect(document.querySelector(".name .text-accent-strong")?.textContent).toBe("Mark");
    const svg = document.querySelector("svg");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.getAttribute("height")).toBe("24");
    // Check and dimension follow the theme's text color, the balloon keeps the brand orange.
    expect(svg?.querySelector("path")?.getAttribute("stroke")).toBe("currentColor");
    expect(svg?.querySelector("circle")?.getAttribute("stroke")).toBe("#e85d04");
  });
});
