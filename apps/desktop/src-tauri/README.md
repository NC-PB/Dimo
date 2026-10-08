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

## Tile protocol

The custom scheme `dimo` serves sheet tiles at `tile/{doc}/{sheet}/{zoom}/{x}/{y}` (`src/tiles.rs`,
logic in `dimo_pdf::tiles`). `doc` is the content hash returned by `open_document`, zoom levels are
powers of two (`2^zoom` pixels per sheet unit), tiles are 512 px PNG images. The URL differs per
platform: `dimo://localhost/tile/...` on macOS and Linux, `http://dimo.localhost/tile/...` on
Windows. Build URLs with `tileUrl` from `src/lib/viewport/tiles.ts`, never by hand. The CSP allows
both forms in `img-src`.

The handler never blocks: it queues the request with the tile service and a worker answers. Call
`set_tile_interest` with the visible tile ranges (plus a margin) whenever the view changes; queued
requests outside them are answered with `204 No Content`. Other statuses: `400` bad route, `404`
unknown document or tile outside the sheet, `503` PDFium missing. Rendered tiles are cached in
memory (128 MiB LRU) and in `<user cache dir>/io.github.nc-pb.dimo/tiles/`, keyed by content hash.
