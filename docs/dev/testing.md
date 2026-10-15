# Testing

| Layer | Tool | Where |
|---|---|---|
| Rust unit and integration | `cargo test` | next to the code, `tests/` per crate |
| Parsers | `proptest` round trip (generate, print, parse) | `dimo-notation` |
| Command engine | `proptest` random command, undo and redo sequences against snapshots; numbering rules as tables | `dimo-core/tests/` |
| Tolerance tables | table driven tests from `data/tolerances/*.test.toml` | `dimo-tolerance` |
| Output formats (CSV, XLSX, PDF structure, project JSON, report data JSON) | `insta` snapshots | `dimo-io`, `dimo-report` |
| Report PDF text | text runs extracted with PDFium, `insta` snapshots (dimo-report may not use dimo-pdf) | `dimo-cli/tests/report_text.rs` |
| Generated JSON schemas (`docs/schema/`) | test compares with the types; regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-core --test corpus_truth` (truth) or `-p dimo-io --test schema` (project) or `-p dimo-tolerance --test schema` (tolerance tables) or `-p dimo-report --test report` (report data) | `dimo-core/tests/`, `dimo-io/tests/`, `dimo-tolerance/tests/`, `dimo-report/tests/` |
| Export determinism | list exports compared byte for byte with committed golden files; regenerate after an intended change with `DIMO_UPDATE_GOLDEN=1 cargo test -p dimo-io --test export_list` | `dimo-io/tests/golden/` |
| Project file migrations | one fixture per schema version in folder mode, never changed after release; new one with `DIMO_WRITE_FIXTURE=1 cargo test -p dimo-io --test migrations` | `dimo-io/tests/fixtures/v<n>/` |
| Box select (M2 exit criterion) | box select with the tolerance engine on every truth region of the corpus and of synth seeds 1 to 40, per callout category; fails below 99 percent of the common callouts, limits required (angles under a general tolerance read with the shorter leg of the truth, as typed on the card). Report: `cargo run -p dimo-cli -- eval box-select` | `dimo-detect/src/evaluation.rs`, `dimo-detect/tests/box_select_corpus.rs`, `dimo-cli/tests/box_select_evaluation.rs`, run by `check.sh` in CI |
| Recognition quality | corpus evaluation (recall, precision, field accuracy, calibration) | from M4, gated in CI (NFR-REC-05) |
| Frontend units | Vitest; fakes of the Rust commands from `src/lib/test/fakes.ts` | `apps/desktop/src/**/*.test.ts` |
| Release bundle | `scripts/check-release-bundle.mjs` builds the production frontend and fails if it holds code of `src/lib/dev` (run by `check.sh`) | `scripts/` |
| End to end | `scripts/e2e-smoke.sh` drives the real app through the development hooks (see "End to end tests" below); manual script for the rest | `scripts/e2e/`, `docs/dev/manual-test-m1.md` to `m3` |
| User guide shortcut reference | vitest compares the generated tables in `docs/user/<lang>/shortcuts.md` with `shortcuts.ts` and the message files; a mismatch prints the tables to paste | `apps/desktop/src/lib/shortcuts-docs.test.ts` |
| Timing bounds | `#[ignore]` tests named `perf_*` (measurement state 20 ms, box select 50 ms, XLSX report 5 s), never in the default run; `scripts/perf-tests.sh` runs them one at a time, CI job `perf-tests` on macOS (informative, shared runners vary) | `src-tauri/tests/measurement.rs`, `src-tauri/tests/recognition.rs`, `dimo-report/tests/xlsx.rs` |
| Performance | timed harness with a synthetic 50 sheet PDF, run by `scripts/perf.sh` (release build, takes minutes, not in CI) | `crates/dimo-pdf/examples/perf.rs`, generator `dimo-synth --stress`, results in `docs/perf/` |

## Rules

- Every bug fix comes with a test that fails without the fix.
- Test data builders grow with the model: extend the existing builder or fixture instead of
  adding one per schema wave (no new `model_v<n>.rs` or `edit_v<n>` helpers).
