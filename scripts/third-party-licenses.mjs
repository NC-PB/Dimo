#!/usr/bin/env node
// Third party license list (T3.23, NFR-MNT-04, AGENTS.md rules 9, 10, 11). Plain Node, no
// dependencies, no network of its own.
//
// Writes THIRD_PARTY_LICENSES in the repository root: every third party component that ships in
// DimoMark, grouped by license, each distinct license text once.
//
// - Rust crates: the normal (not build, not dev) dependency graph of dimo-desktop from
//   `cargo metadata --filter-platform` for each release target, joined. Proc macro crates and
//   their dependencies are included: they do not link into the binary, but the code they generate
//   does, and listing too much is safe while listing too little is not.
// - npm packages: the packages whose code Vite puts into the production bundle, read from the
//   module list of a real build, plus the packages that generate code or CSS into it
//   (EXTRA_NPM). Build tools and dev dependencies whose code does not ship are left out.
// - The system libraries and tools the Linux AppImage bundles, from scripts/appimage/ (see the
//   README there): the Debian copyright file of each Ubuntu source package, unchanged, and the
//   texts of /usr/share/common-licenses/ they refer to, from scripts/license-texts/.
// - PDFium (scripts/pdfium.toml) and the code it bundles, from vendor/pdfium/<target>/
//   (./scripts/fetch-pdfium.sh). The archives differ per platform only in line ends, encoding
//   and comment markers; the texts are normalized, and every fetched target must give the same.
//
// License texts come from the files of each package. A package without a license file gets the
// standard text from scripts/license-texts/ (see the README there). Copyright lines are taken
// out of the texts and listed with each component, so equal texts merge.
//
// Usage: node scripts/third-party-licenses.mjs           write THIRD_PARTY_LICENSES
//        node scripts/third-party-licenses.mjs --check   fail if the file is not up to date
// Needs `pnpm install`, the crates of Cargo.lock (cargo fetches them) and a fetched PDFium.
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const ROOT = join(import.meta.dirname, "..");
const DESKTOP = join(ROOT, "apps", "desktop");
const OUTPUT = join(ROOT, "THIRD_PARTY_LICENSES");
const STANDARD_TEXTS = join(ROOT, "scripts", "license-texts");
const PDFIUM_VENDOR = join(ROOT, "vendor", "pdfium");
const APPIMAGE = join(ROOT, "scripts", "appimage");

/**
 * Files in /usr/share/common-licenses/ of Debian and Ubuntu, by the SPDX id of their text in
 * scripts/license-texts/. `GPL` and `LGPL` are links to version 3 there; some copyright files
 * write `GPL-2.0` or `GPL-3.0` for the same files.
 */
const COMMON_LICENSES = {
  "Apache-2.0": "Apache-2.0",
  Artistic: "Artistic-1.0-Perl",
  "CC0-1.0": "CC0-1.0",
  "GFDL-1.2": "GFDL-1.2-only",
  "GFDL-1.3": "GFDL-1.3-only",
  GPL: "GPL-3.0-only",
  "GPL-1": "GPL-1.0-only",
  "GPL-2": "GPL-2.0-only",
  "GPL-2.0": "GPL-2.0-only",
  "GPL-3": "GPL-3.0-only",
  "GPL-3.0": "GPL-3.0-only",
  LGPL: "LGPL-3.0-only",
  "LGPL-2": "LGPL-2.0-only",
  "LGPL-2.1": "LGPL-2.1-only",
  "LGPL-3": "LGPL-3.0-only",
  "MPL-1.1": "MPL-1.1",
  "MPL-2.0": "MPL-2.0",
};

/** Rust targets of the release builds (.github/workflows/release.yml). */
const RUST_TARGETS = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "x86_64-unknown-linux-gnu",
];

/**
 * npm packages that ship without showing up as a bundled module: Paraglide compiles its runtime
 * into src/lib/paraglide, Tailwind CSS writes its base styles into the CSS, Vite adds its module
 * preload helper.
 */
const EXTRA_NPM = ["@inlang/paraglide-js", "tailwindcss", "vite"];

/**
 * Licenses in the order a choice prefers them ("MIT OR Apache-2.0" lists MIT). Mirrors the allow
 * list of deny.toml; a license outside it stops the script.
 */
