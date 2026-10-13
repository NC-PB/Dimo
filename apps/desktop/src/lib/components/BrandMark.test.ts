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
  it("shows the product name next to a decorative mark in the text color", () => {
    component = mount(BrandMark, { target: document.body, props: { size: 24 } });
    expect(document.body.textContent?.trim()).toBe("Dimo");
    const svg = document.querySelector("svg");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.getAttribute("height")).toBe("24");
    // Outline follows the theme's text color, the dot keeps the brand orange.
    expect(svg?.querySelector("rect")?.getAttribute("stroke")).toBe("currentColor");
    expect(svg?.querySelector("circle")?.getAttribute("fill")).toBe("#e85d04");
  });
});
