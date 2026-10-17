// @vitest-environment happy-dom
/**
 * File > About shows the copyright notice and the full license text of every bundled font and
 * of the icon set (M3 decision 29, T3.16 acceptance; SIL OFL 1.1 condition).
 */
import { flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BUNDLED, DIMO_LICENSE, DIMO_NOTICE } from "./about";
import AboutPane from "./shell/AboutPane.svelte";
import packageJson from "../../package.json";
import cargoLock from "../../../../Cargo.lock?raw";
import pdfiumToml from "../../../../scripts/pdfium.toml?raw";
import THIRD_PARTY_LICENSES from "../../../../THIRD_PARTY_LICENSES?raw";

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

  it("includes the Apache 2.0 license and the NOTICE of DimoMark", () => {
    expect(DIMO_LICENSE).toContain("Apache License");
    expect(DIMO_LICENSE).toContain("Version 2.0, January 2004");
    expect(DIMO_NOTICE).toContain("DimoMark");
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

describe("Third party license list (T3.23)", () => {
  /** The components the list names, as "name version". */
  function listed(kind: "Rust crate" | "npm package"): string[] {
    return [...THIRD_PARTY_LICENSES.matchAll(/^\* (\S+) (\S+) \((Rust crate|npm package)\)$/gm)]
      .filter((match) => match[3] === kind)
      .map((match) => `${match[1]} ${match[2]}`);
  }

  // A cheap guard next to `scripts/third-party-licenses.mjs --check` in check.sh: a changed
  // dependency fails here too until the list is generated again.
  it("lists Rust crates in the versions of Cargo.lock", () => {
    const locked = new Set(
      [...cargoLock.matchAll(/^name = "([^"]+)"\nversion = "([^"]+)"/gm)].map(
        (match) => `${match[1]} ${match[2]}`,
      ),
    );
    const crates = listed("Rust crate");
    expect(crates.length).toBeGreaterThan(100);
    for (const crate of crates) expect(locked, crate).toContain(crate);
    for (const name of ["tauri", "typst", "pdfium-render", "rust_decimal"]) {
      const version = [...locked].filter((entry) => entry.startsWith(`${name} `));
      expect(version, name).toHaveLength(1);
      expect(crates, name).toContain(version[0]);
    }
  });

  it("lists the bundled npm packages in their pinned versions", () => {
    const pinned: Record<string, string> = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    };
    const packages = listed("npm package");
    for (const name of [
      "@lucide/svelte",
      "@tanstack/svelte-table",
      "@tanstack/virtual-core",
      "@tauri-apps/api",
      "@inlang/paraglide-js",
      "bits-ui",
      "svelte",
      "tailwindcss",
      "vite",
    ]) {
      expect(packages, name).toContain(`${name} ${pinned[name]}`);
    }
  });

  it("includes PDFium in the pinned version with its bundled licenses", () => {
    const version = /^version = "([^"]+)"/m.exec(pdfiumToml)?.[1];
    expect(THIRD_PARTY_LICENSES).toContain(`PDFium ${version ?? "?"} (`);
    expect(THIRD_PARTY_LICENSES).toContain("licenses/freetype.txt");
  });

  it("states the number of crates and packages it lists", () => {
    const header =
      /^- the Rust standard library \S+ and (\d+) Rust crates.*\n- (\d+) npm packages/m.exec(
        THIRD_PARTY_LICENSES,
      );
    // A component under two licenses (AND) is listed in both groups.
    expect(Number(header?.[1])).toBe(new Set(listed("Rust crate")).size);
    expect(Number(header?.[2])).toBe(new Set(listed("npm package")).size);
  });

  it("shows the whole list on the About pane", async () => {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(AboutPane, { target });
    flushSync();
    await vi.waitFor(() =>
      expect(target?.querySelector('[data-license="third-party"]')).not.toBeNull(),
    );
    const shown = target.querySelector<HTMLElement>('[data-license="third-party"]');
    expect(shown?.textContent).toBe(THIRD_PARTY_LICENSES);
  });
});

describe("Copy diagnostics (D-06, R.10)", () => {
  function mountWith(fetchDiagnostics: () => Promise<unknown>): HTMLElement {
    target = document.createElement("div");
    document.body.append(target);
    component = mount(AboutPane, {
      target,
      props: { fetchDiagnostics: fetchDiagnostics as never },
    });
    flushSync();
    return target;
  }

  async function click(root: HTMLElement): Promise<string> {
    root.querySelector<HTMLButtonElement>('[data-testid="copy-diagnostics"]')?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
    return root.querySelector('[data-testid="diagnostics-status"]')?.textContent?.trim() ?? "";
  }

  it("puts the text from Rust on the clipboard", async () => {
    const written: string[] = [];
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
    });
    let calls = 0;
    const root = mountWith(() => {
      calls += 1;
      return Promise.resolve({ status: "ok", data: `DimoMark diagnostics ${calls}\n` });
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    // WebKit allows the write only inside the click: the text loaded with the pane is written
    // before anything waits for Rust.
    root.querySelector<HTMLButtonElement>('[data-testid="copy-diagnostics"]')?.click();
    expect(written).toEqual(["DimoMark diagnostics 1\n"]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(await click(root)).toBe("Diagnostics copied to the clipboard.");
    expect(written).toEqual(["DimoMark diagnostics 1\n", "DimoMark diagnostics 2\n"]);
  });

  it("shows why copying failed", async () => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: () => Promise.reject(new Error("denied")) },
    });
    const root = mountWith(() => Promise.resolve({ status: "ok", data: "x" }));
    expect(await click(root)).toBe(
      "Could not copy the diagnostics: The clipboard is not available.",
    );
    unmount(component as never);
    component = null;
    target?.remove();
    const failed = mountWith(() =>
      Promise.resolve({ status: "error", error: { kind: "task_failed", message: "boom" } }),
    );
    expect(await click(failed)).toBe(
      "Could not copy the diagnostics: A background task stopped unexpectedly. Try again.",
    );
  });
});
