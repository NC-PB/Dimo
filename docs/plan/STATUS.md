# Status

Single place for progress. Agents update this file at the end of every task.

## Current

- Milestone: **M0 Foundations** ([plan](M0.md)) finishing, **M1 Manual ballooning** ([plan](M1.md)) implemented, waiting for owner review and exit criterion
- Next: **owner review of M0 and M1** (manual test, exit criterion, open "Owner: confirm" decisions), then plan M2
- Release target: 0.1 after M3

## Tasks M0

| Task | State | Notes |
|---|---|---|
| T0.1 Cargo workspace skeleton | done | Dependency direction enforced by `crates/dimo-cli/tests/dependency_direction.rs` |
| T0.2 Tauri 2 shell with Svelte 5 | review | Owner: confirm the decisions marked "owner" in the log below |
| T0.3 Typed IPC with tauri-specta | done | Bindings test in `apps/desktop/src-tauri/tests/bindings.rs` |
| T0.4 Checks and CI | done | CI green on macOS, Windows, Ubuntu (run 37987569387). Owner: enable branch protection |
| T0.5 PDFium integration | done | Snapshot verified on mac arm64 only; `fetch-pdfium.ps1` untested |
| T0.6 Text runs with geometry | done | 24 of 24 truth callouts matched by runs; thresholds need calibration on more drawings (Q-10) |
| T0.7 Tile protocol and cache | done | Windows webview not verified (CI build only) |
| T0.8 Viewport with SVG overlay | review | Owner: confirm 500 balloons at 60 fps on own Mac (agent measured mean 16.67 ms, p95 18 ms on M2 16 GB, dev build), tune wheel zoom speed |
| T0.9 Performance harness | done | M2 16 GB: first sheet 0.82 s, 500 balloons 60 fps (p95 18 ms), memory 222 MiB Rust plus about 0.5 GiB WebKit. Windows laptop to measure. Fast pan at 100 percent has some late frames, see T1.11 |
| T0.10 Corpus truth format | review | Owner: review `corpus/truth/test_drawing_1.truth.json`, see log 2026-10-09 |
| T0.11 Synthetic generator (stretch) | done | `cargo run -p dimo-synth -- --seed N --count 6`. Owner: check fit table `FITS` in `tools/synth/src/lib.rs` (D-43) |

## Tasks M1

| Task | State | Notes |
|---|---|---|
| T1.0 M0 follow-ups | review | Owner: confirm pinch and wheel zoom feel in `pnpm dev` |
| T1.1 Domain model and command engine | done | Owner: confirm "unlock renumbers" (D-23 is silent) |
| T1.2 Ballooned PDF writer | done | Owner: confirm Open Sans Bold and the flag shape |
| T1.3 Project file, autosave and recovery | done | Owner: confirm uncompressed container |
| T1.4 CSV and XLSX characteristic list | done | Owner: confirm columns, German headers, umlauts in CSV, whether non inspected rows are exported |
| T1.5 Project session and IPC | done | Owner: check the unsaved changes prompt on window close and quit in the native window |
| T1.6 Balloon placement and editing | done | Owner: shortcuts V, B, S, Shift+arrows; value field fills requirement text only |
| T1.7 Characteristic table | done | 1000 rows: mean 17.2 ms, p95 21 ms. Review fixes merged (T1.7a). Owner: German kind names (Flachsenkung, Kegelsenkung, Fahnenhinweis) |
| T1.8 Sheet rotation, scale and units | done | Owner: glance at German strings; shortcuts R and Shift+R |
| T1.9 Export and settings views | done | Owner: rejected balloons left out of PDF, export options per user, issued lock kept when the write fails, shortcuts Cmd+1/Cmd+E/Cmd+, |
| T1.10 User guide and exit check | review | Smoke test passes (`scripts/e2e-smoke.sh`). Owner: run `docs/dev/manual-test-m1.md` and the exit criterion `docs/perf/M1-exit.md` |
| T1.11 Page cache on the render thread | done | Fast pan at 100 percent: max frame 32 ms to 18 ms. Raster sheets to measure in M4 |

