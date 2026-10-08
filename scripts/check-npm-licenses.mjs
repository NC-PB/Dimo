#!/usr/bin/env node
// npm license check (NFR-MNT-04, AGENTS.md rule 10). Plain Node, no dependencies.
// Reads `pnpm licenses list --json` for the installed packages and fails on any license that is
// neither on the allow list nor an accepted exception. The allow list mirrors deny.toml.
// Usage: node scripts/check-npm-licenses.mjs   (run `pnpm install` first)
import { execFileSync } from "node:child_process";

// Keep in sync with [licenses] allow in deny.toml. MPL-2.0 is not here on purpose: it is only
// accepted for the build time exception below.
const ALLOWED = new Set([
  "MIT",
  "MIT-0",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "BSL-1.0",
  "CC0-1.0",
  "Unicode-3.0",
  "Unicode-DFS-2016",
]);

// Accepted build time exceptions (decision log in docs/plan/STATUS.md, 2026-10-08).
// None of this code ships in the app bundle. Each entry: package name pattern -> license.
const EXCEPTIONS = [
  { name: /^lightningcss(-.+)?$/, license: "MPL-2.0" }, // used by Tailwind and Vite
  { name: /^@lix-js\/sdk-.+$/, license: "Unknown" }, // unlabeled binaries, repository is MIT (Paraglide)
  { name: /^minimatch$/, license: "BlueOak-1.0.0" },
  { name: /^tslib$/, license: "0BSD" },
];

// Evaluates an SPDX expression: OR needs one allowed side, AND needs all. "X WITH Y" is one id.
function satisfies(expr, ok) {
  const tokens = expr.replace(/\(/g, " ( ").replace(/\)/g, " ) ").split(/\s+/).filter(Boolean);
  let pos = 0;
  const parseOr = () => {
    let res = parseAnd();
    while (tokens[pos] === "OR") {
      pos++;
      const rhs = parseAnd();
      res = res || rhs;
    }
    return res;
  };
  const parseAnd = () => {
    let res = parseAtom();
    while (tokens[pos] === "AND") {
      pos++;
      const rhs = parseAtom();
      res = res && rhs;
    }
    return res;
  };
  const parseAtom = () => {
    if (tokens[pos] === "(") {
      pos++;
      const res = parseOr();
      pos++; // ")"
      return res;
    }
    let id = tokens[pos++];
    if (tokens[pos] === "WITH") {
      id += " WITH " + tokens[pos + 1];
      pos += 2;
    }
    return ok(id);
  };
  const result = parseOr();
  return pos === tokens.length ? result : false;
}

const out = execFileSync("pnpm", ["licenses", "list", "--json"], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
  shell: process.platform === "win32", // pnpm is a .cmd shim on Windows
});
const byLicense = JSON.parse(out);

const bad = [];
let count = 0;
for (const [license, pkgs] of Object.entries(byLicense)) {
  for (const pkg of pkgs) {
    count++;
    const excepted = EXCEPTIONS.some((e) => e.license === license && e.name.test(pkg.name));
    if (!excepted && !satisfies(license, (id) => ALLOWED.has(id))) {
      bad.push(`${pkg.name}@${pkg.versions.join(",")}: ${license}`);
    }
  }
}

if (bad.length > 0) {
  console.error("npm packages with licenses outside the allow list:");
  for (const line of bad) console.error("  " + line);
  console.error(
    "Allow list: scripts/check-npm-licenses.mjs (mirrors deny.toml). Exceptions need an owner decision.",
  );
  process.exit(1);
}
console.log(`npm licenses ok (${count} packages)`);