const LICENSES = [
  ["MIT", "MIT License"],
  ["Apache-2.0", "Apache License 2.0"],
  ["Apache-2.0 WITH LLVM-exception", "Apache License 2.0 with LLVM Exceptions"],
  ["BSD-3-Clause", "BSD 3-Clause License"],
  ["BSD-2-Clause", "BSD 2-Clause License"],
  ["ISC", "ISC License"],
  ["Zlib", "zlib License"],
  ["BSL-1.0", "Boost Software License 1.0"],
  ["Unicode-3.0", "Unicode License v3"],
  ["Unicode-DFS-2016", "Unicode License Agreement, Data Files and Software (2016)"],
  ["MIT-0", "MIT No Attribution"],
  ["CC0-1.0", "Creative Commons Zero v1.0 Universal"],
  ["MPL-2.0", "Mozilla Public License 2.0"],
  ["CDLA-Permissive-2.0", "Community Data License Agreement Permissive 2.0"],
];
const LICENSE_NAME = new Map(LICENSES);
const LICENSE_RANK = new Map(LICENSES.map(([id], i) => [id, i]));

/** Recognizes a license text by phrases only that license contains. Order matters. */
const DETECT = [
  ["Apache-2.0 WITH LLVM-exception", (t) => /LLVM Exceptions to the Apache 2\.0 License/i.test(t)],
  ["Apache-2.0", (t) => /TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION/i.test(t)],
  ["MPL-2.0", (t) => /Mozilla Public License,? Version 2\.0/i.test(t)],
  ["BSL-1.0", (t) => /Boost Software License - Version 1\.0/i.test(t)],
  ["Unicode-3.0", (t) => /UNICODE LICENSE V3/i.test(t)],
  [
    "Unicode-DFS-2016",
    (t) => /UNICODE, INC\. LICENSE AGREEMENT - DATA FILES AND SOFTWARE/i.test(t),
  ],
  ["CC0-1.0", (t) => /CC0 1\.0 Universal/i.test(t)],
  [
    "CDLA-Permissive-2.0",
    (t) => /Community Data License Agreement - Permissive - Version 2\.0/i.test(t),
  ],
  [
    "MIT-0",
    (t) =>
      /Permission is hereby granted, free of charge/i.test(t) &&
      !/above copyright notice and this permission notice shall be included/i.test(t),
  ],
  ["MIT", (t) => /Permission is hereby granted, free of charge/i.test(t)],
  [
    "BSD-3-Clause",
    (t) =>
      /Redistribution and use in source and binary forms/i.test(t) &&
      /(Neither the name|names of its contributors|name of the copyright holder)/i.test(t),
  ],
  ["BSD-2-Clause", (t) => /Redistribution and use in source and binary forms/i.test(t)],
  ["ISC", (t) => /Permission to use, copy, modify, and(\/or)? distribute this software/i.test(t)],
  ["Zlib", (t) => /altered source versions must be plainly marked/i.test(t)],
];

/**
 * Every license a text contains. A more specific match hides the looser one it also fits:
 * MIT-0 hides MIT, BSD-3-Clause hides BSD-2-Clause, and the Unicode and Boost texts, which grant
 * permission without the MIT notice clause, hide MIT-0.
 */
function detectAll(text) {
  const flat = text.replace(/\s+/g, " ");
  const ids = DETECT.filter(([, test]) => test(flat)).map(([id]) => id);
  const hides = {
    "MIT-0": ["MIT"],
    "BSD-3-Clause": ["BSD-2-Clause"],
    "Apache-2.0 WITH LLVM-exception": ["Apache-2.0"],
    "Unicode-3.0": ["MIT-0"],
    "Unicode-DFS-2016": ["MIT-0"],
    "BSL-1.0": ["MIT-0"],
  };
  return ids.filter((id) => !ids.some((other) => hides[other]?.includes(id)));
}

/**
 * Remarks printed before a NOTICE file that names more than ships. Keep them true: they say what
 * DimoMark leaves out.
 */
const NOTICE_REMARKS = {
  "typst-assets":
    "DimoMark builds typst-assets without its `fonts` feature, so the Libertinus, New Computer\n" +
    "Modern and DejaVu fonts named below are not part of DimoMark. The Foxit fonts, ICC profiles\n" +
    "and HTML and MathML data are.",
};

