# Frontend conventions

Location: `apps/desktop/src`. Svelte 5 with runes, TypeScript strict, Vite, no SSR, no router (D-50).

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
- Text: every user visible string goes through Paraglide messages, English and German (FR-SET-04).
- Shortcuts: defined in one file, shown in the `?` cheat sheet (D-52). Every action reachable by keyboard.
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
typed text as `update_fields`; decimals are never parsed in TypeScript, Rust's refusal is shown
in the table's status line. Reordering sends one `move_characteristics`.

To check scrolling with a large list in `tauri dev`:

```sh
node scripts/dev-characteristics.mjs 1000 target/dev/chars-1000.json
DIMO_DEV_OPEN=$PWD/corpus/drawings/test_drawing_1.pdf DIMO_DEV_SCRIPT=$PWD/target/dev/chars-1000.json \
  VITE_DIMO_DEV_TABLE_CHECK=1 pnpm -C apps/desktop tauri dev
```

The frame times of a scripted scroll are printed to the terminal ("0 balloons", the viewport size
is the table's) and shown in the developer tools.

## Tauri capabilities

Grant the webview only the commands it uses. No shell plugin, no fs plugin, no http plugin.
File dialogs go through Rust commands that return paths the core then opens.
