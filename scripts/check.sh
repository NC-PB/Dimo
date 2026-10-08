#!/usr/bin/env bash
# Single entry point for local checks and CI. Skips parts that do not exist yet.
# Usage: ./scripts/check.sh [--fast]   (--fast skips cargo deny and frontend tests)
set -euo pipefail
cd "$(dirname "$0")/.."

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

step() { printf '\n==> %s\n' "$*"; }

if [[ -f Cargo.toml ]]; then
  step "cargo fmt"
  cargo fmt --all -- --check
  step "cargo clippy"
  cargo clippy --workspace --all-targets -- -D warnings
  step "cargo test"
  cargo test --workspace
  if [[ $FAST -eq 0 ]]; then
    if command -v cargo-deny >/dev/null 2>&1; then
      step "cargo deny"
      cargo deny check
    else
      echo "cargo-deny not installed, skipping (install: cargo install --locked cargo-deny)" >&2
      [[ "${CI:-}" == "true" ]] && exit 1
    fi
  fi
else
  echo "No Cargo.toml yet, skipping Rust checks"
fi

if [[ -f apps/desktop/package.json ]]; then
  step "frontend check"
  pnpm -C apps/desktop check
  step "frontend lint"
  pnpm -C apps/desktop lint
  if [[ $FAST -eq 0 ]]; then
    step "frontend test"
    pnpm -C apps/desktop test
  fi
else
  echo "No frontend yet, skipping frontend checks"
fi

step "all checks passed"