/** File names that hold license texts or notices, in a package's top folder. */
const LICENSE_FILE = /^(licen[cs]e|copying|copyright|unlicen[cs]e)([-._].*)?$/i;
const NOTICE_FILE = /^notice([-._].*)?$/i;

// ---------------------------------------------------------------------------------------------
// Text helpers

/** Decodes UTF-8, or Latin-1 for the few old files that are not UTF-8. */
function readText(path) {
  const bytes = readFileSync(path);
  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    text = new TextDecoder("latin1").decode(bytes);
  }
  return tidy(text);
}

/** LF line ends, no BOM, no trailing spaces, no blank lines at the start or end. */
function tidy(text) {
  return text
    .replace(/^﻿/, "")
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((line) => line.replace(/[ \t]+$/, ""))
    .join("\n")
    .replace(/^\n+/, "")
    .replace(/\n+$/, "");
}

const COPYRIGHT = /^[\s*#;/-]*(copyright\b|\(c\)\s*\d|©)/i;
const NOT_A_NOTICE =
  /\[yyyy\]|\{yyyy\}|<year>|\[year\]|copyright (notice|holder|owner|license|statement|law|protection|and related|\[)|copyright\s*$|^\s*copyright\s*:?\s*$/i;

/** The copyright line of a standard text, with fields to fill in. */
const PLACEHOLDER = /<year>|\[yyyy\]|\{yyyy\}|\[year\]|<owner>|<copyright holders>/i;

/** Lines that state a copyright, as in the text. */
function copyrightLines(text) {
  return text
    .split("\n")
    .filter((line) => COPYRIGHT.test(line) && !NOT_A_NOTICE.test(line) && line.length < 240)
    .map((line) => line.replace(/^[\s*#;/-]+/, "").replace(/\s+/g, " "));
}

/** The license text without its copyright lines and, for Apache, without the appendix. */
function licenseBody(id, text) {
  let body = text
    .split("\n")
    .filter(
      (line) => !(COPYRIGHT.test(line) && (!NOT_A_NOTICE.test(line) || PLACEHOLDER.test(line))),
    )
    .join("\n");
  if (id.startsWith("Apache-2.0")) {
    const end = body.search(/END OF TERMS AND CONDITIONS/);
    if (end >= 0 && id === "Apache-2.0") {
      body = body.slice(0, end + "END OF TERMS AND CONDITIONS".length);
    }
  }
  return tidy(body.replace(/\n{3,}/g, "\n\n"));
}

/** Compares texts by their words only, so wrapping, titles in different case and quotes merge. */
function textKey(body) {
  return body
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .replace(/^\s*((the|mit|isc|license|licence)\s+)+/, "")
    .trim();
}

/** Byte order on code units, the same on every platform and locale. */
function compare(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** Compares versions part by part, numbers as numbers. */
function compareVersion(a, b) {
  const pa = a.split(/[.+-]/);
  const pb = b.split(/[.+-]/);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? "";
    const y = pb[i] ?? "";
    const nx = /^\d+$/.test(x) ? Number(x) : NaN;
    const ny = /^\d+$/.test(y) ? Number(y) : NaN;
    const c = !isNaN(nx) && !isNaN(ny) ? nx - ny : compare(x, y);
    if (c !== 0) return Math.sign(c);
  }
  return 0;
}

// ---------------------------------------------------------------------------------------------
// License expressions

/**
 * The licenses DimoMark uses a component under: for OR the most preferred allowed side, for AND
 * all sides. `MIT/Apache-2.0` (old Cargo syntax) counts as OR.
 */
function chooseLicenses(expression, who, shipped = new Set()) {
  const tokens = expression
    .replace(/\//g, " OR ")
    .replace(/\(/g, " ( ")
    .replace(/\)/g, " ) ")
    .split(/\s+/)
    .filter(Boolean);
  let pos = 0;
  const allowed = (ids) => ids !== null && ids.every((id) => LICENSE_RANK.has(id));
  // A side whose text the package ships comes first, then the preference order.
  const rank = (ids) =>
    Math.max(...ids.map((id) => LICENSE_RANK.get(id) + (shipped.has(id) ? 0 : LICENSES.length)));
  const parseOr = () => {
    let best = parseAnd();
    while (tokens[pos] === "OR") {
      pos++;
      const next = parseAnd();
      if (!allowed(best) || (allowed(next) && rank(next) < rank(best))) best = next;
    }
    return best;
  };
  const parseAnd = () => {
    let ids = parseAtom();
    while (tokens[pos] === "AND") {
      pos++;
      const next = parseAtom();
      ids = ids && next ? [...ids, ...next] : null;
    }
    return ids;
  };
  const parseAtom = () => {
    if (tokens[pos] === "(") {
      pos++;
      const ids = parseOr();
      pos++; // ")"
      return ids;
    }
    let id = tokens[pos++];
    if (tokens[pos] === "WITH") {
      id += " WITH " + tokens[pos + 1];
      pos += 2;
    }
    return [id];
  };
  const ids = parseOr();
  if (pos !== tokens.length || !allowed(ids)) {
    throw new Error(`${who}: license "${expression}" is not on the allow list of deny.toml`);
  }
  return [...new Set(ids)].sort((a, b) => LICENSE_RANK.get(a) - LICENSE_RANK.get(b));
}

// ---------------------------------------------------------------------------------------------
// Components

/**
 * A component: `kind` "Rust crate" or "npm package", its folder, and the licenses it is used
 * under. `licenseFile` is a file the package names as its license (Cargo `license-file`).
 */
function component(kind, name, version, dir, expression, authors, source, licenseFile = null) {
  const files = readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f) || NOTICE_FILE.test(f))
    .sort(compare);
  if (licenseFile && !files.includes(licenseFile)) files.push(licenseFile);
  const texts = files.map((f) => ({ file: f, text: readText(join(dir, f)) }));
  const who = `${name} ${version}`;
  const detected = texts
    .filter(({ file }) => !NOTICE_FILE.test(file))
    .map(({ text }) => ({ text, ids: detectAll(text) }));
  const shipped = new Set(detected.flatMap((d) => d.ids));
  const stated = expression || detected.find((d) => d.ids.length > 0)?.ids.join(" AND ");
  if (!stated) throw new Error(`${who}: no license stated and none recognized`);
  const ids = chooseLicenses(stated, who, shipped);
  const copyright = [
    ...new Set(
      texts
        .filter(({ file }) => !NOTICE_FILE.test(file))
        .flatMap(({ text }) => copyrightLines(text)),
    ),
  ];
  const notices = ids.some((id) => id.startsWith("Apache-2.0"))
    ? texts.filter(({ file }) => NOTICE_FILE.test(file)).map(({ text }) => text)
    : [];
  // A file with only this license wins over a file that holds several (chrono has one file
  // with the MIT and the Apache text); the combined file is shown whole.
  const licenses = ids.map((id) => {
    const own =
      detected.find((d) => d.ids.length === 1 && d.ids[0] === id) ??
      detected.find((d) => d.ids.includes(id));
    return { id, text: own ? own.text : standardText(id, who) };
  });
  return { kind, name, version, copyright, authors, source, notices, licenses };
}

/** The standard text of a license from scripts/license-texts/. */
function standardText(id, who) {
  const path = join(STANDARD_TEXTS, `${id.replace(/ /g, "_")}.txt`);
  if (!existsSync(path)) {
    throw new Error(`${who}: no ${id} text in the package and none in scripts/license-texts/`);
  }
  return readText(path);
}

/**
 * The Rust standard library (std, core, alloc), linked into every Rust program. It is MIT OR
 * Apache-2.0 like most crates; its MIT file states no copyright line, so the standard text is
 * used. The version is the pinned toolchain.
 */
function rustStd() {
  const toolchain = readFileSync(join(ROOT, "rust-toolchain.toml"), "utf8");
  return {
    kind: "Rust toolchain",
    name: "Rust standard library",
    version: toolchain.match(/^channel\s*=\s*"([^"]+)"/m)[1],
    copyright: [],
    authors: ["The Rust Project Developers"],
    source: "https://github.com/rust-lang/rust",
    notices: [],
    licenses: [{ id: "MIT", text: standardText("MIT", "Rust standard library") }],
  };
}

function rustCrates() {
  const byId = new Map();
  for (const target of RUST_TARGETS) {
    const meta = JSON.parse(
      execFileSync(
        "cargo",
        ["metadata", "--format-version", "1", "--locked", "--filter-platform", target],
        { cwd: ROOT, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
      ),
    );
    const packages = new Map(meta.packages.map((p) => [p.id, p]));
    const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
    const root = meta.packages.find((p) => p.name === "dimo-desktop" && p.source === null);
    const seen = new Set([root.id]);
    const queue = [root.id];
    while (queue.length > 0) {
      const node = nodes.get(queue.shift());
      for (const dep of node.deps) {
        if (!dep.dep_kinds.some((k) => k.kind === null) || seen.has(dep.pkg)) continue;
        seen.add(dep.pkg);
        queue.push(dep.pkg);
      }
    }
    for (const id of seen) {
      const p = packages.get(id);
      if (p.source !== null) byId.set(`${p.name} ${p.version}`, p); // workspace crates are ours
      if (p.name === "typst-assets" && nodes.get(id).features.includes("fonts")) {
        throw new Error("typst-assets now builds in its fonts: update NOTICE_REMARKS");
      }
    }
  }
  return [...byId.values()].map((p) =>
    component(
      "Rust crate",
      p.name,
      p.version,
      dirname(p.manifest_path),
      p.license,
      p.authors.map((a) => a.replace(/\s*<[^>]*>/g, "").trim()).filter(Boolean),
      p.repository,
      p.license_file ? p.license_file.split(/[\\/]/).pop() : null,
    ),
  );
}

/** Builds the frontend in a temporary folder and returns the folders of the bundled packages. */
async function npmPackages() {
  const require = createRequire(join(DESKTOP, "package.json"));
  const { build } = await import(pathToFileURL(require.resolve("vite")).href);
  const outDir = mkdtempSync(join(tmpdir(), "dimo-licenses-"));
  const modules = new Set();
  try {
    await build({
      root: DESKTOP,
      logLevel: "error",
      build: { outDir, emptyOutDir: true, write: false },
      plugins: [
        {
          name: "dimo-collect-modules",
          generateBundle(_options, bundle) {
            for (const chunk of Object.values(bundle)) {
              if (chunk.type !== "chunk") continue;
              for (const [id, info] of Object.entries(chunk.modules)) {
                if (info.renderedLength > 0) modules.add(id.replace(/\\/g, "/"));
              }
            }
          },
        },
      ],
    });
  } finally {
    rmSync(outDir, { recursive: true, force: true });
  }
  const dirs = new Set();
  for (const id of modules) {
    const match = id.match(/^(.*\/node_modules\/(?:@[^/]+\/)?[^/]+)\//);
    if (match) dirs.add(match[1]);
  }
  for (const name of EXTRA_NPM) {
    dirs.add(realpathSync(join(DESKTOP, "node_modules", name)).replace(/\\/g, "/"));
  }
  const byId = new Map();
  for (const dir of dirs) {
    const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    const author = typeof pkg.author === "string" ? pkg.author : pkg.author?.name;
    byId.set(
      `${pkg.name} ${pkg.version}`,
      component(
        "npm package",
        pkg.name,
        pkg.version,
        dir,
        typeof pkg.license === "string" ? pkg.license : pkg.license?.type,
        author ? [author.replace(/\s*[<(][^>)]*[>)]/g, "").trim()] : [],
        typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url,
      ),
    );
  }
  return [...byId.values()];
}

/** PDFium texts: LF line ends, no `//` comment markers (the macOS archive has them). */
function pdfiumText(path) {
  return tidy(
    readText(path)
      .split("\n")
      .map((line) => line.replace(/^\/\/ ?/, ""))
      .join("\n"),
  );
}

function pdfium() {
  const toml = readFileSync(join(ROOT, "scripts", "pdfium.toml"), "utf8");
  const version = toml.match(/^version\s*=\s*"([^"]+)"/m)[1];
  const targets = existsSync(PDFIUM_VENDOR)
    ? readdirSync(PDFIUM_VENDOR)
        .filter((t) => existsSync(join(PDFIUM_VENDOR, t, "licenses")))
        .sort(compare)
    : [];
  if (targets.length === 0) {
    throw new Error("PDFium is not fetched: run ./scripts/fetch-pdfium.sh first");
  }
  let result = null;
  for (const target of targets) {
    const dir = join(PDFIUM_VENDOR, target);
    const files = readdirSync(join(dir, "licenses")).sort(compare);
    const parts = [
      { file: "LICENSE", text: pdfiumText(join(dir, "LICENSE")) },
      ...files.map((f) => ({ file: `licenses/${f}`, text: pdfiumText(join(dir, "licenses", f)) })),
    ];
    const versionFile = readText(join(dir, "VERSION"));
    if (versionFile.replace(/\D+/g, ".").replace(/^\.|\.$/g, "") !== version) {
      throw new Error(`vendor/pdfium/${target} is not PDFium ${version}: fetch it again`);
    }
    if (result === null) {
      result = parts;
    } else if (JSON.stringify(parts) !== JSON.stringify(result)) {
      throw new Error(`the PDFium licenses of ${target} differ from ${targets[0]}: check by hand`);
    }
  }
  return { version, release: toml.match(/^release\s*=\s*"([^"]+)"/m)[1], parts: result };
}

/** Data lines of a tab separated file in scripts/appimage/, without `#` comments. */
function tsv(name) {
  return readText(join(APPIMAGE, name))
    .split("\n")
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => line.split("\t"));
}

/** License names of a Debian copyright file in the machine readable format, or null. */
function dep5Licenses(text) {
  // Two GStreamer files put a free text note before the Format line.
  if (!/^Format:.*copyright-format/m.test(text)) return null;
  const names = new Set();
  for (const stanza of text.split(/\n\s*\n/)) {
    if (!/^Files:/m.test(stanza)) continue;
    const license = stanza.match(/^License:[ \t]*(.*)$/m);
    if (license && license[1].trim() !== "") names.add(license[1].trim());
  }
  return [...names].sort(compare).join("; ");
}

/**
 * The Linux AppImage (T3.23): the Ubuntu source packages of the bundled system libraries with
 * their binary packages, files and Debian copyright file, the tools of the AppImage itself, and
 * the common license texts they refer to.
 */
function appimage() {
  const sources = new Map();
  const files = new Set();
  const fail = (why) => {
    throw new Error(`scripts/appimage/libs.txt: ${why}`);
  };
  for (const row of tsv("libs.txt")) {
    const [file, pkg, version, source, sourceVersion] = row;
    if (row.length !== 5 || row.some((field) => field === "" || field === "?")) {
      fail(`incomplete line for ${file}`);
    }
    if (files.has(file)) fail(`${file} is listed twice`);
    files.add(file);
    if (!sources.has(source)) {
      sources.set(source, { source, sourceVersion, packages: new Map(), files: [] });
    }
    const s = sources.get(source);
    if (s.sourceVersion !== sourceVersion) fail(`two versions of the source package ${source}`);
    if ((s.packages.get(pkg) ?? version) !== version) fail(`two versions of the package ${pkg}`);
    s.packages.set(pkg, version);
    s.files.push(file);
  }
  const summaries = new Map(tsv("sources.tsv"));
  const copyrightDir = join(APPIMAGE, "copyright");
  for (const file of readdirSync(copyrightDir)) {
    if (!sources.has(file)) throw new Error(`scripts/appimage/copyright/${file} is not bundled`);
  }
  for (const s of sources.values()) {
    const path = join(copyrightDir, s.source);
    if (!existsSync(path)) throw new Error(`scripts/appimage/copyright/${s.source} is missing`);
    s.copyright = readText(path);
    const dep5 = dep5Licenses(s.copyright);
    if (dep5 && summaries.has(s.source)) {
      throw new Error(
        `scripts/appimage/sources.tsv: ${s.source} needs no line, its file names its licenses`,
      );
    }
    s.licenses = dep5 || summaries.get(s.source);
    if (!s.licenses) {
      throw new Error(`scripts/appimage/sources.tsv: add the license summary of ${s.source}`);
    }
  }
  for (const source of summaries.keys()) {
    if (!sources.has(source))
      throw new Error(`scripts/appimage/sources.tsv: ${source} is not bundled`);
  }
  const tools = tsv("other.tsv").map(([name, part, license, textFile, copyright, source]) => ({
    name,
    part,
    license,
    copyright: copyright === "-" ? null : copyright,
    source,
    text: textFile.startsWith("other/") ? readText(join(APPIMAGE, textFile)) : null,
    common: textFile.startsWith("other/") ? null : textFile,
  }));
  // Common license texts: by SPDX id, with the names the copyright files use for it.
  const common = new Map();
  const use = (id, name) => {
    if (!common.has(id)) common.set(id, new Set());
    common.get(id).add(name);
  };
  for (const s of sources.values()) {
    for (const [, name] of s.copyright.matchAll(
      /\/usr\/share\/common-licenses\/([A-Za-z0-9.+-]*[A-Za-z0-9+])/g,
    )) {
      if (!COMMON_LICENSES[name]) {
        throw new Error(
          `${s.source}: unknown /usr/share/common-licenses/${name}, add it to COMMON_LICENSES`,
        );
      }
      // Debian has no GPL-2.0 or GPL-3.0 file; some copyright files still write it so.
      use(
        COMMON_LICENSES[name],
        /\.0$/.test(name)
          ? `${name} (as some copyright files write it)`
          : `/usr/share/common-licenses/${name}`,
      );
    }
  }
  for (const t of tools) if (t.common) use(t.common, t.name);
  return {
    sources: [...sources.values()].sort((a, b) => compare(a.source, b.source)),
    tools,
    common: [...common.keys()].sort(compare).map((id) => ({
      id,
      names: [...common.get(id)].sort(compare),
      text: standardText(id, "scripts/appimage"),
    })),
  };
}

// ---------------------------------------------------------------------------------------------
// Output

const RULE = "=".repeat(80);
const THIN = "-".repeat(80);

function render(components, pdf, linux) {
  // Group: license id -> distinct texts -> components.
  const groups = new Map();
  for (const c of components) {
    for (const { id, text } of c.licenses) {
      if (!groups.has(id)) groups.set(id, new Map());
      const body = licenseBody(id, text);
      const key = textKey(body);
      const variants = groups.get(id);
      if (!variants.has(key)) variants.set(key, { bodies: [], users: [] });
      variants.get(key).bodies.push({ c, body });
      variants.get(key).users.push(c);
    }
  }
  const order = (a, b) =>
    compare(a.name, b.name) || compareVersion(a.version, b.version) || compare(a.kind, b.kind);

  const rust = components.filter((c) => c.kind === "Rust crate").length;
  const npm = components.filter((c) => c.kind === "npm package").length;
  const std = components.find((c) => c.kind === "Rust toolchain");
  const out = [];
  out.push(
    "THIRD PARTY LICENSES",
    "",
    "DimoMark is licensed under the Apache License, Version 2.0 (see LICENSE and NOTICE).",
    "This file lists the third party components that ship in DimoMark, with their copyright",
    "notices and license texts. The bundled fonts and icons are listed in NOTICE and in",
    "File > About.",
    "",
    `- the Rust standard library ${std.version} and ${rust} Rust crates in the program (all platforms)`,
    `- ${npm} npm packages in the user interface`,
    `- PDFium ${pdf.version} and the code it bundles`,
    `- in the Linux AppImage only: ${linux.sources.length} Ubuntu 22.04 source packages of system`,
    "  libraries and the tools that make up the AppImage",
    "",
    "Components that offer a choice of licenses are listed under the license DimoMark uses.",
    "Copyright notices are taken from the license files of each component and listed with it;",
    "where a component states none, its authors or its source are listed. Each license text is",
    "shown once, after the components it covers.",
    "",
    "Generated by scripts/third-party-licenses.mjs. Do not edit by hand.",
  );

  const ids = [...groups.keys()].sort((a, b) => LICENSE_RANK.get(a) - LICENSE_RANK.get(b));
  for (const id of ids) {
    const variants = [...groups.get(id).values()];
    for (const v of variants) {
      v.users.sort(order);
      v.bodies.sort((a, b) => order(a.c, b.c));
    }
    variants.sort((a, b) => order(a.users[0], b.users[0]));
    const count = variants.reduce((n, v) => n + v.users.length, 0);
    out.push("", "", RULE, `${LICENSE_NAME.get(id)} (${id}), ${count} components`, RULE);
    variants.forEach((v, i) => {
      if (variants.length > 1) out.push("", `Text ${i + 1} of ${variants.length}`);
      out.push("");
      for (const c of v.users) {
        out.push(`* ${c.name} ${c.version} (${c.kind})`);
        if (c.copyright.length > 0) {
          for (const line of c.copyright) out.push(`    ${line}`);
        } else if (c.authors.length > 0) {
          out.push(`    Authors: ${c.authors.join(", ")}`);
        } else if (c.source && id !== "MPL-2.0") {
          out.push(`    No copyright notice stated. Source: ${c.source}`);
        }
        // MPL-2.0 section 3.2(a): tell recipients where the source code is.
        if (id === "MPL-2.0") out.push(`    Source code: ${c.source ?? "not stated"}`);
      }
      out.push("", THIN, v.bodies[0].body, THIN);
    });
  }

  const notices = components.filter((c) => c.notices.length > 0).sort(order);
  if (notices.length > 0) {
    out.push("", "", RULE, "NOTICE files of components under the Apache License 2.0", RULE);
    for (const c of notices) {
      out.push("", `* ${c.name} ${c.version} (${c.kind})`, "");
      if (NOTICE_REMARKS[c.name]) out.push(NOTICE_REMARKS[c.name], "");
      out.push(...c.notices, "");
    }
  }

  out.push(
    "",
    "",
    RULE,
    `PDFium ${pdf.version} (${pdf.release})`,
    RULE,
    "",
    "DimoMark ships the PDFium library as built by the pdfium-binaries project. The texts",
    "below come with that build: its own license, then the licenses of PDFium and of the code",
    "PDFium bundles, unchanged except for line ends and comment markers.",
  );
  for (const part of pdf.parts) {
    out.push("", THIN, part.file, THIN, part.text);
  }

  out.push(
    "",
    "",
    RULE,
    "Linux AppImage: bundled system libraries",
    RULE,
    "",
    "The AppImage for Linux brings the system libraries below with it, taken from the Ubuntu",
    "22.04 packages it is built on. The macOS and Windows builds and the Linux .deb package do",
    "not contain them. For each Ubuntu source package: its version, the binary packages and",
    "files DimoMark takes from it, the licenses its copyright file names, and the Debian",
    "copyright file itself, unchanged except for line ends and trailing spaces. The copyright",
    "file is what applies. The texts it refers to in /usr/share/common-licenses/ follow at the",
    "end of this part. The source code of each package is available from Ubuntu at the address",
    "given.",
  );
  for (const s of linux.sources) {
    out.push("", THIN, `* ${s.source} ${s.sourceVersion} (Ubuntu source package)`);
    const packages = [...s.packages].sort(([a], [b]) => compare(a, b));
    out.push(`    Packages: ${packages.map(([p, v]) => `${p} ${v}`).join(", ")}`);
    out.push(`    Licenses: ${s.licenses}`);
    out.push("    Files:", ...s.files.sort(compare).map((f) => `      ${f}`));
    out.push(
      `    Source code: https://launchpad.net/ubuntu/+source/${s.source}/${s.sourceVersion}`,
    );
    out.push(THIN, s.copyright);
  }
  out.push("", "", RULE, "Linux AppImage: the tools that make up the AppImage", RULE);
  for (const t of linux.tools) {
    out.push("", THIN, `* ${t.name} (${t.license})`, `    ${t.part}`);
    if (t.copyright) out.push(`    ${t.copyright}`);
    out.push(`    Source code: ${t.source}`);
    out.push(THIN, t.text ?? `The text of ${t.common} follows below.`);
  }
  out.push("", "", RULE, "Linux AppImage: license texts referred to above", RULE);
  for (const c of linux.common) {
    out.push("", THIN, `${c.id}: ${c.names.join(", ")}`, THIN, c.text);
  }
  return out.join("\n") + "\n";
}

// ---------------------------------------------------------------------------------------------

const check = process.argv.includes("--check");
const pdf = pdfium();
const linux = appimage();
const components = [rustStd(), ...rustCrates(), ...(await npmPackages())];
const text = render(components, pdf, linux);

if (check) {
  const current = existsSync(OUTPUT) ? readFileSync(OUTPUT, "utf8").replace(/\r\n/g, "\n") : "";
  if (current !== text) {
    console.error(
      "THIRD_PARTY_LICENSES is out of date: a dependency changed. Run\n" +
        "  node scripts/third-party-licenses.mjs\n" +
        "and commit the result.",
    );
    process.exit(1);
  }
  console.log(`THIRD_PARTY_LICENSES is up to date (${components.length} components and PDFium)`);
} else {
  writeFileSync(OUTPUT, text);
  console.log(`wrote THIRD_PARTY_LICENSES (${components.length} components and PDFium)`);
}
