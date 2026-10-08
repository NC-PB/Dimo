---
name: ipc-command
description: Add a Tauri command, event, channel or custom protocol route end to end in Dimo (core command, src-tauri wrapper, specta bindings, capability, frontend call). Use whenever the frontend needs new data or a new action.
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
   serializable error enum. Register in the builder and in the specta collector.
3. Allow it in the capability file of the main window. Nothing more.
4. Regenerate bindings, use them in the frontend.

## Long running work

Use the job runner: job ID, `JobProgress` events, cancellation token, results via channel.
Jobs read an immutable snapshot and return proposals or files.

## Binary data

Tiles and image crops go through the `dimo://` protocol, never through commands as base64.

## Checklist

- [ ] No business rule added in TypeScript
- [ ] Bindings regenerated and committed
- [ ] Capability updated minimally
- [ ] Rust test for the logic, frontend test if there is view logic
