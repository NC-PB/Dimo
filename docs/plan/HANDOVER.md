# Handover

State of the project at the end of the session of 2026-10-08 to 2026-10-09, for the next session
(human or agent). [STATUS.md](STATUS.md) stays the single source of truth for task states and
decisions; this file explains where things stand, how the work was organized and what to watch for.

## Where things stand

| Milestone | State |
|---|---|
| M0 Foundations | All tasks implemented. CI green on macOS, Windows, Ubuntu (run 37987569387). NFR-PERF-01 to 03 met on the owner's Mac ([perf](../perf/M0.md)). Waits for owner reviews |
| M1 Manual ballooning MVP | All tasks implemented ([plan](M1.md)). End to end smoke test passes (`./scripts/e2e-smoke.sh`). Waits for the owner's manual test and exit criterion |
| M2 Assisted capture | Not planned yet. Next step: draft `docs/plan/M2.md` |

What the app does today (`pnpm dev` from the repository root): new project from a PDF drawing,
place balloons by click or box (B), type values, edit everything in the characteristic table,
reorder and renumber, styles, multi select and group move, undo and redo, sheet rotation, unit and
scale, save, open, autosave and crash recovery, export ballooned PDF, CSV and XLSX (optionally as
"issued", which locks numbering), settings for theme, language, balloon defaults and audit user.
User guide: [docs/user/en/README.md](../user/en/README.md) (German in `docs/user/de/`).

Numbers at handover: 102 commits on `main`, 216 frontend tests, Rust tests across 9 crates plus
`tools/synth`. One local commit (`docs: CI green ...`) and this handover are not pushed yet.

## Waiting for the owner

Nothing below blocks M2 planning, but M2 tolerance work needs item 4.

1. **Review M1 in the app.** Run [docs/dev/manual-test-m1.md](../dev/manual-test-m1.md) (native
   dialogs, close prompt, crash recovery, trackpad gestures, exports in other programs, settings).
2. **Exit criterion M1.** Follow [docs/perf/M1-exit.md](../perf/M1-exit.md) and record the result
   in STATUS.md.
3. **Open decisions** marked "Owner: confirm" in the STATUS decision log. The ones with the most
   impact:
   - Numbering: unlock renumbers to 1..n immediately; an "issued" lock stays if the export fails.
   - Exports: 20 columns, German headers, umlauts in CSV without BOM, rejected characteristics left
     out, non inspected kept with `Inspect = no`, export options remembered per user.
   - Balloons: Open Sans Bold, flag shape (box with pointed left end), D-24 read as 7 mm in sheet
     space.
   - Shortcuts: V, B, S, R, Shift+R, Cmd+N/O/S/Z, Cmd+1, Cmd+E, Cmd+comma, Shift+arrows.
   - German terms: Flachsenkung, Kegelsenkung, Fahnenhinweis; formal "Sie".
   - Uncompressed `.dimo` container; Tauri identifier `io.github.nc-pb.dimo`.
   - specta release candidates and the ignored `paste` advisory; build time npm license
     exceptions (MPL-2.0 lightningcss, unlabeled lix binaries, BlueOak, 0BSD).
4. **Values to verify against the Tabellenbuch** (D-43): truth file
   `corpus/truth/test_drawing_1.truth.json` (c01, c04, c06 ISO 286 limits; c02 and c12 `Ø8 f7`;
   c05 `R15 H7`; whether the burr note c24 belongs in it) and the fit table `FITS` in
   `tools/synth/src/lib.rs`.
5. **GitHub:** enable branch protection on `main` with required checks `check (macos-15)`,
   `check (windows-2025)`, `check (ubuntu-24.04)`, `npm licenses`, `DCO sign-off`.
6. **Corpus:** more drawings, especially scans and outlined text (Q-10). Needed to calibrate the
   text run and sheet kind thresholds before M4.
7. Windows laptop performance numbers (`docs/perf/M0.md`, "to measure").

## How the work was organized

- Tasks were run in **waves of parallel agents**, each in its own git worktree, with model and
  effort chosen per task (Opus high for architecture heavy or tricky work, Sonnet medium or high
  for well bounded work). The coordinator session merged each branch into `main`, ran
  `./scripts/check.sh` on the merged state, updated STATUS.md and the milestone file, and removed
  the worktree. Agents never edited STATUS.md or `M<n>.md` and never pushed.
