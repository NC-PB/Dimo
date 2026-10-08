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
logic in `dimo_pdf::tiles`). `doc` is the content hash returned by `open_document_dialog`, zoom levels are
powers of two (`2^zoom` pixels per sheet unit), tiles are 512 px PNG images. The URL differs per
platform: `dimo://localhost/tile/...` on macOS and Linux, `http://dimo.localhost/tile/...` on
Windows. Build URLs with `tileUrl` from `src/lib/viewport/tiles.ts`, never by hand. The CSP allows
both forms in `img-src`.

The handler never blocks: it queues the request with the tile service and a worker answers. Call
`set_tile_interest` with the visible tile ranges (plus a margin) whenever the view changes; queued
requests outside them are answered with `204 No Content`. Other statuses: `400` bad route, `404`
unknown document or tile outside the sheet, `503` PDFium missing. Rendered tiles are cached in
memory (128 MiB LRU) and in `<user cache dir>/io.github.nc-pb.dimo/tiles/`, keyed by content hash.

## Opening drawings

`open_document_dialog` shows the native file dialog from Rust (`rfd`, no dialog or fs plugin and
no extra capability) and opens the chosen PDF. The webview never passes a path (NFR-SEC-01).

## Development helpers

Debug builds read these environment variables once at startup through `dev_startup`
(`src/dev.rs`); release builds ignore them:

| Variable                          | Effect                                                     |
| --------------------------------- | ---------------------------------------------------------- |
| `DIMO_DEV_OPEN=<path>`            | open this PDF at startup                                   |
| `DIMO_DEV_SHEET=<n>`              | show zero based sheet `n` first                            |
| `DIMO_DEV_BALLOONS=<n>`           | place `n` dummy balloons per sheet                         |
| `DIMO_DEV_ANCHORS=<x,y;x,y;...>`  | dummy balloon leaders end at these sheet points            |
| `DIMO_DEV_VIEW=<percent>@<x>,<y>` | start at this zoom with the sheet point centered           |
| `DIMO_DEV_PAN_CHECK=1`            | run the scripted pan and print frame times to the terminal |

The side panel of a dev build also has a dummy balloon toggle and a "Measure pan" button.
