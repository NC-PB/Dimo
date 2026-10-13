# Frontend conventions

Location: `apps/desktop/src`. Svelte 5 with runes, TypeScript strict, Vite, no SSR, no router (ADR 0007).

## Rules

- No business logic. The frontend never computes limits, numbering or pass/fail. It renders what
  Rust sends and dispatches commands. If you need a derived value, add it to the Rust response.
- IPC only through the generated bindings in `src/lib/ipc/`. Never call `invoke` with a string name.
- State: runes based stores in `src/lib/stores/`. Patches from Rust are applied in one place.
- Interactive gestures (drag, box select) update local view state optimistically and send one
  command on release.
- Viewport: tiles via `dimo://tile/...`, one SVG overlay in sheet space with a single transform.
  Do not render per element transforms for pan and zoom.
- Tables: TanStack Table plus TanStack Virtual. Never render all rows.
- UI primitives: Bits UI. Styling: Tailwind 4 with design tokens in one theme file, light and dark (D-51).
- Colors (M3 decision 24): `bg`, `surface`, `sunken`, `hover`, `border`, `border-strong`, `text`,
  `text-muted`. `accent` (orange) is for marks only: selection bars, underlines, outlines, the
  focus ring. Filled buttons and accent colored text use `bg-accent-strong text-accent-ink`
  (plain `accent` with white text is below 4.5:1). `accent-soft` tints a selected background.
  `danger`, `success` and `warning` never use the accent. `tokens.test.ts` checks the contrast of
  every pair in both themes; add a pair there when you introduce a new combination.
- Fonts (decision 25): body text is `system-ui`; `h1` to `h3` and the brand name use Space
  Grotesk (`font-heading`), values and code `font-mono` (JetBrains Mono). The files are bundled
  from `data/fonts/` (`theme/fonts.css`), never loaded from the network.
- Text: every user visible string goes through Paraglide messages, English and German (FR-SET-04).
- Shortcuts: keys defined in one file (`shortcuts.ts`), shown in the `?` cheat sheet (D-52); commands
  name their shortcut from the registry (`commands.svelte.ts`). Every action reachable by keyboard.
- Pass/fail and status always use shape plus color, never color alone (NFR-UX-05, D-24).
- Accessibility: semantic elements, labels on controls, focus visible.

## Project state

`src/lib/stores/project.svelte.ts` (`projectStore`) mirrors the open project. It is the only place
that applies Rust's state:

- `project-loaded` (and `project_state` at startup): full snapshot, or `null` when closed.
  `session` increases with every load or close.
- `project-patched`: a `Patch` with a `revision` that increases by one. A gap or a patch that does
  not fit makes the store fetch the full state again.
- `project-status-changed`: file name, modified, undo/redo and autosave state after save or an
  autosave write.

`projectStore.project` is replaced, never mutated (`$state.raw`). `applyChanges` in `patch.ts`
gives new objects only to changed lists and items, so unchanged rows and balloons keep their
identity for keyed lists and TanStack memoization.

Reading: `characteristics`, `characteristicById`, `sheets` (current revision), `balloonsOnSheet(id)`,
`status`, `canUndo`, `canRedo`. Changing: `execute(command)`, `undo()`, `redo()`. They run one after
another in call order. When `execute` resolves the store already shows the change (the command
result and the event carry the same revision; the first one applies), and the returned patch tells
you what was created, for example the ID of a new characteristic to select. Several changes that
must be one undo step go into one `batch` command. Selection, hover and tool state are view state
and belong in their own stores, keyed by ID.

New, open and window close ask about unsaved changes through `unsavedPrompt`
(`UnsavedChangesDialog.svelte`); Rust refuses to drop changes unless the call says `discard`.

## Selection and the characteristic table

`src/lib/stores/selection.svelte.ts` holds the selected characteristic IDs for the viewport and
the table: `ids`, `primary`, `size`, `isEmpty`, `has(id)`; `select(ids, primary?)`, `add`,
`remove`, `toggle`, `clear`, `retain(exists)`; `focus(id, from)` for "edit this one now". The
view named in `from` (`"viewport"` or `"table"`) takes the keyboard focus, the other one only
brings the item into view: after placing a balloon the viewport's value field has the keys and
the table only makes the row active.

The viewport follows a selection of exactly one characteristic (`viewport/reveal.ts`): it shows
the balloon's sheet and centers the balloon when it is out of view or on another sheet. Group
selections never move the view. While the table has focus, single key window shortcuts (R, V,
B, S, zoom, PageUp) are ignored (`worksInTable` in `shortcuts.ts`); Cmd/Ctrl shortcuts still work.

The table (`src/lib/table/`) uses TanStack Table for the column and row model and
`@tanstack/virtual-core` (wrapped in `virtual.svelte.ts`) with one fixed row height. Cells send
typed text as `update_fields`; decimals are never parsed in TypeScript, and a quantity is
read only if it is plain digits (anything else goes to Rust as `NaN`, which it refuses). Rust's
refusal is shown in the table's status line: `rejected` errors carry a machine readable `reason`
(`RejectReason`) that `src/lib/i18n.ts` translates, unknown reasons show Rust's English text.
Reordering sends one `move_characteristics`, also while numbering is locked: Rust refuses it with
`numbering_locked` and the table explains the lock.

To check scrolling with a large list in `tauri dev`:

```sh
node scripts/dev-characteristics.mjs 1000 target/dev/chars-1000.json
DIMO_DEV_OPEN=$PWD/corpus/drawings/test_drawing_1.pdf DIMO_DEV_SCRIPT=$PWD/target/dev/chars-1000.json \
  VITE_DIMO_DEV_TABLE_CHECK=1 pnpm -C apps/desktop tauri dev
```

The frame times of a scripted scroll are printed to the terminal ("0 balloons", the viewport size
is the table's) and shown in the developer tools.

## Views, settings and exports

The shell follows [ADR 0007](../adr/0007-office-style-shell.md) (`src/lib/shell/`): `TitleBar`,
`TabBar` with one ribbon per tab (`HomeRibbon`, `DrawingRibbon`, ...), the work area and
`StatusBar`. `stores/view.svelte.ts` holds the tab (`tab`: file, home, drawing, review, measure,
report, view) and the work area it shows (`current`: drawing, review, measure, export, settings,
file); Home and Drawing both show the drawing, the View tab keeps the area that was shown. The
drawing stays mounted (hidden) while another area is shown, so tiles, zoom and table scroll are
kept; drawing shortcuts act only while it is shown (`anyView` in `shortcuts.ts` marks the
others). Hiding the table or the side panel (View tab) hides them with CSS, they stay mounted.

Commands (T3.15, decision 27): `src/lib/commands.svelte.ts` defines every ribbon and title bar
command once: label, icon (`src/lib/icons.ts`, Lucide, one import per icon), the shortcut action
of `shortcuts.ts`, a tooltip sentence, `disabled()` returning the reason or `null`, `pressed()`
for on/off commands, and `run()`. `CommandButton` renders one (large, small or icon only);
disabled commands use `aria-disabled` so the tooltip with the reason still shows. The window
shortcut handler (`Shortcuts.svelte`) runs the same command when a key has one, so key and
button share one disabled state. Keys are defined only in `shortcuts.ts`; a command never repeats
them. A new command gets an entry in the registry, a message for label and description in both
languages, and a place in a ribbon; `commands.test.ts` checks labels, icons, tooltips and
shortcuts. Ribbon groups that do not fit the window move into "More" (`ribbon-fit.ts`).

- Settings of the user (`stores/settings.svelte.ts`): theme, UI language, audit user name and
  the last export options. Rust stores them in `settings.json` in the app config directory
  (`src-tauri/src/settings.rs`). `main.ts` reads them before mounting, so the first render has
  the stored language and theme. The theme sets `data-theme` on `<html>` (`theme/theme.ts`);
  colors live only in `theme/tokens.css`.
- Exports (`stores/export.svelte.ts`): `export_project` opens the save dialog in Rust and
  returns a job ID; `job-progress` and `job-finished` events carry the progress and the result.
  Events can arrive before the ID, so job states are kept by ID.
- Tolerances (`stores/tolerance.svelte.ts`, T2.8): `tolerance_tables` lists shipped and custom
  tables with their classes and draft state, settings change with `set_tolerance_settings`,
  `import_tolerance_table` opens the file dialog in Rust (`DIMO_DEV_IMPORT_TABLE` skips it in
  debug builds), `reinterpret_characteristics` re-reads the selection as one undo step, and
  `explain_characteristic` and `characteristic_history` feed the detail panel
  (`CharacteristicDetail.svelte`). Markers of a derivation are drawn by `ToleranceBadges.svelte`
  only, each with its own shape.
- Verdicts (T3.3): `measurement_state` returns the verdict of every cell (`pass`, `fail`,
  `not_evaluated`, `missing`), the effective result type and draft mark per row, and counts with
  a `pass`, `fail` or `open` verdict per row, per sample and for the report. Rows are the
  measured characteristics in number order, cells follow the sample order. Call it again after
  each `project-patched` event; the frontend never compares values with limits (rule 2).
- Measurement grid (T3.5, `src/lib/measure/`): `MeasureView` refreshes `measurementStore`
  (`stores/measurement.svelte.ts`) after every project change, and asks for
  `changed_since_issued_report` (decision 16 hint marker) only when a report was issued. Rows
  come from `measurement_state`, not from a frontend filter. Rows are virtualized
  (`RowVirtualizer`), sample columns are windowed by hand (fixed width), the left columns are
  sticky. Every commit is one `set_results` (typed text, Rust parses it); the active cell moves on
  at once and a refused value is opened again with the reason. Cmd/Ctrl+V moves the focus to a
  hidden text field for the browser's paste, whose `paste` event the grid handles; the dev UI
  script steps `cell` and `paste` drive it. Grid keys are the `grid` scope of `shortcuts.ts`. In
  `tauri dev`, `devTools.commitTimes` holds commit to repaint times and the UI script report
  prints their mean; `node scripts/dev-characteristics.mjs 300 out.json 10` makes a 300 by 10
  project for the check.
- Balloon sizes: `viewport/balloons.ts` follows `dimo_core::BalloonMetrics` with the generated
  `BALLOON_METRICS`. The Rust test `balloon_layout` writes `balloon-layout.fixture.json`, and
  `balloons.test.ts` checks the viewport against it, so viewport and ballooned PDF agree.

To check exports in `tauri dev` without the save dialog, set `DIMO_DEV_EXPORT_DIR=<dir>` (debug
builds only) and click the export buttons, by hand or from a `DIMO_DEV_UI_SCRIPT`
(steps `show`, `select`, `check`, `button`, see `src/lib/dev/ui-script.ts`).

## Tauri capabilities

Grant the webview only the commands it uses. No shell plugin, no fs plugin, no http plugin.
File dialogs go through Rust commands that return paths the core then opens.