- Every agent prompt listed: files to read, scope from the milestone file, the 14 day and license
  rules, the shared build directory, how to verify in the real app (dev hooks plus window capture),
  "do not edit STATUS/M<n>", commit with `-s`, and a short final report with decisions for the log.
- Merges with real conflicts between two feature branches were handed to a dedicated merge agent
  in a worktree instead of being resolved by hand on `main`.

## Pitfalls learned (read before running agents)

- **Disk space.** Each worktree used to build its own `target/` (2 to 4 GB). Agents now share the
  main one: `export CARGO_TARGET_DIR=/Users/peterburgener/Repositories/dimo/target`. The shared
  `target/` still grows to about 30 GB. Delete `target/debug/incremental` after each wave. The disk
  ran full twice; when it is full even the Bash tool fails, then free space from the terminal panel.
- **Stale paths with a shared target.** Code using `env!("CARGO_MANIFEST_DIR")` bakes in the path
  of the worktree that compiled it. Tests now read the variable at run time, and
  `dimo_pdf::library::vendor_library_path` searches upwards from the run time path. If a build
  script or test fails with a path into `.claude/worktrees/...`, `touch` the file or its `build.rs`.
- **Check the real exit code.** `./scripts/check.sh | grep ...` hides failures. Redirect to a log
  and test `$?` before committing. Run the full check, not single crates, before every commit
  (formatting of `build.rs` broke CI once).
- **14 day rule for crates.** Cargo cannot enforce it on stable. After any `Cargo.lock` change run
  `./scripts/check-lock-age.sh` (slow, about one request per second; crates.io rate limits
  parallel runs, the sparse index `https://index.crates.io` has `pubtime` as a fallback).
  Downgrade with `cargo update -p name@version --precise older`. Tauri sibling crates must stay on
  the 2.11.6 release set. pnpm enforces the rule itself (`minimumReleaseAge`).
- **macOS app verification.** The native window cannot be clicked by tools. Use the debug only
  `DIMO_DEV_*` hooks and the dev UI script (`apps/desktop/src-tauri/src/dev.rs`), find the window
  with a small Swift `CGWindowList` script and capture it with `screencapture -l <id>`. Kill only
  processes you started, by PID.
- **corepack** 0.34 (bundled with Node 22.20) cannot start pnpm 12, so pnpm is pinned to 11.27.1.
  `COREPACK_ENABLE_DOWNLOAD_PROMPT=0` is set in `.claude/settings.json`.
- **Permissions.** The allow list in `.claude/settings.json` covers routine local commands. `git
  push`, `curl`, `rm -rf`, `cargo add/install`, `pnpm add` still ask. The guard hook enforces the
  hard rules (sign-off, corpus, LICENSE, accepted ADRs, verified tolerance tables).
- **Intermittent:** the three `dimo-io` export snapshot tests failed once in a full local check and
  never again in 8 runs. Watch CI.

## Useful commands

```sh
pnpm dev                                  # run the app
DIMO_REQUIRE_PDFIUM=1 ./scripts/check.sh  # everything CI runs, PDFium tests required
./scripts/e2e-smoke.sh                    # end to end smoke test of the M1 flow (about 30 s)
./scripts/check-lock-age.sh               # 14 day rule for Cargo.lock (network)
./scripts/perf.sh                         # performance harness (release build)
./scripts/fetch-pdfium.sh                 # PDFium binaries into vendor/
cargo run -p dimo-synth -- --seed 7 --count 6   # synthetic drawing plus truth file
```

## Next steps

1. Push the local commits (`git push origin main`) and watch CI.
2. Owner works through "Waiting for the owner" above; record outcomes in STATUS.md.
3. Draft `docs/plan/M2.md` (box select with PDF text, callout parser in `dimo-notation`, tolerance
   engine and tables in `dimo-tolerance` and `data/tolerances/`, explanations, numbering
   strategies) from the roadmap, FR-TOL-*, FR-BAL-04/05/07, FR-CHR-08 and spec 08. Tolerance tables
   are drafts until the owner verifies them (rule 7, D-43; use the `tolerance-table` skill).
4. Run M2 in waves as above.
