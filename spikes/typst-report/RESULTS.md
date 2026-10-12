# T3.2 spike results: Typst as a library for PDF reports

Measured on 2026-10-12 on the owner's Mac (Apple silicon, 8 cores, macOS 27, Rust 1.99.0).
Spike code: `src/sandbox.rs` (World and render function), `src/main.rs` (driver),
`report.typ` (table with repeated header from `/data.json`). Run it with
`cargo run --release -- render <rows> <samples> <out.pdf>` and `cargo run --release -- sandbox`.

**Verdict: stop at the gate.** Determinism, sandbox, licenses and render time are fine. The
binary size growth of the desktop app (+34 MB stripped, +45 MB unstripped, more than three times
the current app) is far above the limit of about 10 MB set for this spike, and `cargo deny check`
fails on six advisories that have no fix upstream. Nothing was added to `crates/dimo-report` or to
the workspace `Cargo.lock`. The decision goes to the owner (see "Options" below).

## Version

| Crate | Version | Published | License |
|---|---|---|---|
| `typst` | 0.15.1 (latest) | 2026-07-17 | Apache-2.0 |
| `typst-layout` | 0.15.1 | 2026-07-17 | Apache-2.0 |
| `typst-pdf` | 0.15.1 | 2026-07-17 | Apache-2.0 |

`typst-layout` is needed because `PagedDocument` is not re-exported by `typst` 0.15. None of the
three crates has cargo features, so nothing can be switched off (bibliography, syntax highlighting,
hyphenation for all languages, WebAssembly plugins, PDF and SVG image decoding are always
compiled in). `typst-assets` has an optional `fonts` feature that is off, so no fallback fonts are
embedded.

## Numbers

| Measurement | Result |
|---|---|
| Crates added to the workspace `Cargo.lock` | **168** (514 to 682 packages), no existing version changed |
| Licenses | all allowed by `deny.toml` (`licenses ok`, `bans ok`, `sources ok`) |
| `cargo deny check` (advisories) | **fails**: 2 vulnerabilities and 4 unmaintained notices, no fix available (below) |
| 14 day rule | 4 crates too new with the default resolution (`fearless_simd` 1.1.0, `hayro-ccitt` 0.4.0, `hayro-jbig2` 0.3.1, `thin-vec` 0.2.21); fixed in the spike lock with `cargo update -p hayro-jbig2 --precise 0.3.0` (drops `fearless_simd` 1.x and `hayro-ccitt` 0.4) and `cargo update -p thin-vec --precise 0.2.19`. All others at least 14 days old (cutoff 2026-09-28) |
| Desktop release binary, stripped | 15.7 MB to 50.0 MB, **+34.3 MB** |
| Desktop release binary, unstripped | 21.9 MB to 66.8 MB, **+44.8 MB** |
| Desktop binary compressed (as in an installer), xz / gzip | 3.4 to 12.6 MB (+9.2 MB) / 5.4 to 18.9 MB (+13.5 MB) |
| Clean release build of `dimo-desktop` | 174 s to 340 s, **+166 s** (+95 percent) |
| Clean debug build of the spike alone | 66 s (check.sh and tests build debug) |
| Render, 3 pages (90 rows, 6 samples), release | 38 to 67 ms cold, 2 ms warm |
| Render, 10 pages (300 rows, 10 samples), release | 164 to 194 ms cold, 11 to 13 ms warm |
| Render, 3 pages, debug build | 620 to 770 ms cold |
| PDF size | 214 KB for 3 pages, 900 KB for 10 pages (tagged PDF, subset fonts) |
| Determinism | byte identical: two cold renders in one process, a warm render, two separate processes, debug and release build (same SHA-256 every time) |
| Sandbox | every escape attempt fails with a clear error (below) |

How the binary size was measured: two clean release builds of `dimo-desktop` in separate target
directories, the second with `typst`, `typst-layout` and `typst-pdf` added to `src-tauri` and
`src/sandbox.rs` included into `main.rs` with `#[path]`, called behind an environment variable so
the linker keeps it. The temporary change was reverted and never committed. The current release
profile has no LTO and no `opt-level = "s"`; tuning the profile would shrink both binaries but not
remove the embedded data (ICU segmenter and locale data, hyphenation patterns, syntax definitions,
citation styles), which makes up a large part of the growth.