- Timing bounds never run in the default test run: name such a test `perf_*` and mark it
  `#[ignore]`. Hand run release measurements that assert nothing (for example
  `render_time_300_by_10` in `src-tauri/tests/report_export.rs`) stay ignored under their own
  name.
- Snapshot changes are reviewed, never blindly accepted.
- Tests must not need the network. PDFium is loaded from `vendor/pdfium/`; tests that need it
  are skipped with a clear message if it is missing, and CI always provides it. With `CI=true` or
  `DIMO_REQUIRE_PDFIUM=1` a missing library fails these tests instead of skipping them.
- Frontend tests fake Rust with `src/lib/test/fakes.ts` (project, tolerance, numbering and
  measurement commands, a layout stub for happy-dom) and take the result type from
  `$lib/ipc/result`. They find elements by `data-*` attributes (`data-command`, `data-tab`,
  `data-answer`, `data-testid`) and compare texts with the message functions (`m.save()`), not
  with English words, so a reworded label does not break them.
- Corpus based tests reference drawings by file name and verify the SHA-256 from `PROVENANCE.md`.
- Ground truth files `corpus/truth/*.truth.json` are validated against `docs/schema/truth.schema.json`
  and checked for consistency (`dimo_core::truth`). Expected limits come from the drawing and the
  tolerance rules, never from recognizer output.
- `corpus/drawings/test_drawing_1.pdf` is a required regression test: it has a real text layer
  (74 text objects) that some libraries fail to see. Sheet classification must report `vector_text`.

## End to end tests

The tech stack names WebdriverIO with tauri-driver for end to end tests. tauri-driver needs a
WebDriver server for the platform's web view, and WKWebView on macOS has none, so it cannot drive
the app on the main development platform. Instead the app drives itself: debug builds read
`DIMO_DEV_*` environment variables (`apps/desktop/src-tauri/src/dev.rs`) that open a drawing, play a
JSON script of pointer, key and button steps in the webview and write what happened to the
terminal. Release builds ignore all of them (NFR-SEC-01).

| Variable | Effect |
|---|---|
| `DIMO_DEV_OPEN=<path>` | Create a project from this PDF, or open this `.dimo` file, at startup |
| `DIMO_DEV_UI_SCRIPT=<file>` | Play these UI steps once the project (or, without one, the start screen) is shown (steps in `src/lib/dev/ui-script.ts`) |
| `DIMO_DEV_EXPORT_DIR=<dir>` | Exports skip the save dialog and write their suggested file name here |
| `DIMO_DEV_SAVE_AS=<file>` | "Save as" skips the save dialog and saves here |
| `DIMO_DEV_NEW_DRAWING=<pdf>` | "New project" skips the open dialog and uses this PDF (start screen and setup pane, T3.17) |
| `DIMO_DEV_HOME=<dir>` | Settings, autosave and tile cache live in `config`, `data` and `cache` below this folder |

`DIMO_DEV_SCRIPT` (document commands, see `dev.rs`) and the other `DIMO_DEV_*` variables are for
measurements and screenshots.

### Smoke test

```sh
./scripts/e2e-smoke.sh          # about 30 seconds after the build
./scripts/e2e-smoke.sh --keep   # keep the work folder (always kept on failure)
```

It builds the app and the `text_runs` example, starts the Vite dev server (port 1420 must be
free, so stop `tauri dev` first), and runs the app several times. The window opens on screen and
the test needs a desktop session. It is not part of `check.sh`. Moving the mouse over the window
while it runs is harmless: the UI script's pointer events carry their own pointer ID, so the
drawing gestures ignore the real mouse, and the script prints how many real pointer events it saw.

