# dimo-desktop

Tauri shell of the Dimo desktop app. It wires commands, events, custom protocols and capabilities
to the `dimo-*` crates and holds no business logic (ADR 0001). The frontend lives in `../src`.

## IPC

Commands and events are registered in `specta_builder()` (`src/lib.rs`), their types live in
`src/ipc.rs`. tauri-specta writes the TypeScript bindings to `../src/lib/ipc/bindings.ts` on every
debug start and in `cargo test -p dimo-desktop --test bindings`. That test fails when the committed
file is out of date and rewrites it, so review the diff and commit it with the Rust change.
The file is excluded from eslint and prettier because it must stay byte identical to the generator
output.

Every app command is declared in `build.rs` (`AppManifest::commands`) and must be allowed by name in
`capabilities/default.json` (`allow-<command>`), otherwise the webview cannot call it.