States: `todo`, `in progress`, `review` (waits for owner), `done`, `blocked` (reason in notes).

## Waiting for the owner

| Item | Since |
|---|---|
| Tolerance tables to verify against Tabellenbuch (D-43) | from M2 |
| More corpus drawings, especially scans and outlined text (Q-10) | before M4 |
| Trademark search Swissreg / EUIPO (Q-01) | before 0.1 |

## Decision log

Decisions made during implementation that are not in the spec. Newest first.
Format: date, decision, reason, who. Decisions that change architecture become ADRs.

| Date | Decision | Reason | By |
|---|---|---|---|
| 2026-10-09 | Windows test executables get the Common Controls v6 manifest through `rustc-link-arg-tests` in src-tauri `build.rs` (`windows-test-manifest.xml`) | tauri-build embeds it only into the app executable | coordinator |
| 2026-10-09 | End to end: `scripts/e2e-smoke.sh` drives the real debug app through the dev UI script (main flow, reopen, crash recovery) and checks the written files; replaces WebdriverIO on macOS (WKWebView has no WebDriver). New dev hooks `DIMO_DEV_SAVE_AS`, `DIMO_DEV_HOME`; `bindings.ts` is written only when its content changes | Test the real app where tauri-driver cannot | T1.10, agent |
| 2026-10-09 | Exports run as background jobs (`export_project` returns a job id, `JobProgress` then `JobFinished`), written to a temp file and renamed; "export as issued" locks numbering as an undoable command in the same step as the snapshot; annotation dates from `ProjectSession::modified()`; rejected characteristics left out of PDF, CSV and XLSX; export options remembered per user | FR-EXP-01, FR-EXP-09, FR-EXP-11, D-23, D-33. **Owner: confirm** | T1.9, agent |
| 2026-10-09 | Balloon geometry single source: `dimo_core::BalloonStyle::layout`, used by the PDF export; the viewport reads generated `BALLOON_METRICS` and is checked against a Rust written fixture. D-24 "scaled with sheet size" read as 7 mm in sheet space; text width estimate 0.6 em per digit (tested never narrower than the real font) | No duplicated rule between export and viewport | T1.9, agent |
| 2026-10-09 | App settings (theme, UI language, audit user, export options) stored by Rust in `settings.json` in the app config dir, loaded before first render; language picker moved to Settings; Review and Measure views show a "later version" placeholder | D-27, D-50, D-51, NFR-SEC-01 | T1.9, agent |
| 2026-10-09 | Balloon tools: Select (V), Place (B), style popover (S), Shift+arrows nudge 1 mm; click placement stores no source region, drag placement stores an oriented box; balloon placed one balloon height up and right on screen; value field fills `requirement_text` only; balloon size and font rule in `viewport/balloons.ts` (to be moved to one place with the export, T1.9) | FR-BAL-01, FR-BAL-02, FR-BAL-12, FR-CHR-09. **Owner: confirm shortcuts** | T1.6, agent |
| 2026-10-09 | Selection store from T1.6 (`focus(id, from)`); the view named in `from` takes keyboard focus, others only scroll; a viewport focus request only activates the table cell. While the table has focus only Cmd/Ctrl shortcuts and `?` reach the window | Avoid focus theft and key clashes between table and drawing | merge T1.6/T1.7 |
| 2026-10-09 | Rust refusals carry `RejectReason` (12 kinds), translated in the frontend (`refusal_*`); argument parse errors from Tauri are translated by the kind of value typed; unknown errors show Rust's text | FR-SET-04, every visible string through Paraglide | T1.7a, agent |
| 2026-10-09 | Characteristic table: ARIA grid in a resizable bottom panel, fixed 28 px rows, TanStack Table 9.2.4 plus virtual-core 3.17.11; edits send typed text as `update_fields`, Rust validates decimals and its error is shown; edits apply to the active row only | NFR-UX, rule 2 | T1.7, agent |
| 2026-10-09 | Shared selection store `stores/selection.svelte.ts` (`ids`, `select(ids, mode)`, `clear`, `has`, `focus`/`focusRequest`) used by table and viewport | One selection for table and drawing | coordinator |
| 2026-10-09 | Sheet rotation rotates the view transform (one matrix for tiles and overlay), not the rendered tiles; tile cache stays valid, Rust unchanged. Rotating keeps zoom and center, fit on first show; scale as exact ratio with presets and custom; shortcuts R and Shift+R | Rule 4: stored geometry stays in sheet space | T1.8, agent |
| 2026-10-09 | IPC: `project_state`, `new_project`, `open_project`, `save_project`, `save_project_as`, `confirm_close` (each with `discard` where data could be lost), `execute(Command)`, `undo`, `redo`; events `project-loaded`, `project-patched` (with revision), `project-status-changed`, `close-requested`. Rust refuses to drop unsaved changes unless `discard` is true | Rust owns the document (ADR 0001); one patch path into the frontend | T1.5, agent |
| 2026-10-09 | Frontend `projectStore`: `$state.raw` project replaced per patch, unchanged items keep identity, revision gap triggers a full reload; selection lives in separate view stores | One place applies patches (frontend.md) | T1.5, agent |
| 2026-10-09 | Unsaved projects autosave to `<app data>/autosave/<uuid>.dimo` plus journal, restored at startup with a notice; instance lock is an OS file lock on `<project>.lock` (released by the OS on crash, stale lock files taken over) | NFR-REL-01 for never saved projects; no double editing. **Owner: confirm** | T1.5, agent |
| 2026-10-09 | Shortcuts Cmd+N, Cmd+S, Shift+Cmd+S, Cmd+Z, Shift+Cmd+Z, Ctrl+Y | D-52. **Owner: confirm** | T1.5, agent |
| 2026-10-09 | `vendor_library_path` searches upwards from the run time `CARGO_MANIFEST_DIR` before the compile time path | Shared build output across worktrees produced stale PDFium paths | coordinator |
| 2026-10-09 | `.dimo` container entries written uncompressed (manifest, project, audit, drawings in that order); ZIP time and manifest created/modified derived from the project, not the clock; deflated files from other tools still load; unknown entries ignored; read limits 1 GiB per drawing, 4 GiB total, 10,000 entries | Byte identical saves independent of compression library (FR-EXP-11 spirit), drawings are already compressed. **Owner: confirm** | T1.3, agent |
| 2026-10-09 | Journal `<file>.journal` next to the project, one line per audit entry with its changes, replayed with `apply_change`; a journal that does not match the project moves to `.journal.stale` | NFR-REL-01, D-28 | T1.3, agent |
| 2026-10-09 | One `zip` 8.6.0 for project files and XLSX (rust_xlsxwriter accepts ^8.3) | Merge of T1.3 and T1.4 | coordinator |
| 2026-10-09 | Render thread keeps the 4 most recently used loaded pages per document (about 10.5 MiB per dense A0 page), shared by tiles, text runs and sheet analysis; pages closed before their document, failed loads not cached | Page load (52 ms) dominated tile time; no `unsafe` needed because pdfium-render 0.9.4 pages do not borrow the document. Cap by bytes or 1 page for raster sheets if M4 measurements demand | T1.11, agent |
| 2026-10-09 | Characteristic list export: 20 columns (No to Comment, see `docs/user/en/exports.md`), rows in number order, rejected characteristics skipped, non inspected kept with `Inspect = no`; enum values stable English identifiers, only headers and sheet name localized; CSV UTF-8 without BOM, comma, LF, stored decimal digits; XLSX numbers up to 15 significant digits as number cells with matching format, longer as text; fixed doc properties and ZIP times | FR-EXP-09, FR-EXP-11, D-32. **Owner: confirm** | T1.4, agent |
| 2026-10-09 | CSV cells starting with `=`, `+`, `-`, `@` are not escaped (negative deviations start with `-`); user docs warn | Escaping would corrupt numeric values for CMM import | T1.4, agent |
| 2026-10-09 | Ballooned PDF written through pdfium-render page objects (full rewrite, not incremental update), not lopdf (about 25 extra crates). Trailer `/ID` parts that PDFium fills from clock and random are replaced by SHA-256 based values of equal length; annotation dates come from the caller | FR-EXP-01, FR-EXP-11 deterministic output | T1.2, agent |
| 2026-10-09 | Balloon number font: Open Sans Bold 1.10 (Apache 2.0) in `data/fonts/`, subset at runtime; balloon text limited to printable ASCII except quote characters; flag shape is a box with a pointed left end | D-24 "bold sans". **Owner: confirm font and flag shape** | T1.2, agent |
| 2026-10-09 | Known limits of the PDF writer: PDFium does not wrap original content in q/Q (an unclosed clip in the original could clip balloons) and drops unreferenced objects | Accepted for M1, revisit if a corpus drawing shows it | T1.2, agent |
| 2026-10-09 | Numbering: unlocked always 1..n in placement order, unlocking renumbers immediately; locked adds get highest number ever used plus one, moves refused while locked; `number` is `u32` (sub-numbers need a migration in M2) | D-21, D-23. **Owner: confirm unlock behavior** | T1.1, agent |
| 2026-10-09 | Limits derived from nominal plus deviations unless set directly; missing deviation clears derived limits; nominal implies a unit (degrees for angles, else sheet unit); hand placed characteristics start as kind `other`; new kinds counterbore, countersink, depth, material_process, other | FR-CHR-01, FR-CHR-02 | T1.1, agent |
| 2026-10-09 | `Environment` trait provides UUIDs, clock (RFC 3339 `Timestamp`) and user name; tests use `FixedEnvironment`. `specta::Type` behind a `dimo-core` feature `specta` enabled only by src-tauri. Undo applies inverted changes, redo re-applies stored changes so IDs stay stable | Determinism (rule 11), core free of IPC concerns | T1.1, agent |
| 2026-10-09 | Performance harness is a `dimo-pdf` example run by `scripts/perf.sh`, stress PDF from `dimo-synth --stress --sheets 50` written to `target/perf/`, never committed. `sheet_sizes` reads sizes without loading page content (open 1971 ms to 199 ms) | No criterion dependency needed; NFR-PERF-01 was missed before the fix | T0.9, agent |
| 2026-10-09 | `render_region` pads left and top itself (shared with tiles), skipped only if the padded image would exceed the max render side. Wheel gain 0.002 per px, pinch gain 0.01 per px clamped at 40 px per event, constants in `ZOOM_TUNING`; Safari gesture events supported | PDFium glyph edge quirk; separate tuning for mouse and trackpad | T1.0, agent |
| 2026-10-09 | File dialog via `rfd` called from Rust; temporary `open_document(path)` removed; only `allow-open-document-dialog` granted | No dialog or fs plugin in the webview (NFR-SEC-01) | T0.8, agent |
| 2026-10-09 | Viewport: one view transform `screen = sheet * scale + t` snapped to device pixels; tile level is the coarsest one not upscaled; max zoom 16 px per sheet unit; backdrop layer plus one tile prefetch margin; one device pixel tile overlap against seams | Crisp at all zoom levels, single transform per layer | T0.8, agent |
| 2026-10-09 | Dev only hooks in debug builds: `DIMO_DEV_OPEN`, `DIMO_DEV_SHEET`, `DIMO_DEV_BALLOONS`, `DIMO_DEV_ANCHORS`, `DIMO_DEV_VIEW`, `DIMO_DEV_PAN_CHECK`; `dev_report_frame_times` | Lets agents verify the native window without clicking; no effect in release builds | T0.8, agent |
| 2026-10-09 | Shortcuts: Cmd/Ctrl+O, +/=, -, 0, arrow keys, PgUp/PgDn, ? (D-52). Dummy balloons use a fixed 7 mm size; D-24 sizing belongs to the real balloon style in Rust | Defaults, owner may change | T0.8, agent |
| 2026-10-09 | Tiles: `dimo://localhost/tile/{content hash}/{sheet}/{zoom}/{x}/{y}` (`http://dimo.localhost/...` on Windows), 512 px, zoom -4 to 5 as 2^zoom px per sheet unit; logic in `dimo_pdf::tiles::TileService`, src-tauri only wires the async scheme | Rust owns logic (ADR 0001), never block main thread | T0.7, agent |
| 2026-10-09 | Tile encoding PNG `Compression::Fast`, greyscale when possible (0.7 to 0.9 ms, 5 to 7 KB per tile); memory LRU 128 MiB, disk cache 2 GiB in the app cache dir keyed by render version and content hash; `Cache-Control: immutable` | Fast and small; bump `RENDER_VERSION` in `tiles/disk.rs` when PDFium or encoder settings change | T0.7, agent |
| 2026-10-09 | Tile cancellation by interest: `set_tile_interest(doc, ranges)`, queued tiles outside get HTTP 204 | Simple, no per request ids | T0.7, agent |
| 2026-10-09 | Tiles render with a 32 unit left and top margin, then crop | PDFium draws anti aliased glyphs wrong where they cross the left or top bitmap edge. `Document::render_region` has the same issue, fix before OCR crops (M5) | T0.7, agent |
| 2026-10-09 | `open_document(path)` command is temporary; T0.8 replaces it with a Rust file dialog command | NFR-SEC-01: the webview must not pass arbitrary paths | T0.7, agent |
| 2026-10-09 | Sheet kind from PDFium page objects (text objects, fonts, paths, images, form XObjects) plus share of unmapped characters: `vector_text` needs at least 10 readable chars with at most half unmapped, `raster` if images cover at least 50 percent, else `vector_outlined`. No lopdf | PDFium already exposes what is needed; lopdf adds about 19 crates and a second parser | T0.6, agent |
| 2026-10-09 | Text runs: same font, size within 5 percent, rotation within 1 degree, baseline within 0.2 em, gap -0.3 to 0.8 em; space inserted from 0.2 em; invisible and generated chars skipped; subset tags stripped from font names | Matches all callouts of test_drawing_1; to be calibrated (Q-10) | T0.6, agent |
| 2026-10-09 | Synthetic generator writes the PDF by hand (no lopdf yet), Helvetica with WinAnsi plus `/Differences` for minus, Ø, ± and a ToUnicode CMap; deterministic (no Info, ID or dates); regions from Helvetica AFM widths | lopdf 0.45 too new, 0.44 not needed for a first cut; keeps text extractable | T0.11, agent |
| 2026-10-09 | PDFium pinned to chromium/7881 (matches pdfium-render 0.9.4 feature `pdfium_7881`); all PDFium work on one dedicated render thread; lookup order: explicit path, `DIMO_PDFIUM_PATH`, `vendor/pdfium/<target>/`, never a system library; renders capped at 16384 px per side | pdfium-render binds PDFium once per process and documents borrow it, a single owner thread avoids `unsafe` | T0.5, agent |
| 2026-10-09 | PDFium tests skip with a message when the library is missing, fail with `CI=true` or `DIMO_REQUIRE_PDFIUM=1`. Tile snapshot fingerprint: 396 x 306 render, 12 px blocks, 4 grey levels, hashed | Contributors without PDFium can still run checks; fingerprint robust to tiny rasterizer differences | T0.5, agent |
| 2026-10-09 | Installers must ship the `licenses/` folder from the PDFium archive | BSD-3-Clause attribution | T0.5, agent, for release 0.1 |
| 2026-10-09 | Truth types in `dimo-core::truth` (with shared `SheetKind`, `CharacteristicKind`, `Unit`, `ToleranceRule`, `OrientedBox`, strict decimal strings). Sheet indexes zero based, characteristic ids file local (`c01`), new rule values `no_tolerance_defined` and `drawing_rule`, callout parts joined by one space | Reusable by the M1 domain model; core stays free of IO | T0.10, agent |
| 2026-10-09 | `zerocopy` pinned to 0.8.57 | 0.8.62 was published the same day (rule 10) | T0.10, agent |
| 2026-10-09 | specta =2.0.0-rc.25, specta-typescript =0.0.12, tauri-specta =2.0.0-rc.25 (release candidates, no stable exists) | Required for generated IPC types (rule 6). **Owner: confirm** | T0.3, agent |
| 2026-10-09 | `deny.toml` ignores RUSTSEC-2024-0436 (`paste` unmaintained), build time macro required by specta | No vulnerability, no alternative. **Owner: confirm** | T0.3, agent |
| 2026-10-09 | `bindings.ts` written on debug start and by `cargo test -p dimo-desktop --test bindings` (rewrites and fails on mismatch); excluded from eslint and prettier, still type checked | Keeps the file byte identical to the generator | T0.3, agent |
| 2026-10-09 | CI: actions pinned by commit SHA, toolchain via `rustup`, cargo-deny via prebuilt binary, npm license check and DCO check as own scripts | Fewer third party actions, same checks locally and in CI | T0.4, agent |
| 2026-10-09 | Wave based parallel work: agents in git worktrees, coordinator merges and owns STATUS.md and M0.md | Speed; worktrees each need 2 to 4 GB of build output, delete `target/` of merged worktrees | owner request |
| 2026-10-08 | TypeScript 6.0.3 instead of 7.x | svelte-check 4.7 and typescript-eslint 8.70 support TypeScript only up to 6.0 | T0.2, agent |
| 2026-10-08 | pnpm 11.27.1 instead of 12.x | corepack 0.34 (bundled with Node 22.20) cannot start pnpm 12 (`bin/pnpm.mjs`) | T0.2, agent |
| 2026-10-08 | Tauri app identifier `io.github.nc-pb.dimo` | Determines the app data folder, hard to change after release. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | Theme follows the OS only. The settings switch of D-51 comes with the Settings view | Settings view has no content yet. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | UI language: stored in localStorage, else OS language, else English. Switching reloads the window | Paraglide default, every message rerenders without per message reactivity | T0.2, agent |
| 2026-10-08 | Paraglide message format plugin loaded from `node_modules`, not from a CDN | Offline, reproducible builds (rule 9). Package has no license field, its LICENSE file is MIT | T0.2, agent |
| 2026-10-08 | Accepted build time npm licenses outside rule 10 list: MPL-2.0 (`lightningcss`, used by Tailwind and Vite), unlabeled `@lix-js/sdk-<platform>` binaries (repository `opral/lix` is MIT, used by Paraglide at build time), BlueOak-1.0.0 (`minimatch`), 0BSD (`tslib`). None of the MPL or unlabeled code ships in the app bundle | Needed for the T0.4 npm license check. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | `deny.toml` ignores six "unmaintained" advisories (proc-macro-error, unic-*) that come through Tauri 2.11, no upgrade path | No vulnerability, only maintenance status. Remove when Tauri drops them | T0.2, agent |
| 2026-10-08 | Cargo deps checked for the 14 day rule with `scripts/check-lock-age.sh`; tauri sibling crates pinned to the 2.11.6 release set | Cargo has no stable min-publish-age yet; tauri 2.11.6 breaks with tauri-runtime 2.12 | T0.2, agent |
| 2026-10-08 | Internal dependency allow list: notation, pdf, vision, io, report may use core; tolerance may use core and notation; detect as in rust.md. Widen only with a log entry | rust.md leaves these edges open; core is a pure leaf, so no cycles | T0.1, agent |
| 2026-10-08 | `unwrap`/`expect`/`print_stdout`/`print_stderr` warn workspace wide, allowed in tests via `clippy.toml`; `PDFium` is a valid doc identifier | Enforces rust.md rules mechanically | T0.1, agent |
| 2026-10-08 | Code repository at `~/Repositories/dimo` (public `github.com/NC-PB/dimo`). Spec, ADRs and corpus were copied from the planning folder `project_moonshot`; from now on this repository is the source of truth | One self contained repo for agents and contributors | owner, setup |
| 2026-10-08 | pnpm (via corepack) as package manager for the frontend | Fast, strict, standard with Tauri; spec left it open | setup, owner may change |
| 2026-10-08 | `scripts/check.sh` as the single local and CI entry point | Same checks everywhere, no extra task runner dependency | setup |

