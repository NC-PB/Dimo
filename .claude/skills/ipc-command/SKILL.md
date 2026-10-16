---
name: ipc-command
description: Add a Tauri command, event, channel or custom protocol route end to end in DimoMark (core command, src-tauri wrapper, specta bindings, capability, frontend call). Use whenever the frontend needs new data or a new action.
---

# IPC command end to end

Principles: Rust owns the document (ADR 0001), types are generated (NFR-MNT-03), the webview gets
only the permissions it needs (NFR-SEC-01).

## Document changing action

1. `dimo-core`: add a variant to `Command` with its data. Implement apply, inverse (undo), audit
   entry, and the resulting patch. Unit test: apply then undo restores the state exactly.
2. `src-tauri`: the generic `execute_command` path should already handle it. Only add a dedicated
   Tauri command if the action is not a document command (file dialogs, settings, jobs).
3. Regenerate bindings (debug build or the bindings test), commit `bindings.ts`.
4. Frontend: dispatch through the generated function, apply the returned patch in the store.
   Never mutate domain data in the store before the patch arrives, except optimistic gesture state.

## Query or non document command

1. Implement the logic in the right crate, not in `src-tauri`.
2. `src-tauri`: thin `#[tauri::command]` + `#[specta::specta]` wrapper, error type mapped to a
   serializable error enum. Register it in `collect_commands!` in `src/lib.rs`.
3. Add its name to `apps/desktop/src-tauri/commands.txt` (one name per line, under its module
   comment). `build.rs` turns that table into the permission set `allow-app-commands`, which
   `capabilities/default.json` already references. Do not touch the capability file. The test
   `cargo test -p dimo-desktop --test command_table` fails if the table and `collect_commands!`
   differ in either direction.
4. Regenerate bindings with `cargo test -p dimo-desktop --test bindings` (run twice: the first run
   rewrites `bindings.ts` and fails), use them in the frontend. On a `bindings.ts` merge conflict
   take either side and regenerate. Never format the file.

## Long running work

Use the job runner: job ID, `JobProgress` events, cancellation token, results via channel.
Jobs read an immutable snapshot and return proposals or files.

## Binary data

Tiles and image crops go through the `dimo://` protocol, never through commands as base64.

## Checklist

- [ ] No business rule added in TypeScript
- [ ] Bindings regenerated and committed
- [ ] Name added to `commands.txt` (no capability edit needed)
- [ ] Rust test for the logic, frontend test if there is view logic
