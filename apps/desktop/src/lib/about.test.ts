// @vitest-environment happy-dom
/**
 * File > About shows the copyright notice and the full license text of every bundled font and
 * of the icon set (M3 decision 29, T3.16 acceptance; SIL OFL 1.1 condition).
 */
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import { BUNDLED, DIMO_LICENSE, DIMO_NOTICE } from "./about";
import AboutPane from "./shell/AboutPane.svelte";
import packageJson from "../../package.json";

/** Every font file in data/fonts, by file name. */
const FONT_FILES = Object.keys(
  import.meta.glob("../../../../data/fonts/*.{ttf,otf,woff,woff2}", {
    query: "?url",
    import: "default",
    eager: true,
  }),
).map((path) => path.split("/").pop() ?? "");

/** The OFL texts next to the fonts, by file name. */
const OFL_TEXTS = Object.fromEntries(
  Object.entries(
    import.meta.glob<string>("../../../../data/fonts/*-OFL.txt", {
      query: "?raw",
      import: "default",
      eager: true,
    }),
  ).map(([path, text]) => [path.split("/").pop() ?? "", text]),
);

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement | null = null;

afterEach(() => {
  if (component) {
    void unmount(component);
    component = null;
  }
  target?.remove();
});

describe("About licenses (M3 decision 29)", () => {
  it("covers every font file in data/fonts exactly once", () => {
    expect(FONT_FILES.length).toBeGreaterThanOrEqual(6);
    for (const file of FONT_FILES) {
      const owners = BUNDLED.filter((a) => a.files.includes(file));
      expect(
        owners.map((a) => a.id),
        file,
      ).toHaveLength(1);
    }
    for (const asset of BUNDLED) {
      for (const file of asset.files) {
        expect(FONT_FILES, `${asset.id} lists ${file}`).toContain(file);
      }
    }
  });

  it("embeds the full license text of each font family, unchanged", () => {
    for (const asset of BUNDLED.filter((a) => a.files.length > 0)) {
      if (asset.license.startsWith("SIL Open Font License")) {
        const family = asset.files[0]?.split("-")[0] ?? "";
        const ofl = OFL_TEXTS[`${family}-OFL.txt`];
        expect(ofl, `${family}-OFL.txt`).toBeDefined();
        expect(asset.text).toBe(ofl);
        expect(asset.text).toContain("SIL OPEN FONT LICENSE Version 1.1");
        // The copyright notice of the license file is shown as well.
        expect(asset.text).toContain(asset.copyright);
      } else {
        expect(asset.license).toBe("Apache License 2.0");
        expect(asset.text).toBe(DIMO_LICENSE);
      }
    }
    // Every license file next to the fonts is shown.
    for (const [file, text] of Object.entries(OFL_TEXTS)) {
      expect(
        BUNDLED.some((a) => a.text === text),
        file,
      ).toBe(true);
    }
  });

  it("includes the Lucide license with the ISC and the MIT text", () => {
    const lucide = BUNDLED.find((a) => a.id === "lucide");
    expect(lucide?.text).toContain("ISC License");
    expect(lucide?.text).toContain("Lucide Icons and Contributors");
    expect(lucide?.text).toContain("The MIT License (MIT)");
    expect(lucide?.text).toContain("Cole Bemis");
    // The version shown is the pinned one, so a dependency bump updates About too.
    expect(lucide?.name).toContain(packageJson.dependencies["@lucide/svelte"]);
  });

  it("includes the Apache 2.0 license and the NOTICE of Dimo", () => {
    expect(DIMO_LICENSE).toContain("Apache License");
    expect(DIMO_LICENSE).toContain("Version 2.0, January 2004");
    expect(DIMO_NOTICE).toContain("Dimo");
    expect(DIMO_NOTICE).toContain("SIL Open Font License");
  });

  it("shows every text in full on the About pane", () => {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(AboutPane, { target });
    flushSync();
    const shown = (id: string) =>
      target?.querySelector<HTMLElement>(`[data-license="${id}"]`)?.textContent ?? "";
    expect(shown("dimo")).toBe(DIMO_LICENSE);
    expect(shown("notice")).toBe(DIMO_NOTICE);
    for (const asset of BUNDLED) {
      expect(shown(asset.id), asset.id).toBe(asset.text);
      expect(target.textContent).toContain(asset.copyright);
    }
  });
});
