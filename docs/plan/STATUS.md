# Status

Single place for progress. Agents update this file at the end of every task.

## Current

- Milestone: **M0 Foundations** ([plan](M0.md))
- Next task: **T0.9 Performance harness** (needs free disk space and measurements on the owner's Mac)
- Release target: 0.1 after M3

## Tasks M0

| Task | State | Notes |
|---|---|---|
| T0.1 Cargo workspace skeleton | done | Dependency direction enforced by `crates/dimo-cli/tests/dependency_direction.rs` |
| T0.2 Tauri 2 shell with Svelte 5 | review | Owner: confirm the decisions marked "owner" in the log below |
| T0.3 Typed IPC with tauri-specta | done | Bindings test in `apps/desktop/src-tauri/tests/bindings.rs` |
| T0.4 Checks and CI | review | Owner: push, confirm CI green on 3 platforms, enable branch protection |
| T0.5 PDFium integration | done | Snapshot verified on mac arm64 only; `fetch-pdfium.ps1` untested |
| T0.6 Text runs with geometry | done | 24 of 24 truth callouts matched by runs; thresholds need calibration on more drawings (Q-10) |
| T0.7 Tile protocol and cache | done | Windows webview not verified (CI build only) |
| T0.8 Viewport with SVG overlay | review | Owner: confirm 500 balloons at 60 fps on own Mac (agent measured mean 16.67 ms, p95 18 ms on M2 16 GB, dev build), tune wheel zoom speed |
| T0.9 Performance harness | todo | |
| T0.10 Corpus truth format | review | Owner: review `corpus/truth/test_drawing_1.truth.json`, see log 2026-10-09 |
| T0.11 Synthetic generator (stretch) | done | `cargo run -p dimo-synth -- --seed N --count 6`. Owner: check fit table `FITS` in `tools/synth/src/lib.rs` (D-43) |

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

- 2026-10-09: T0.8 merged, in review. Pitfall: tests use `env!("CARGO_MANIFEST_DIR")`, so a target dir shared with deleted worktrees can run stale test binaries with dead paths; touch the test files or rebuild when that happens. Disk below 1 GB free, T0.9 waits for space.
- 2026-10-09: T0.7 done and merged. Wave 2 complete. Disk nearly full (about 2 GB free); merged worktrees removed.
- 2026-10-09: T0.6 and T0.11 done and merged. PDFium treats a `/ToUnicode` name (Identity-H) as code = Unicode; correct for test_drawing_1 only because CIDs equal code points (details in corpus notes).
- 2026-10-09: T0.5 done and merged. Wave 1 complete. Disk ran full during parallel builds; wave 2 agents share the main `target/`.
- 2026-10-09: T0.3 done, T0.4 and T0.10 in review (merged into main). Truth file has 24 characteristics; owner to check: ISO 286 limits of c01, c04, c06 (`Ø8 c10`, `h6`, `d9`) taken from memory, the plain `Ø8 f7` (c02, c12) using 7.987/7.972 vs the printed toleranced variant, `R15 H7` (c05) applied literally, whether the burr note (c24) belongs in the truth. New finding: the font `/ToUnicode` is the name `/Identity-H`, not a CMap stream (in corpus notes).
- 2026-10-08: T0.2 in review. Tauri shell with Svelte 5, Tailwind 4 tokens, Bits UI, Paraglide (en, de), view store. `tauri dev` opened the native window (checked by window capture). View and language switch tested in the browser pane against the same dev server; clicking inside the native window was not possible with the available tools.
- 2026-10-08: T0.1 done. Workspace with 8 library stubs and `dimo-cli` (clap 4.6.7). `cargo deny check` passes (wildcard paths allowed for unpublished internal crates); all locked crates are at least 14 days old.
- 2026-10-08: Repository bootstrapped: agent instructions, conventions, M0 plan, governance files, guard hooks. No code yet.
