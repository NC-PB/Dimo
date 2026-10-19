# Status

Single place for progress. Agents update this file at the end of every task.
Session handover with context, pitfalls and owner items: [HANDOVER.md](HANDOVER.md).

## Current

- **Release 0.1 in progress** ([plan](R01.md)). Done: R.4 tables, R.10 diagnostics, R.11 version 0.1.0 and icon, R.13 release workflow (draft `v0.1.0-rc.2`), T3.22 title bar, T3.23 license list, R.12 agent part. R.9 update check, R.19 images open and store, R.20 image editor, R.21 ballooned PDF of images. Next agent tasks: R.22 agent part (smoke step, guide page for photos), R.8 after the demo drawing
- Product renamed to DimoMark (owner, 2026-10-16, trademark check by the owner pending); bundle id and paths stay `dimo`
- Owner, for 0.1: demo drawing (R.7), trademark check for DimoMark (R.5), the rc.2 installers on Windows 10 and 11 step by step (`docs/perf/R01-platforms.md`; rc.2 installer works on the owner's Windows PC, 2026-10-18), T3.21 shop floor user test and T3.11 exit run
- Roadmap after 0.1 merged to three milestones M4 to M6 ([ADR 0009](../adr/0009-roadmap-consolidation.md), accepted 2026-10-16); ADR 0010 accepted 2026-10-16
- M3 built incl. UI rework; M2 implemented, owner exit open; M0 and M1 implemented, owner reviews open

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

## Tasks M2

| Task | State | Notes |
|---|---|---|
| T2.1 Callout parser | done | `parse_callout`, `Callout`, canonical printer; 4 property tests at 10,000 cases. Owner: confirm inch heuristic and limit order (decision log 2026-10-10) |
| T2.2 Tolerance table format and ISO 2768-1 | review | Format, loader, schema, release gate (`scripts/check-table-status.sh`, CI job "tolerance tables (D-43)"). Owner: verify `iso-2768-1.toml` (D-43) |
| T2.3 ISO 286 fit tables | review | Cross-checked against ISO 286-1/-2:2010 (2026-10-10), about 2300 vectors; synth `FITS` and corpus truth fits match the table. Owner: verify `iso-286.toml` (D-43) |
| T2.4 Data model version 2 | done | Schema version 2 with migration; `DisplayNumber`, `ToleranceDerivation`, numbering and tolerance settings, zone grid, views, `Proposal`, `AcceptProposals`, history query. Sub-numbers as separate characteristics (owner confirmed) |
| T2.5 Tolerance engine | done | `interpret`, `ToleranceContext::for_project`, `explain` (en, de). All truth limits reproduced on draft tables. Owner: confirm rounding and fit rules (decision log 2026-10-10) |
| T2.6 Box select with PDF text | done | Box tool proposal card (Enter accepts, Esc discards), typed callouts through the same Rust path, real engine wired (`src-tauri/src/recognition.rs`). Known bug: keystrokes typed right after a placing click can be lost before the value editor has focus (smoke test flaked once), fix after T2.7 merges |
| T2.7 Numbering strategies | done | Strategies sheet/zone, view, clockwise per view, kind, manual; preview with ghost numbers; zone grid and view editors. Owner: confirm unlocked renumbering and sub-number grouping rules (decision log 2026-10-10) |
| T2.7a Follow-ups from wave 2 | done | Type-ahead kept after a placing click (editor and proposal card), zone grid defaults in `dimo-core::zones`, locked inserts after the selection. Insert after selection confirmed by the owner |
| T2.8 Tolerance UI, explanations and history | done | Rule column and markers, detail panel with explanation and history, tolerance settings and custom table import (crash safe through the journal), re-interpret, Rule column in CSV and XLSX |
| T2.9 Evaluation and exit criterion | review | `dimo-cli eval box-select`, CI gate on the strict row: 623/623 (100 %) on common callouts after T2.9a (draft tables). Owner: review truth file and tables, confirm on a real drawing |
| T2.9a Shorter leg input for angles | done | Leg field on the proposal card and in the detail panel (`set_shorter_leg`), stored as `Characteristic.shorter_leg` with its unit, kept by re-interpretation; schema stays 2. Strict evaluation 623/623. Spec review fixes merged |
| T2.10 User guide | done | User guide en/de checked against the code (mismatches fixed), `docs/dev/manual-test-m2.md`, smoke test section 5 (tolerance settings, box select, re-interpret). Owner: confirm hand derived values in the manual test (inch `.250` limits) |


## Tasks M3

| Task | State | Notes |
|---|---|---|
| T3.1 Data model version 3 | done | Schema 3 with migration from 2: samples, results, report info, issued reports, `result_type`; commands with inverses, result and serial parsing in Rust, load check of sample and result data. Spec reviewed |
| T3.2 Typst report rendering | done | `dimo_report::render_pdf` with a sandboxed Typst world (template files, `/data.json`, Open Sans Regular and Bold only), embedded template sets, placeholder `dimo-default`. Typst 0.15.1, 166 crates, 7 advisories ignored with reasons, lock age ok. Spike numbers in `spikes/typst-report/RESULTS.md` (accepted by the owner) |
| T3.3 Evaluation and verdicts | done | `dimo_core::evaluation` (verdicts, summaries, overall), result type pinning, query `measurement_state` (300 x 10 cells: 5.4 ms mean quiet, test asserts fastest of 30 under 20 ms). Two property tests at 10,000 cases |
| T3.4 Project and report details | done | Project and report panel in the Drawing side panel, company name in Settings; new projects start with company name and audit user as inspector (set by Rust at creation, no undo step). Owner: check German wording. Measure view placement moves to T3.5 |
| T3.5 Measurement grid | done | Measure view: sample bar, virtualized grid (rows from `measurement_state`), sample panel, project details, keyboard flow of decision 7, block paste as one undo step, shape marks, decision 16 marker (`changed_since_issued_report`). 300 x 10: commit to repaint 39.8 ms mean (dev build). Owner: press real Cmd+V once |
| T3.6 Report model and default templates | done | `ReportModel::build` (all values preformatted, codes for verdicts and rules), schema `docs/schema/report-data.schema.json`, `dimo-default` v1 (check sheet, inspection report, `labels.toml` en/de), PDF text snapshots in dimo-cli. Clean room audit passed. Owner: German labels (Prüfblatt, Prüfbericht, Änderungsstand, Geprüft von, Freigegeben von, i.O./n.i.O.) and empty "Approved by" date |
| T3.7 XLSX check sheet and inspection report | done | `dimo_report::render_xlsx` from the same `ReportModel`, labels from `labels.toml`, number cells with typed scale, marks in number formats plus fill, A4 landscape with repeated rows and columns, 6 sample columns per printed page, fixed properties and ZIP times. Owner: open the sample workbooks in a spreadsheet program (repair prompt, glyphs, print preview) |
| T3.8 Report export in the app | done | Report cards in the Export view (PDF or XLSX, en or de, blank columns), cancellable jobs (`cancel_job`, checked between steps), `report_export_warnings`, issued export records lock, filled report info and `IssuedReport` in one undo step after writing. 300 x 10 release: PDF 522 ms (1.69 MB), XLSX 12 ms |
| T3.9 Follow-ups from waves 2 and 3 | done | XLSX counts per sample, `SetResultType auto` with results refused, identical custom table import reported as unchanged, all reject reasons except `internal` translated, ghost number component test. Still English in the German UI: OS, PDFium, file, project and export error details, custom table parser messages, autosave error (needs structured error kinds, later). Smoke flake seen once in step 5 under load; 3 of 3 passes on a quiet machine. Delete confirmation for characteristics with results (component tests only, not run in the app) and German wording merged |
| T3.10 User guide and manual test | done | User guide en/de for every M3 feature, `docs/dev/manual-test-m3.md` (16 sections: real Cmd+V, four XLSX files in a spreadsheet program, German wording), smoke step 8 reads verdicts back from the exported PDF and XLSX. Owner: run the manual test |
| T3.11 Exit check | review | `docs/perf/M3-exit.md` written; agent dry run on synth seed 7 (25 inspected, 3 parts, invented values): verdicts and report correct, all re-exports byte identical, no bug. Owner: run the procedure on a real drawing and record the result |
| T3.12 ADR for the new shell | done | ADR 0007 accepted by the owner 2026-10-13; spec 12 D-50 points to it |
| T3.13 UI bug fixes | done | Place balloon toggle bound to the store (clicking the active button returns to Select); after a general tolerance change Dimo offers to update characteristics without tolerance or with the previous general tolerance (`general_tolerance_candidates`, `apply_general_tolerance`, one undo step, hand entered limits untouched). Notice wording confirmed by the owner 2026-10-13 |
| T3.14 Theme tokens and brand | done | cnc-master colors light/dark (contrast test, 45 checks, danger/success/warning kept apart from orange), Space Grotesk and JetBrains Mono bundled from the owner's own projects with OFL texts in `data/fonts/`, brand mark in the toolbar. Follow-ups: focus ring on sunken areas (2.87:1), values in mono, app icon from the brand mark |
| T3.15 App shell, tabs and ribbon | done | Title bar, tabs (File, Home, Drawing, Review, Measure, Report, View), one-row ribbon with More overflow, command registry (`commands.svelte.ts`) read by ribbon, tooltips, title bar and cheat sheet, status bar with sheet tabs and zoom, placing bar; Lucide icons (ISC); old toolbars removed. Owner: ribbon labels 13 px and status bar 12 px vs decision 28 (14 px body); native window title above the in-window title bar |
| T3.16 File view, Options and About | done | File page (Info, New, Open with recent projects, Save, Save as, Export, Print disabled until T3.19, Close project, About, Options), recent list in Rust (`recent-projects.json`, max 10), Settings view removed (decimal rules, company tables, project balloon style in a Drawing tab dialog), About with all bundled license texts (test). Merged through a merge agent (conflicts with T3.17). Owner: third party dependency license list before 0.1? |
| T3.17 Guidance | done | Start screen (cards, how it works, slot for recent projects), setup pane after a new project (general tolerance from Rust's tables, unit and scale, part data; "Set up drawing" reopens it), empty states with a next step in every view and pane. Smoke step 9 |
| T3.18 Properties pane | done | Pane follows the selection (none, one, many) in Drawing; shared `Pane` frame now also used by Measure and Report (merge agent); zone grid and views as pane sections; dialogs for custom scale and numbering options; stale write race fixed (`executeWith`) |
| T3.19 Measure and Report layout | done | Progress counts in `measurement_state` (`to_inspect`, `complete_samples`), Measure bar with value field and verdict words from Rust (`LimitPosition`), report live preview equal to Save copy (test), "Before you issue" checks (`report_checks`), Print through the system viewer, File > Export pane for drawing and lists |
| T3.20 Wording and German pass | done | Glossary in `docs/dev/translations.md` applied (94 messages, 20 dead keys removed), German labels fit at 1280 px (Drawing tab 12 px spare), user guide, manual test M3 and exit procedure on the new shell, ProjectDetails removed; smoke: `until` step, no hot reload in the smoke test (`DIMO_E2E=1`), background throttling off in scripted debug runs. Glossary confirmed by the owner 2026-10-13 |
| T3.21 Exit test with a shop floor user | review | Procedure `docs/perf/M3-ui-test.md` written. Owner: run it with one shop floor person who has not used Dimo, record the findings |
| T3.22 One title bar | review | macOS overlay title bar, traffic lights inside the app title bar (88 px kept free, none in full screen), drag region with only `start-dragging` and `internal-toggle-maximize` (macOS capability file); Windows and Linux keep the native frame. Smoke passes twice. Owner: one real drag and double click on macOS |
| T3.23 Third party license list | done | `node scripts/third-party-licenses.mjs` writes `THIRD_PARTY_LICENSES` (541 crates of the four release targets, Rust std, 20 bundled npm packages, PDFium; about 450 KB), `--check` in check.sh, shown in About, shipped in every installer. Rerun the generator after any change to Cargo.lock, pnpm-lock.yaml, rust-toolchain.toml or pdfium.toml. Follow-up 2026-10-17: the AppImage system libraries (246 files from 95 Ubuntu 22.04 source packages, AppImage runtime and tools) are listed with their Debian copyright files (`scripts/appimage/`), file now about 2 MB; the linux release job checks the real AppImage against that data (`scripts/appimage-libs.sh --check`, never run on a runner yet; on a mismatch it uploads `appimage-license-data` for an update, expect that after runner image updates) |

## Tasks 0.1

Plan: [R01.md](R01.md). Only tasks that have started are listed.

| Task | State | Notes |
|---|---|---|
| R.4 Tolerance tables verified | done | Owner set `verified` in both tables and `checked_by_owner = true` in both vector files 2026-10-16; release gate passes |
| R.11 Version, icon and identity | review | Version 0.1.0 (check-version.sh, version test), app icon and wordmark from brand concept B, name DimoMark, newer project message as smoke step 11. Owner: icon in dock, taskbar and installer |
| R.12 Windows specifics | review | WebView2 check before the window (`webview2.rs`, `tauri::webview_version`, message box through rfd, en and de together), `longPathAware` manifest, recent list keeps paths as chosen (no `\\?\` forms), path tests with umlauts, long paths and UNC (`tests/paths.rs`), owner checklist `docs/perf/R01-platforms.md`. Windows-only tests run on Windows CI only. Owner: MSI and portable zip on Windows 10 22H2 and 11 |
| R.13 Release workflow | done | `.github/workflows/release.yml`: gates (CI green on the commit, version equals tag, tables verified, dependency age, cargo deny, npm licenses), builds macOS universal DMG, Windows MSI and portable zip, Linux AppImage and deb (Ubuntu 22.04), draft release with SHA256SUMS. PDFium bundled (`scripts/stage-pdfium.sh`, `tauri.release.<os>.json`, `tiles::bundled_pdfium`). Test tag `v0.1.0-rc.2` gave a complete draft release 2026-10-17. Installers not yet run on Windows or Linux (R.14) |
| R.9 Check for update button | done | File > About > Check for update: one GET to the GitHub Releases API on click only (ureq with rustls, 11 new crates, all permissive), shows the newer version with a link to the release page, the latest-version or a clear offline message; never downloads. Policy file `{"update_check": false}` hides it (macOS `/Library/Application Support/DimoMark/policy.json`, Windows `%ProgramData%\DimoMark\policy.json`, Linux `/etc/dimomark/policy.json`). Tested with mocked responses; owner tried the button 2026-10-17 |
| R.19 Images as drawings: open and store | done | JPEG, PNG and multi-page TIFF open as projects (start screen, File > New, dialog filter), EXIF and TIFF orientation applied, stored unchanged as `drawings/<sha256>.<ext>`. `dimo_pdf::ImageSource` (image 0.25.10 with jpeg and png, tiff 0.11.3 direct for multi-page and CCITT fax; new crates tiff and fax, both MIT): decodes every page once on open, keeps at most two pages reduced to 300 dpi at sheet size, render key from hash plus edit settings, quarter turns and paper applied (fine angle, corners, trim, clean up refused until R.20). `RasterOpener::open_image`, `Sheet.raster_dpi` set. HEIC sniffed and refused with the two fixes (en, de). Box select on an image finds no text and falls back to manual placement. Run in the app 2026-10-18: phone photo JPEG and 3-page TIFF open, typed balloon gets fit limits, box on the image places a manual balloon, save and reopen keep both; smoke test 78/78 twice. Owner confirmed the 297 mm photo default and the corner frame 2026-10-18. Lists and reports work; the ballooned PDF of an image is refused before the save dialog and its card is off until R.21. Tests: `dimo-pdf/tests/image_source.rs`, `dimo-io/tests/images.rs` (save, reopen, save byte identical), `dimo-export/tests/image_drawing.rs` (exports identical after save and reopen), `src-tauri/tests/image_drawing.rs` (box select), v3 migration test |
| R.20 Image editing: rotate, trim, resize, corners | done | Drawing > Adjust image (button on for photo and scan drawings) and step 1 of the setup pane for images: Rotate (quarter turns, slider ±10°, typed up to ±45°), Corners (handles start at the sheet edge found in the photo, else the image corners), Trim (rectangle), Paper (A4 to A0, Letter, Tabloid, custom mm, portrait or landscape, or from the photo), Clean up; live preview drawn by Rust per stage (`preview_image_edit`), Reset, Apply as one undo step (`apply_image_edit`). Every edit is one projective map per page in `dimo_pdf::image_source::geometry`; tiles sample through its inverse; clean up and corner finding in `clean.rs` and `corners.rs`; `PIXELS_VERSION` 3. `Command::SetImageEdit` carries the `SheetMap` that `dimo_io::project::plan_image_edit` derives: sheet size and dpi, balloons, anchors, source regions, views and zone grid move with the image; the app asks before balloons leave the sheet. Undo and redo switch the drawing back (`ProjectPatched.drawing`). Run in the app 2026-10-18 (UI script, synthetic skewed phone photo with frame and title block): corners found on the paper, A4 landscape with clean up, balloon on "120 ±0.1" moved with the sheet, undo and redo restore it. Owner tested with a real phone photo 2026-10-19: import and adjusting work (R01 R.20 first criterion). Tests: `dimo-pdf` geometry, clean, corners units and `tests/image_source.rs`, `dimo-core/tests/commands.rs` (geometry, undo, old commands), `dimo-io/tests/images.rs` (acceptance: balloon stays on its pixel, undo), `src-tauri/tests/image_drawing.rs`, `image-edit.test.ts`, `image-editor.test.ts` |
| R.21 Ballooned PDF and reports from image drawings | done | The ballooned PDF of a photo or scan is the edited sheet with the balloons on top: `ImageSource::to_pdf` (dimo-pdf `image_source/pdf.rs`) writes one page per sheet, media box = sheet size, the sheet drawn at its kept resolution (at most 300 dpi) in strips of 512 rows; grey pages 8 bit Flate (`miniz_oxide`, already a workspace crate), colour pages JPEG quality 90; no dates, `/ID` from the SHA-256 of the body. `dimo_export::ballooned` then adds the balloons with `write_ballooned` as for PDFs (page content or annotations). Image refusal removed from dimo-export, the shell (`ExportReason::ImageDrawing`) and the export card. Reports already name the original image and its SHA-256 (test). Tests: `dimo-pdf/tests/image_source.rs` (same bytes twice, Flate grey, DCT colour, PDFium reads A4 landscape, picture not flipped, a balloon number in place, a 17000 px wide scan), `dimo-export/tests/image_drawing.rs` (A4 photo project with two balloons: page size, balloon numbers at their sheet points, every export identical after save and reopen, both balloon modes), `dimo-export/tests/plan.rs`. Run in the app 2026-10-19: photo, two typed balloons, File > Export > ballooned PDF; the PDF shows the sheet with both balloons where they were placed. Not printed on paper |
| R.10 Diagnostics and logs | review | File > About > Copy diagnostics (`diagnostics` command): version, platform, PDF engine, log file, settings with user and company name as set/empty, last 200 log lines; home folder, login, user and company name replaced. Log calls reviewed: paths, I/O errors, counters, Typst diagnostics, no drawing content. Owner: read one copied text from a release build |

States: `todo`, `in progress`, `review` (waits for owner), `done`, `blocked` (reason in notes).

## Waiting for the owner

| Item | Since |
|---|---|
| More corpus drawings, especially scans and outlined text (Q-10) | before M4 |
| Trademark search Swissreg / EUIPO (Q-01) | before 0.1 |

## Decision log

Moved to [decisions.md](decisions.md): decisions not in the spec, newest first. Add new ones
there.

## Log

Short entries, newest first: date, task, what changed, anything the next session must know.

- 2026-10-19: Retro follow-ups. `scripts/check.sh` checks the generated files (schemas, bindings, balloon layout fixture, license list) before `cargo test` and runs the tests with `--no-fail-fast`; `scripts/regen.sh` regenerates all of them. Decision log moved to `docs/plan/decisions.md`. spec-reviewer flags weakened assertions in existing tests.
- 2026-10-18: R.20 image editor to review (Adjust image, sheet map with `SetImageEdit`, undo follows the drawing). Check green; run in the app with a synthetic skewed photo. Owner: real phone photo acceptance. Next agent task R.21 (ballooned PDF of images: the edited sheet is drawn by `ImageSource`, tiles or a full render at the paper size).
- 2026-10-10: Wave 3 merged (T2.9, T2.7a, T2.8; four additive conflicts in the command list, capabilities and bindings). Check and two smoke runs green. Left in M2: T2.10 user guide, owner items for T2.9.
- 2026-10-10: ISO 286 table cross-checked and corrected (js/JS rule, range to 3150 mm, CD/EF/FG), merged, check green.
- 2026-10-10: T2.7 merged (numbering strategies) with six additive conflicts in shared desktop files (command list, capabilities, viewport, dev UI script; bindings regenerated) and one test fixed for the T2.6 proposal fields. Check and smoke test green. Wave 2 complete.
- 2026-10-10: T2.6 merged (box select), check green; smoke test failed once (typed text of balloons 4 and 5 lost: editor not focused yet when typing started), passed on rerun. Parallel agents now get their own `CARGO_TARGET_DIR` (two worktrees building into one `target/` kept invalidating each other).
- 2026-10-10: T2.5 merged (tolerance engine), check green. T2.6 must replace its fallback interpreter with `dimo_tolerance::interpret` when merged.
- 2026-10-10: T2.4 merged (data model v2), check and smoke test green on main. Follow-ups carried into T2.5, T2.6, T2.8 in M2.md. Wave 1 complete.
- 2026-10-10: T2.1 merged (callout parser). Wave 1 done except T2.4, held for the owner's confirmation of M2 decisions 1 to 4.
- 2026-10-09: T2.2 and T2.3 merged (tolerance table format, ISO 2768-1 and ISO 286 drafts). Tables wait for the owner's Tabellenbuch check.
- 2026-10-09: M2 plan drafted (`docs/plan/M2.md`): 10 tasks in 3 waves, 6 decisions for the owner to confirm.
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
