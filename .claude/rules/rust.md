---
paths:
  - "crates/**/*.rs"
  - "apps/desktop/src-tauri/**/*.rs"
  - "tools/**/*.rs"
  - "**/Cargo.toml"
---

# Rust files

Full conventions: `docs/dev/rust.md`. Most important:

- Tolerance values are `rust_decimal::Decimal`, never floats.
- Library crates: `thiserror`, no `unwrap`/`expect` outside tests, `tracing` instead of `println!`.
- Only `src-tauri` depends on `tauri`. Respect the crate dependency direction.
- Document mutations are `Command`s in `dimo-core` with an inverse and an audit entry.
- New dependencies: exact version, permissive license, release at least 14 days old, added to
  `[workspace.dependencies]`.
