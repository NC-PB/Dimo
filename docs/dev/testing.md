# Testing

| Layer | Tool | Where |
|---|---|---|
| Rust unit and integration | `cargo test` | next to the code, `tests/` per crate |
| Parsers | `proptest` round trip (generate, print, parse) | `dimo-notation` |
| Command engine | `proptest` random command, undo and redo sequences against snapshots; numbering rules as tables | `dimo-core/tests/` |
| Tolerance tables | table driven tests from `data/tolerances/*.test.toml` | `dimo-tolerance` |
| Output formats (CSV, XLSX, PDF structure, project JSON) | `insta` snapshots | `dimo-io`, `dimo-report` |
| Generated JSON schemas (`docs/schema/`) | test compares with the types; regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-core --test corpus_truth` (truth) or `-p dimo-io --test schema` (project) | `dimo-core/tests/`, `dimo-io/tests/` |
| Project file migrations | one fixture per schema version in folder mode, never changed after release; new one with `DIMO_WRITE_FIXTURE=1 cargo test -p dimo-io --test migrations` | `dimo-io/tests/fixtures/v<n>/` |
| Recognition quality | corpus evaluation (recall, precision, field accuracy, calibration) | from M4, gated in CI (NFR-REC-05) |
| Frontend units | Vitest | `apps/desktop/src/**/*.test.ts` |
| End to end | `scripts/e2e-smoke.sh` drives the real app through the development hooks (see "End to end tests" below); manual script for the rest | `scripts/e2e/`, `docs/dev/manual-test-m1.md` |
| User guide shortcut reference | vitest compares the generated tables in `docs/user/<lang>/shortcuts.md` with `shortcuts.ts` and the message files; a mismatch prints the tables to paste | `apps/desktop/src/lib/shortcuts-docs.test.ts` |
| Performance | timed harness with a synthetic 50 sheet PDF, run by `scripts/perf.sh` (release build, takes minutes, not in CI) | `crates/dimo-pdf/examples/perf.rs`, generator `dimo-synth --stress`, results in `docs/perf/` |

## Rules

- Every bug fix comes with a test that fails without the fix.
- Snapshot changes are reviewed, never blindly accepted.
- Tests must not need the network. PDFium is loaded from `vendor/pdfium/`; tests that need it
  are skipped with a clear message if it is missing, and CI always provides it. With `CI=true` or
  `DIMO_REQUIRE_PDFIUM=1` a missing library fails these tests instead of skipping them.
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
| `DIMO_DEV_UI_SCRIPT=<file>` | Play these UI steps once the project is shown (steps in `src/lib/dev/ui-script.ts`) |
| `DIMO_DEV_EXPORT_DIR=<dir>` | Exports skip the save dialog and write their suggested file name here |
| `DIMO_DEV_SAVE_AS=<file>` | "Save as" skips the save dialog and saves here |
| `DIMO_DEV_HOME=<dir>` | Settings, autosave and tile cache live in `config`, `data` and `cache` below this folder |

`DIMO_DEV_SCRIPT` (document commands, see `dev.rs`) and the other `DIMO_DEV_*` variables are for
measurements and screenshots.

### Smoke test

```sh
./scripts/e2e-smoke.sh          # about 30 seconds after the build
./scripts/e2e-smoke.sh --keep   # keep the work folder (always kept on failure)
```

It builds the app and the `text_runs` example, starts the Vite dev server (port 1420 must be
free, so stop `tauri dev` first), and runs the app three times. The window opens on screen and
the test needs a desktop session. It is not part of `check.sh`.

1. Main flow on `corpus/drawings/test_drawing_1.pdf`: place five balloons (one by dragging a box),
   edit nominal, deviations and a comment in the table, move a row with Alt+Up, rotate the sheet,
   set scale 2:1 and unit inch, save as, export PDF, CSV and XLSX "as issued", save again.
   Checked from the files: the project (characteristics, exact digits, derived limits, balloons,
   sheet properties, numbering lock, audit log, no journal left), the CSV rows, the XLSX row
   count and strings, and the PDF (five more text runs, the numbers 1 to 5 in the balloon font,
   each at the center of its balloon).
2. Reopen the saved project: balloons, rotation and lock equal the state at the end of step 1.
3. Crash: place two balloons in an unsaved project, kill the app with SIGKILL, start it again:
   the project is restored with a recovery notice.

Work happens in a temporary folder below the target directory with its own `DIMO_DEV_HOME`, so
your settings and your own unsaved work are untouched. The language is forced to English because
the scripts click buttons by their label.

To add a step to the flow, extend `scripts/e2e/main-flow.json` and the checks in the script. A
`report` step writes balloons, selection, rotation, exports and focus to the log, which is what
the script compares.

### Manual test

What the script cannot reach (native file dialogs, the close prompt, pinch zoom feel, looking at
the result) is in `docs/dev/manual-test-m1.md`. Run it before a release and after changes to
dialogs, window events or viewport gestures.

### Other platforms

The script runs on macOS. On Linux it should work under `xvfb-run` with WebKitGTK, and on Windows
it needs a bash environment; neither is tried, and there is no CI job for it. Where WebDriver is
available (Linux with `webkit2gtk-driver`, Windows with Edge WebDriver), WebdriverIO with
tauri-driver can be added later. Until it exists, its place in the table above is taken by the
script and the manual test.