Determinism setup: no clock (`World::today` returns `None`, `set document(date: none)`),
`PdfOptions::ident` set to the SHA-256 of the data JSON, `creator` fixed, `timestamp: None`.
`typst::comemo::evict(0)` between runs so the two "cold" renders share no cache.

## Sandbox

The World serves exactly two in-memory files (`/main.typ` and `/data.json`) and the two bundled
Open Sans fonts. It has no file system access, no package resolver, no network code (no HTTP
crate in the tree) and no clock. Results of `cargo run --release -- sandbox`:

| Attempt | Result |
|---|---|
| `read("/etc/passwd")` | error: file /etc/passwd is not part of the template |
| `read("../../Cargo.toml")` | error: path would escape the project root |
| `import "/other.typ"`, `include "secret.typ"` | error: file is not part of the template |
| `import "@preview/..."`, `import "@local/..."` | error: packages are not available in reports |
| `plugin("/evil.wasm")` | error: file is not part of the template (plugins can only load bytes from the World) |
| `image("/Users/x/a.png")` | error: file is not part of the template |
| `datetime.today()` | error: unable to get the current date |
| `json("/data.json")` | renders |

## Advisories that fail `cargo deny check`

All arrive through `typst-library` and have no upgrade path on our side.

| ID | Crate | Kind | Path |
|---|---|---|---|
| RUSTSEC-2026-0194 | quick-xml 0.38.4 | vulnerability: quadratic time on a start tag with many attributes | citationberg 0.7.0 (latest) > hayagriva > typst-library |
| RUSTSEC-2026-0195 | quick-xml 0.38.4 | vulnerability: unbounded namespace allocation in `NsReader` | same |
| RUSTSEC-2025-0141 | bincode 1.3.3 | unmaintained | syntect > two-face > typst-library |
| RUSTSEC-2024-0320 | yaml-rust 0.4.5 | unmaintained | syntect |
| RUSTSEC-2026-0206 | rustybuzz 0.20.1 | unmaintained | typst-layout, krilla |
| RUSTSEC-2026-0192 | ttf-parser 0.25.1 | unmaintained | typst-library, fontdb, krilla |

The two quick-xml issues are only reachable when a template parses CSL (bibliography) XML. Report
templates are embedded in the binary and the data JSON cannot make Typst parse XML with quick-xml
(Typst's `xml()` uses roxmltree), so they are not reachable from user data. They could be listed
as ignores in `deny.toml` with that reason, like the existing Tauri entries, but that is a
decision for the owner, not for this spike.

## Glyph coverage of Open Sans 1.10 (Regular and Bold)

Present: `Ø ± ° ′ ″ × −` (U+2212) and `ä ö ü Ä Ö Ü ß`. Missing: `⌀` (U+2300) and the symbols
`✓ ✗ ○ ●`. Without fallback fonts a missing glyph prints as an empty box, so templates must map
`⌀` to `Ø` (a show rule `#show "⌀": "Ø"` does it) and draw the pass, fail and not evaluated
marks (decision 9) as shapes, not as characters.

## Options for the owner

1. **Accept Typst** as D-30 and spec 06 say: +34 MB stripped (+9 to 14 MB in a compressed
   installer), +166 s clean release build, ignore the six advisories in `deny.toml` with reasons,
   pin the four downgrades. Everything else in T3.2 works as planned (sandbox, determinism, fast
   rendering, real text layout with tables, repeated headers and page x of y).
2. **pdfium-render page objects** (already a dependency, no new crates, no size growth): Dimo
   would write its own small layout code (text measuring with the bundled font, table rows, page
   breaks with repeated header, column blocks, page x of y). No template language: the layout of
   the default reports lives in Rust, and custom templates (M7) would need another answer (XLSX
   templates already planned). Deviates from D-30 and spec 06, so it needs a new ADR.
3. **Typst with a smaller release profile** (`lto = "fat"`, `codegen-units = 1`,
   `opt-level = "s"` for the whole app): reduces all code, not the embedded data. Not measured
   here; would need its own measurement and makes every release build slower.
