# MF Foundation

Goal: before any release 0.1 feature work, close the gaps the code review of 2026-10-14 found
where the next work pushes: one drawing source abstraction instead of PDF everywhere, domain
orchestration out of the shell, structured errors, consistent update commands, fewer merge
conflict hotspots, logging, and a frontend that can host the image editor and more dialogs.
Owner decision 2026-10-14: this is the next step, separate from everything else.

**Exit criterion:** F1 to F9 done; the app behaves exactly as before for the user (all tests and
the smoke test pass unchanged in meaning; exports byte identical to before for the same project);
adding an image drawing source, a first run dialog and a new command each need only the documented
touch points (checklists in `docs/dev/rust.md` and `docs/dev/frontend.md`).

Status: **planned 2026-10-14, no code until the owner says so.**

Rules that matter most here: rule 2 (Rust owns the document), rule 4 (one coordinate system),
rule 5 (exact decimals), rule 6 (generated IPC types), rule 11 (determinism), NFR-REL-03 (schema
migrations with tests). No new user features in this milestone; refactoring keeps behaviour.

Source of the findings: three read-only reviews (Rust core crates, desktop shell, frontend),
summarized in the STATUS decision log 2026-10-14.

---

## Decisions taken in this plan

1. **One project file migration.** F1 and F4 change the project file; they share schema version 4
   (with the image drawing source prepared but not yet usable), so release 0.1 does not need a
   second migration for images.
2. **Behaviour stays.** Every task proves unchanged behaviour: existing tests keep their meaning,
   export snapshots stay byte identical, the smoke test passes twice in a row.
3. **Order by risk.** Mechanical splits first (they shrink later diffs), then the data model and
   interfaces, then the shell and frontend that build on them.

---

## Wave 1 (parallel, mechanical)

### F5 Split `command.rs`, checklists

- `dimo-core/src/command.rs` (1422 lines) becomes `command/` with `mod.rs` (Tx, dispatch),
  `characteristics.rs`, `balloons.rs`, `sheets.rs`, `numbering.rs`, `measurement.rs`,
  `reports.rs`; `patch.rs` `apply_change` split the same way. No API change.
- `Change::inverse` generated or simplified (a small macro or a generic replaced-value pattern).
- "New command" and "new persisted field" checklists in `docs/dev/rust.md` (today about 7 to 11
  touch points for a command, about 10 for a field).

Acceptance:
- [x] No source file in `dimo-core/src` above about 600 lines; all tests pass unchanged
- [x] Checklists written

### F6 One command table

- One table file lists every IPC command once; `build.rs` reads it for the Tauri manifest and
  writes one permission set, `capabilities/default.json` references that set, `lib.rs` has a test
  that the table equals the specta command list. `BINDINGS_HEADER` drops its module list. Document
  "on a `bindings.ts` conflict, regenerate" in HANDOVER.md.

Acceptance:
- [x] Adding a command touches the function, the table and the generated bindings only
- [x] Test fails when the table and the registered commands differ

### F9 Small cleanups

- One `Language` type in `dimo-core` (replaces four), one decimal-to-spreadsheet-number helper
  (replaces two copies), rule ids derived from the serde names (replaces two string tables),
  `dimo-pdf` uses the core geometry types (`Rect`, `Size`) instead of its own.
- Timing assertions (20 ms, 50 ms, 5 s) move to ignored performance tests run in a separate CI
  step; the 2.1 s sleeps that prove clock independence are replaced by an injected clock or a
  golden file comparison.
- Remove the unused `dimo-vision` dependency of `dimo-detect` (the crate stays as a stub for OCR).
- Stop the wave-shaped test helpers: no `model_v4.rs` or `edit_v4`; extend one builder.

Acceptance:
- [x] Full test run about 8 s faster; no timing assertion in the default test run
- [x] Export snapshots unchanged

---

## Wave 2 (data model and interfaces)

### F1 Drawing source abstraction

- `DrawingRevision` gains `source: DrawingSource { Pdf, Image { format, edit } }` (image variant
  prepared for release 0.1, not yet creatable); stored file name by source; schema version 4 with
  a migration from 3 and fixtures; JSON Schema regenerated.
- A `RasterSource` trait (sheet count, sheet size, render region at a scale); `TileService` works
  on it; the PDF document implements it. The tile cache key includes the source kind.
- "Create a project from a drawing file" moves from `src-tauri/src/session.rs` into `dimo-io`
  (one function that sniffs the file type and dispatches); the three duplicated create branches
  and the four `ProjectSession::create*` constructors collapse into one with an options struct.
- One hash type (`dimo_pdf::ContentHash` and `dimo_core::Sha256Hex` merged).
- A `RevisionChanged` change and a command to edit revision settings (used by images in 0.1).
- Plan mode; ADR for the drawing source type and schema 4.

Acceptance:
- [x] Version 3 projects open and save as version 4 (migration test); exports byte identical
- [x] The shell no longer calls PDFium to create a project
- [x] A test raster source (synthetic image) renders tiles through `TileService`

### F4 Field-level update commands

- `UpdateProjectInfo`, `UpdateReportInfo` and `UpdateSheet` become field-value lists
  (`ProjectInfoField`, `ReportInfoField`, `SheetField`), like `UpdateFields`; samples the same
  (`SampleField`). The frontend's `executeWith` whole-object writes for these go away. Decimal rule
  add and remove as their own commands. Shares schema 4 with F1 (audit entries migrated).

