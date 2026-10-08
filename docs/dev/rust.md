# Rust conventions

## Workspace

- One Cargo workspace at the root. Crates in `crates/`, Tauri shell in `apps/desktop/src-tauri`.
- Edition 2024, resolver 3. Shared dependency versions in `[workspace.dependencies]`,
  crates use `dep.workspace = true`. Shared lints in `[workspace.lints]`.
- Crate dependency direction (no cycles, no shortcuts):
  `dimo-core` has no IO and depends on no other `dimo-*` crate except shared types.
  `dimo-notation` and `dimo-tolerance` are pure. `dimo-pdf`, `dimo-vision` do IO and rendering.
  `dimo-detect` uses pdf, vision, notation, tolerance. `dimo-io` and `dimo-report` depend on core.
  Only `src-tauri` and `dimo-cli` may depend on everything.
- No crate except `src-tauri` depends on `tauri`.

## Code

- Errors: `thiserror` enums in library crates, `anyhow` only in binaries (`dimo-cli`, `src-tauri`).
  No `unwrap()` or `expect()` outside tests, except for invariants with a comment explaining why.
- Logging: `tracing`, never `println!` in library code.
- Numbers: `rust_decimal::Decimal` for all tolerance values. Geometry in `f64` sheet units.
- IDs: newtype wrappers around `uuid::Uuid` (`CharId`, `BalloonId`, `SheetId`). Never pass raw `Uuid`.
- Serialization: `serde` with `#[serde(rename_all = "snake_case")]`, `schemars` on everything
  that ends up in `project.json`.
- IPC types: derive `specta::Type`. Keep IPC DTOs in one module per crate (`ipc.rs`) when they
  differ from domain types.
- Public items get a doc comment. Reference requirement IDs where they define behavior.
- `unsafe` is not allowed except in a clearly isolated FFI module with a safety comment.
- Long CPU work: rayon, cancellable via a token checked between units of work.
  Never block the tokio runtime or the Tauri main thread.

## Domain specifics

- Commands mutate the document in `dimo-core`. Every command has an inverse for undo and
  produces an audit entry and a patch. Test apply then undo returns the original state.
- Parse errors in `dimo-notation` are values with a position, not panics.
- Tolerance derivations always carry a machine readable rule and a human readable explanation.