## Log

Short entries, newest first: date, task, what changed, anything the next session must know.

- 2026-10-09: T1.0 and T1.1 merged; wave 2 (T1.3, T1.4) started.
- 2026-10-09: T0.9 done, results in `docs/perf/M0.md`. All M0 tasks implemented; M0 closes when the owner finishes the open reviews.
- 2026-10-09: First CI runs on GitHub: macOS green, Windows (unused import in a unix only test) and Ubuntu (text run split with a substitute font) failed; both fixed in a120848. Windows test executables then failed to start (STATUS_ENTRYPOINT_NOT_FOUND, Common Controls v6 manifest missing); fixed in abe16d5 by embedding the manifest for test targets. First fully green run: 37987569387 on 8a29001. One unexplained local failure of the three `dimo-io` export snapshot tests during a full check, not reproducible in 8 later runs; watch in CI.
- 2026-10-09: M1 implemented: T1.9 and T1.10 merged, smoke test passes on main (coordinator run). User guide in `docs/user/{en,de}/README.md`.
- 2026-10-09: Wave 4 merged (T1.6, T1.7, T1.7a, T1.8): balloons can be placed, edited and listed in the app. Disk ran full twice; the shared `target/` reaches about 30 GB, clear `target/debug/incremental` after each wave.
- 2026-10-09: M1 plan drafted (`docs/plan/M1.md`), open M0 points carried over.
- 2026-10-09: T0.8 merged, in review. Pitfall: tests use `env!("CARGO_MANIFEST_DIR")`, so a target dir shared with deleted worktrees can run stale test binaries with dead paths; touch the test files or rebuild when that happens. Disk below 1 GB free, T0.9 waits for space.
- 2026-10-09: T0.7 done and merged. Wave 2 complete. Disk nearly full (about 2 GB free); merged worktrees removed.
- 2026-10-09: T0.6 and T0.11 done and merged. PDFium treats a `/ToUnicode` name (Identity-H) as code = Unicode; correct for test_drawing_1 only because CIDs equal code points (details in corpus notes).
- 2026-10-09: T0.5 done and merged. Wave 1 complete. Disk ran full during parallel builds; wave 2 agents share the main `target/`.
- 2026-10-09: T0.3 done, T0.4 and T0.10 in review (merged into main). Truth file has 24 characteristics; owner to check: ISO 286 limits of c01, c04, c06 (`Ø8 c10`, `h6`, `d9`) taken from memory, the plain `Ø8 f7` (c02, c12) using 7.987/7.972 vs the printed toleranced variant, `R15 H7` (c05) applied literally, whether the burr note (c24) belongs in the truth. New finding: the font `/ToUnicode` is the name `/Identity-H`, not a CMap stream (in corpus notes).
- 2026-10-08: T0.2 in review. Tauri shell with Svelte 5, Tailwind 4 tokens, Bits UI, Paraglide (en, de), view store. `tauri dev` opened the native window (checked by window capture). View and language switch tested in the browser pane against the same dev server; clicking inside the native window was not possible with the available tools.
- 2026-10-08: T0.1 done. Workspace with 8 library stubs and `dimo-cli` (clap 4.6.7). `cargo deny check` passes (wildcard paths allowed for unpublished internal crates); all locked crates are at least 14 days old.
- 2026-10-08: Repository bootstrapped: agent instructions, conventions, M0 plan, governance files, guard hooks. No code yet.