Acceptance:
- [ ] Two fast edits of different fields both survive without `executeWith` (test)
- [ ] No frontend code builds a whole project info, report info, sheet or sample object to send

### F3 Structured errors

- `RejectReason` moves into `dimo-core`; every error that reaches the UI carries a machine kind
  (`ErrorKind`) mapped in one module from `ProjectError`, `PdfError`, export and report errors,
  task join errors (`From<JoinError>`), autosave, table import and recent list errors; the message
  stays as diagnostic text only. Misused kinds fixed (Io for join errors and window closing,
  Project for table loading, PdfiumUnavailable for export engines).
- Frontend translates every kind in en and de; no Rust text shown to the user except the
  diagnostic detail in an expandable line.

Acceptance:
- [x] Every user-visible error is translated (test over all kinds); German UI shows no English
      error text in the smoke test

---

## Wave 3 (shell and frontend, after wave 2)

### F2 Domain orchestration out of the shell

- "Export as issued" (lock numbering, store filled report info, record the issued report),
  the report date rule (today exists three times: `session.rs`, `export.rs`, `report.rs`), the
  issued report info fill, the initial report info of new projects, report checks and report
  warnings move into `dimo-report` / `dimo-core` as one "report issuing and export plan" API that
  takes defaults (company, user, today) and returns commands plus a snapshot.
- One export registry (formats, render through one function) in a small `dimo-export` facade or
  a module of `dimo-report`; the shell keeps only the job, cancel and file writing. The CLI can
  export through the same function.
- `CommandError::MissingTable` in core; region validation and the empty shorter leg rule move out
  of `recognition.rs`.

Acceptance:
- [ ] One report date rule; preview, Save copy, Print and issued export use it (test)
- [ ] The CLI exports a check sheet and an inspection report byte identical to the app (test)

### F7 Logging and shared shell helpers

- A tracing subscriber at startup writing a size-capped log file in the app log folder (basis for
  "Copy diagnostics", R.10); journal, export and settings failures logged.
- `dialogs.rs` (pick and save, with the dev override inside), `fsio.rs` (atomic write with a
  unique temp file; a `JsonStore<T>` for settings, recent projects and later first run and update
  check state; fixes the shared `.json.tmp` race), an `ipc::blocking` helper (replaces seven
  `spawn_blocking(..).map_err(Io)` copies), `export.rs` split into request, pipeline, job and
  opener.
- Opening files on Windows through the system API instead of `cmd /C start`; an `open_url` that
  accepts only the GitHub release page prefix (used by the update check in R.9).

Acceptance:
- [ ] A forced journal write failure appears in the log file
- [ ] `export.rs` and `session.rs` below about 500 lines each

### F8 Frontend foundation

- Shortcuts: a modal counter set by one shared `AppDialog` component blocks single key shortcuts
  while any dialog is open; named shortcut scopes (`drawing`, `grid`, later `imageEditor`,
  `review`) replace `anyView` / `viewportOnly`. Dispatcher test for dialog gating (today the
  project settings dialog lets drawing shortcuts through).
- `SheetCanvas.svelte` split out of `Viewport.svelte` (tiles, transform, pan, zoom, fit, an
  overlay slot); `Viewport` becomes canvas plus balloon overlay and gestures; ready for the image
  editor's handle overlay.
- `AppDialog`, a `TextField` / field class module and a `commitField()` helper replace the copied
  dialogs, field classes and edit-send-show patterns; one blank columns clamp.
- Shared test fakes in `lib/test/fakes.ts`, one exported `Result` type; tests select by command
  id or message function instead of English text.
- Dev modules loaded only through dynamic import in development; CI checks the release bundle
  contains no dev hooks.
- Smoke test: the first run right after a rebuild failed twice in F1 (different UI timing checks: general tolerance not stored, first balloon missing), later runs pass. Find the cause (for example Vite optimizing dependencies on the first load) and make the first run reliable.
- The delete confirmation count, sheet assignment of a characteristic and the result sort order
  come from Rust instead of TypeScript (rule 2 drift).

Acceptance:
- [ ] Opening any dialog blocks drawing shortcuts (test)
- [ ] `SheetCanvas` renders a sheet without the balloon layer (component test)
- [ ] Release bundle contains no dev script code (CI check)

---

## Collected for later (not in MF)

From the same review, planned before M4 (automatic recognition, see the roadmap):

- Token source interface (neutral token type, PDF adapter; box select and layout analysis take
  tokens), so OCR plugs into the same pipeline.
- General job runner (`spawn_job` with progress steps and cancel) for recognition jobs.
- Command registry with `run(ctx)` and `when()` (review queue, contextual tabs), central error
  handling for failed commands.
- Measure grid and characteristic table logic in controllers (1164 and 950 lines today).
- Store reset through a session hook instead of App effects; one store injection pattern.
- Feature folders (`drawing/`, `measure/`, `report/`, `review/`, `image/`, `shell/`).
- Evaluation code (`truth.rs`, box select evaluation) behind a feature or in a `dimo-eval` crate.
- One test fixture builder (`dimo-testkit`).
- Reference counted documents in the tile service; print folder cleanup.

Later: crash recovery and exit locking off the main thread, narrower capabilities, dev commands
registered only in debug builds.

---

## Order

1. Wave 1: F5, F6, F9 in parallel.
2. Wave 2: F1 first (plan mode, ADR), F4 and F3 in parallel once F1's data model is settled.
3. Wave 3: F2, F7, F8 in parallel.
4. Then release 0.1 work (`docs/plan/R01.md`) on the new foundation.
