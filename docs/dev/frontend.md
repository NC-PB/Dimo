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

## Tauri capabilities

Grant the webview only the commands it uses. No shell plugin, no fs plugin, no http plugin.
File dialogs go through Rust commands that return paths the core then opens.
