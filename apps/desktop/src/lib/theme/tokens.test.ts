import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// T3.14, M3 decisions 24 and 28: WCAG 2.2 AA contrast of the color tokens in both themes.
// The colors are read from tokens.css itself, the only place where they are defined.
// Read from disk: Vitest does not process CSS, so a `?raw` import would come back empty.
const tokensCss = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");

type Rgba = { r: number; g: number; b: number; a: number };
type Tokens = Map<string, Rgba>;

/** The `--dimo-*` colors declared directly in the first block that opens with `selector {`. */
function block(selector: string): Tokens {
  const start = tokensCss.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`no block ${selector}`);
  const body = tokensCss.slice(start, tokensCss.indexOf("}", start));
  const tokens: Tokens = new Map();
  for (const [, name = "", value = ""] of body.matchAll(/--dimo-([a-z-]+):\s*([^;]+);/g)) {
    const color = parseColor(value.trim());
    if (color !== null) tokens.set(name, color);
  }
  return tokens;
}

function parseColor(value: string): Rgba | null {
  const hex = /^#([0-9a-f]{6})$/i.exec(value)?.[1];
  if (hex !== undefined) {
    const n = Number.parseInt(hex, 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a: 1 };
  }
  const rgb = /^rgb\((\d+) (\d+) (\d+) \/ ([\d.]+)\)$/.exec(value);
  if (rgb !== null) {
    const [r, g, b, a] = rgb.slice(1).map(Number);
    return { r: r ?? 0, g: g ?? 0, b: b ?? 0, a: a ?? 1 };
  }
  return null; // var() aliases
}

function color(tokens: Tokens, name: string): Rgba {
  const c = tokens.get(name);
  if (c === undefined) throw new Error(`token --dimo-${name} missing`);
  return c;
}

/** `top` painted over the opaque `ground`. */
function over(top: Rgba, ground: Rgba): Rgba {
  const mix = (t: number, g: number) => t * top.a + g * (1 - top.a);
  return { r: mix(top.r, ground.r), g: mix(top.g, ground.g), b: mix(top.b, ground.b), a: 1 };
}

function linear(channel: number): number {
  const s = channel / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

function luminance({ r, g, b }: Rgba): number {
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

function contrast(a: Rgba, b: Rgba): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/** CIELAB (D65), to check that status colors do not look like the accent. */
function lab(c: Rgba): [number, number, number] {
  const [r, g, b] = [linear(c.r), linear(c.g), linear(c.b)];
  const x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
  const y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  const z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
  const f = (t: number) => (t > 216 / 24389 ? Math.cbrt(t) : (24389 / 27 / 116) * t + 16 / 116);
  return [116 * f(y) - 16, 500 * (f(x) - f(y)), 200 * (f(y) - f(z))];
}

function deltaE(a: Rgba, b: Rgba): number {
  const [la, aa, ba] = lab(a);
  const [lb, ab, bb] = lab(b);
  return Math.hypot(la - lb, aa - ab, ba - bb);
}

const light = block(":root");
const themes: [string, Tokens][] = [
  ["light", light],
  // Dark redefines the UI colors; the drawing overlay colors are inherited from light.
  ["dark", new Map([...light, ...block(':root[data-theme="dark"]')])],
];

/** Foreground, background, minimum ratio. 4.5 for text, 3 for marks and control outlines. */
const PAIRS: [string, string, number][] = [
  ["text", "bg", 4.5],
  ["text", "surface", 4.5],
  ["text", "sunken", 4.5],
  ["text", "hover", 4.5],
  ["text-muted", "bg", 4.5],
  ["text-muted", "surface", 4.5],
  ["text-muted", "hover", 4.5],
  ["accent-ink", "accent-strong", 4.5],
  ["accent-ink", "danger", 4.5],
  ["accent-strong", "surface", 4.5],
  ["accent-strong", "bg", 4.5],
  ["accent", "surface", 3],
  ["accent", "bg", 3],
  ["danger", "surface", 4.5],
  ["danger", "bg", 4.5],
  ["success", "surface", 4.5],
  ["success", "bg", 4.5],
  ["warning", "surface", 4.5],
  ["warning", "bg", 4.5],
];

describe("color tokens (T3.14, WCAG 2.2 AA)", () => {
  it("reads the new tokens in both themes", () => {
    const names = [
      "sunken",
      "hover",
      "border-strong",
      "accent-strong",
      "accent-ink",
      "accent-soft",
    ];
    for (const [, tokens] of themes) {
      for (const name of names) expect(tokens.has(name), name).toBe(true);
    }
  });

  for (const [theme, tokens] of themes) {
    for (const [fg, bg, min] of PAIRS) {
      it(`${theme}: ${fg} on ${bg} reaches ${min}:1`, () => {
        expect(contrast(color(tokens, fg), color(tokens, bg))).toBeGreaterThanOrEqual(min);
      });
    }

    it(`${theme}: text stays readable on the accent-soft selection tint`, () => {
      const tint = over(color(tokens, "accent-soft"), color(tokens, "surface"));
      expect(contrast(color(tokens, "text"), tint)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(color(tokens, "text-muted"), tint)).toBeGreaterThanOrEqual(4.5);
    });

    // Decision 24: pass, fail and warnings never look like the orange accent. A CIELAB
    // distance above 20 is a clearly different color, not a shade of the same one.
    it(`${theme}: status colors are distinct from the accent`, () => {
      for (const status of ["danger", "success", "warning"]) {
        for (const accent of ["accent", "accent-strong"]) {
          const distance = deltaE(color(tokens, status), color(tokens, accent));
          expect(distance, `${status} vs ${accent}`).toBeGreaterThan(20);
        }
      }
    });
  }
});
