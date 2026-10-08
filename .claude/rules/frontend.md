---
paths:
  - "apps/desktop/src/**"
  - "apps/desktop/*.json"
  - "apps/desktop/*.ts"
  - "apps/desktop/*.js"
---

# Frontend files

Full conventions: `docs/dev/frontend.md`. Most important:

- No business logic in TypeScript. Limits, numbering, pass/fail come from Rust.
- IPC only through generated bindings in `src/lib/ipc/`.
- Every visible string through Paraglide (`en`, `de`).
- Status and pass/fail use shape plus color.
- Svelte 5 runes, no legacy `$:` or stores API for new code.
