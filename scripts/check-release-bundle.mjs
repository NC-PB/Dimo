#!/usr/bin/env node
// Release bundle check (MF F8). Plain Node, no dependencies.
// Builds the production frontend bundle into a temporary folder and fails if it contains code of
// the development tools in apps/desktop/src/lib/dev (UI script, dev panel, dummy balloons, frame
// time checks). Those modules are loaded only through `import.meta.env.DEV` guarded dynamic
// imports, so Vite drops them from production builds; this check keeps it that way.
// Each marker must also occur in the dev sources, so the check cannot pass because a marker was
// renamed there.
// Usage: node scripts/check-release-bundle.mjs   (run `pnpm install` first)
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ROOT = join(import.meta.dirname, "..");
const DESKTOP = join(ROOT, "apps", "desktop");
const DEV_SOURCES = join(DESKTOP, "src", "lib", "dev");

/** Text that only the development tools contain: log lines and fields of the dev store. */
const MARKERS = [
  "script queued", // dev-tools.svelte.ts, before a UI script starts
  "script done", // ui-script.ts, the end mark scripts/e2e-smoke.sh waits for
  "real pointer events during the script", // ui-script.ts
  "tableScroller", // DevToolsStore, set by the characteristic table
  "commitTimes", // DevToolsStore, set by the measurement grid
];

/** Every file below `dir`, with its path. */
function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

function textOf(paths) {
  return paths.map((path) => ({ path, text: readFileSync(path, "utf8") }));
}

const sources = textOf(files(DEV_SOURCES).filter((p) => /\.(ts|svelte)$/.test(p)));
const missing = MARKERS.filter((marker) => !sources.some((s) => s.text.includes(marker)));
if (missing.length > 0) {
  console.error(`markers not found in ${DEV_SOURCES}: ${missing.join(", ")}`);
  console.error("update MARKERS in scripts/check-release-bundle.mjs");
  process.exit(1);
}

const out = mkdtempSync(join(tmpdir(), "dimo-bundle-"));
let failed = false;
try {
  const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
  execFileSync(
    pnpm,
    ["exec", "vite", "build", "--outDir", out, "--emptyOutDir", "--logLevel", "warn"],
    {
      cwd: DESKTOP,
      stdio: ["ignore", "inherit", "inherit"],
      shell: process.platform === "win32",
    },
  );
  const bundle = textOf(files(out).filter((p) => /\.(js|html|css)$/.test(p)));
  for (const marker of MARKERS) {
    for (const file of bundle.filter((f) => f.text.includes(marker))) {
      console.error(`dev code in the release bundle: "${marker}" in ${file.path}`);
      failed = true;
    }
  }
  if (!failed) {
    console.log(
      `release bundle: no dev code (${String(bundle.length)} files, ${String(MARKERS.length)} markers)`,
    );
  }
} finally {
  rmSync(out, { recursive: true, force: true });
}
process.exit(failed ? 1 : 0);
