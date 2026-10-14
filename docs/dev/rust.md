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

## Checklist: adding a command

A new variant of `dimo_core::Command`. The compiler catches the items marked (exhaustive); the
others are hand kept and easy to miss.

1. Variant with doc comment and requirement IDs: `crates/dimo-core/src/command/mod.rs`
   `Command`. New payload types derive `Serialize`, `Deserialize`, `JsonSchema` and
   `specta::Type` (behind the `specta` feature) and are re-exported in
   `crates/dimo-core/src/lib.rs`.
2. Dispatch arm in `run` in `command/mod.rs` (exhaustive). Keep the arm one call.
3. Handler in the area module under `crates/dimo-core/src/command/` (characteristics, balloons,
   sheets, numbering, settings, measurement, reports). It validates first, then pushes
   `patch::Change`s through `Tx::push`; record a change only if the state differs. Use the
   shared helpers (`known_chars` in `mod.rs`, `change_settings` in `settings.rs`,
   `renumber_if_unlocked` in `numbering.rs`).
4. New refusal: a `CommandError` variant in `command/mod.rs`, its `RejectReason` and mapping in
   `apps/desktop/src-tauri/src/ipc.rs` (`impl From<&dimo_core::CommandError> for RejectReason`,
   exhaustive) with the mapping test in the same file.
5. Frontend text for a new reason: `rejectReasonText` in `apps/desktop/src/lib/i18n.ts`
   (exhaustive), `refusal_<reason>` keys in `apps/desktop/messages/en.json` and `de.json`, the
   `RejectReason` list in `apps/desktop/src/lib/i18n.test.ts`, and `refusalDetail` in
   `apps/desktop/src/lib/table/refusal.ts` if it needs details.
6. Shell: nothing for a new variant. All commands go through the one Tauri command `execute`
   (`apps/desktop/src-tauri/src/project.rs`, `session.rs`). Only a new Tauri command needs the
   function, its entry in the command list (`build.rs`, `lib.rs` `collect_commands!`) and the
   permission in `capabilities/default.json` (see the `ipc-command` skill).
7. Bindings: `cargo test -p dimo-desktop --test bindings` rewrites
   `apps/desktop/src/lib/ipc/bindings.ts` and fails once; run it again and commit the file.
8. History: `source` in `crates/dimo-core/src/history.rs` (not exhaustive) if the command names
   characteristics directly; otherwise its changes count as rule driven. `missing_table` in
   `crates/dimo-io/src/project/session.rs` if the command can name a custom table.
9. New primitive change, if no existing `Change` fits: the variant in
   `crates/dimo-core/src/patch/mod.rs`, an entry in the `inverse_of!` lists in `Change::inverse`
   (exhaustive), a dispatch arm in `Project::apply_change` (exhaustive) with an applier in the
   area module under `patch/`, `applyChange` in `apps/desktop/src/lib/stores/patch.ts`
   (exhaustive) with `patch.test.ts`, and the change type string lists in
   `crates/dimo-io/src/project/migrate.rs` if it embeds a characteristic, sheet, numbering or
   settings state.
10. Frontend call: build the command where it is used, through `ProjectStore.execute` or
    `executeWith` (`apps/desktop/src/lib/stores/project.svelte.ts`).
11. Tests: a case in `crates/dimo-core/tests/commands.rs` (or the area test file), the `Op`
    mirror and `command()` in `crates/dimo-core/tests/undo_redo.rs` (apply then undo returns
    the original state), and the scripted history in `crates/dimo-io/tests/common/mod.rs` so
    audit, container and recovery tests cover it.

## Checklist: adding a persisted field

A new field on `Characteristic`, `Project` or one of its parts (`ProjectInfo`,
`ProjectSettings`, `Numbering`, `ReportInfo`, `Sample`).

1. Struct field with doc comment: `crates/dimo-core/src/characteristic.rs` or `project.rs`
   (and its constructor, `Characteristic::manual`, `Project::new`). Every key is written; use
   `null` for absent values, no `serde(default)`.
2. Editing: for characteristics a `FieldValue` arm and its case in `Characteristic::set_values`
   (exhaustive); for project parts the existing replace command (`UpdateProjectInfo`,
   `SetToleranceSettings`, ...) carries it. If recognition fills it: `Proposal` in
   `crates/dimo-core/src/proposal.rs` and `accept_one` in `command/characteristics.rs`.
3. History: `CharacteristicField` and `changed_fields` in `crates/dimo-core/src/history.rs`
   (hand kept, a missing field silently drops out of the history), and `fieldLabel` in
   `apps/desktop/src/lib/i18n.ts`.
4. Schema version: `SCHEMA_VERSION` in `crates/dimo-io/src/project/mod.rs`; update tests that
   assert it (`grep -rn SCHEMA_VERSION crates/dimo-io/tests`).
5. Migration: a `MIGRATIONS` entry and a `mod vN` in `crates/dimo-io/src/project/migrate.rs`
   with `project` and `audit_entry` functions. The audit migration must also patch the states
   inside stored changes (`characteristic_inserted`, `_removed`, `_changed`, and every other
   change type that embeds the changed struct). Unit tests at the end of the file.
6. Fixtures: never edit old ones under `crates/dimo-io/tests/fixtures/v*/`. Write the new one
   with `DIMO_WRITE_FIXTURE=1 cargo test -p dimo-io --test migrations`, add an `edit_vN` in
   `crates/dimo-io/tests/common/mod.rs`, and review the `migrations__*` and `container__*`
   snapshots with `cargo insta review`.
7. Test builders: `crates/dimo-core/tests/common/mod.rs`, `Value` and `field_values` in
   `crates/dimo-core/tests/undo_redo.rs`, `crates/dimo-report/tests/common/fixture.rs`, and
   the TypeScript literals in `apps/desktop/src/lib/stores/fixtures.ts`.
8. JSON Schema: `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-io --test schema` rewrites
   `docs/schema/project.schema.json`; update `docs/spec/07-data-model.md`.
9. Exports, if the field is a list column: `Column`, `COLUMNS` (fixed size array) and
   `Column::header` in `crates/dimo-io/src/export/columns.rs`, then the `export_list__*`
   snapshots.
10. Report, if it is printed: `Row` and `ReportModel::build` in `crates/dimo-report/src/model.rs`,
    `docs/schema/report-data.schema.json` (checked in `crates/dimo-report/tests/report.rs`), the
    templates in `data/templates/dimo-default/` and `crates/dimo-report/src/xlsx.rs`.
11. Frontend: regenerate `bindings.ts` (see the command checklist), then table columns
    (`apps/desktop/src/lib/table/columns.ts`), the detail editor
    (`apps/desktop/src/lib/components/CharacteristicDetail.svelte`) or the project dialogs, and
    `col_*` and `field_*` keys in `apps/desktop/messages/en.json` and `de.json`.
12. Truth files only if the field is ground truth: `TruthCharacteristic` in
    `crates/dimo-core/src/truth.rs` and `docs/schema/truth.schema.json`.
