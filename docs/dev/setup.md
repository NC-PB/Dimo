# Development setup

## Required tools

| Tool | Version | Install |
|---|---|---|
| Rust | pinned in `rust-toolchain.toml` | `rustup` picks it up automatically |
| Node.js | 22 LTS | nodejs.org or a version manager |
| pnpm | pinned in root `package.json` (`packageManager`) | `corepack enable` |
| Tauri CLI | 2.x, pinned in `apps/desktop/package.json` | installed by `pnpm install` |
| cargo-deny | latest | `cargo install --locked cargo-deny` |
| cargo-insta | latest | `cargo install --locked cargo-insta` |
| PDFium binaries | pinned in `scripts/pdfium.toml` | `./scripts/fetch-pdfium.sh` (task T0.5) |

Platform prerequisites for Tauri 2: see the official Tauri prerequisites page
(Xcode command line tools on macOS, WebView2 and MSVC build tools on Windows, WebKitGTK on Linux).

## First run

```sh
corepack enable
pnpm install
./scripts/fetch-pdfium.sh
./scripts/check.sh
pnpm -C apps/desktop tauri dev
```

## Folders that are not committed

| Path | Content |
|---|---|
| `target/` | Rust build output |
| `node_modules/` | npm packages |
| `vendor/pdfium/` | downloaded PDFium binaries |
| `data/models/*.onnx` | downloaded model files, referenced by checksum in `data/models/manifest.toml` |
| `spikes/*/target` | spike build output |