1. Main flow on `corpus/drawings/test_drawing_1.pdf`: place five balloons (one by dragging a box,
   two typed as key events right after the click without waiting for the value field),
   edit nominal, deviations and a comment in the table, move a row with Alt+Up, rotate the sheet,
   set scale 2:1 and unit inch, save as, export PDF, CSV and XLSX "as issued", save again.
   Checked from the files: the project (characteristics, exact digits, derived limits, balloons,
   sheet properties, numbering lock, audit log, no journal left), the CSV rows, the XLSX row
   count and strings, and the PDF (five more text runs, the numbers 1 to 5 in the balloon font,
   each at the center of its balloon).
2. Reopen the saved project: balloons, rotation and lock equal the state at the end of step 1.
3. Crash: place two balloons in an unsaved project, kill the app with SIGKILL, start it again:
   the project is restored with a recovery notice.
4. Numbering (`scripts/e2e/numbering.json`): place five balloons, draw a zone grid frame, preview
   and apply "sheet, zone, reading order", draw a view, preview and apply "clockwise per view",
   undo. The ghost numbers of each preview must equal the numbers the apply then gives.
5. Tolerance settings (`tolerance.json`): box select without and with a general tolerance, and
   re-interpret.
6. Project and report details (`details.json`): fields typed into the side panel, company name
   and inspector from the settings of the user (Options).
7. Measurement grid (`measure.json`, `measure-setup.json`): three samples, one column typed with
   the keyboard only, a 5 by 3 block pasted (a synthetic paste event), undone and redone.
8. Report export (`report-export.json`, `report-cancel.json`): the inspection report as PDF and
   XLSX twice (byte identical), a cancelled export that leaves no file. The PDF text is read
   back with PDFium (`text_runs`): overall verdict, summary counts and the verdict of each
   sample; the XLSX shared strings hold the verdict words.
9. Start screen and setup pane (`start-setup.json`): no project at startup, the start screen card
   creates one, the setup pane sets general tolerance, unit, scale, part number and customer, and
   Start ballooning switches on Place balloon; checked from the saved project.

10. Error messages in German (`errors.json`): a broken company table and a text file as new
    drawing are refused with German texts.

The last check makes sure the dev server never reloaded the page during the run.

The first run after a build used to fail now and then while later runs passed (MF F8). Two
causes, both fixed: Vite reused a dependency cache from older code, found new imports only when
the app loaded them and reloaded the page (`optimized dependencies changed. reloading` in
`vite.log`), which ends a running UI script; the dev server of the smoke test now bundles the
dependencies from a full scan at every start (`optimizeDeps.force` with `DIMO_E2E=1`). And the UI
script gave a value field three seconds to open and then typed into nothing; the first commands
after a build (PDFium start, empty tile cache) and a machine busy with parallel builds take
longer. Steps now wait for their target (field, button, option, row, cell) for up to 10 to 20
seconds and fail with the reason when it does not come, and a script starts only once the first
tiles of the drawing are shown.

Work happens in a temporary folder below the target directory with its own `DIMO_DEV_HOME`, so
your settings and your own unsaved work are untouched. The language is forced to English because
the scripts click buttons by their label.

To add a step to the flow, extend `scripts/e2e/main-flow.json` and the checks in the script. A
`report` step writes balloons, selection, rotation, exports and focus to the log, which is what
the script compares.

### Manual test

What the script cannot reach (native file dialogs, the close prompt, pinch zoom feel, looking at
the result) is in `docs/dev/manual-test-m1.md`. Run it before a release and after changes to
dialogs, window events or viewport gestures. `docs/dev/manual-test-m2.md` covers box select and
the tolerance views, `docs/dev/manual-test-m3.md` the measurement grid with the real clipboard,
the reports in a spreadsheet program and in print, and the German wording.

### Other platforms

The script runs on macOS. On Linux it should work under `xvfb-run` with WebKitGTK, and on Windows
it needs a bash environment; neither is tried, and there is no CI job for it. Where WebDriver is
available (Linux with `webkit2gtk-driver`, Windows with Edge WebDriver), WebdriverIO with
tauri-driver can be added later. Until it exists, its place in the table above is taken by the
script and the manual test.
