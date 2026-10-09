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
  differ from domain types. `dimo-core` domain types are sent as they are: they derive
  `specta::Type` behind the crate feature `specta`, which only `src-tauri` enables. Use one shape
  for both directions (no `skip_serializing_if` or `serde(default)` on IPC types; optional values
  are `null`), so `specta_serde::Format` exports them without separate serialize and deserialize
  types (`dimo-core/tests/ipc_types.rs`).
- Public items get a doc comment. Reference requirement IDs where they define behavior.
- `unsafe` is not allowed except in a clearly isolated FFI module with a safety comment.
- Long CPU work: rayon, cancellable via a token checked between units of work.
  Never block the tokio runtime or the Tauri main thread.

## Domain specifics

- Commands mutate the document in `dimo-core`. Every command has an inverse for undo and
  produces an audit entry and a patch. Test apply then undo returns the original state.
  Mechanics: `command::execute` compiles a `Command` into primitive `patch::Change`s with
  before and after state; undo applies the inverted changes in reverse order, redo the same
  changes again; `document::Document` keeps the history and the audit entries.
- `dimo-core` has no IO, clock or randomness. New IDs, the time and the audit user come from an
  `env::Environment` passed into each command (`FixedEnvironment` in tests), rule 11.
- Parse errors in `dimo-notation` are values with a position, not panics.
- Tolerance derivations always carry a machine readable rule and a human readable explanation.
