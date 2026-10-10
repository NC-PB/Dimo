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
  # T2.9: the M2 exit criterion is also a test in dimo-cli; this prints the per category
  # report. Needs PDFium, so it runs where PDFium is required (CI, DIMO_REQUIRE_PDFIUM=1).
  if [[ "${CI:-}" == "true" || ( -n "${DIMO_REQUIRE_PDFIUM:-}" && "${DIMO_REQUIRE_PDFIUM}" != "0" ) ]]; then
    step "box select evaluation (M2 exit criterion)"
    cargo run -q -p dimo-cli -- eval box-select
  fi
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

if [[ -d data/tolerances ]]; then
  # D-43: fails with DIMO_RELEASE=1 or on a tag build while a shipped table is a draft,
  # only warns otherwise.
  step "tolerance tables release gate"
  ./scripts/test-check-table-status.sh
  ./scripts/check-table-status.sh
fi

if [[ -f apps/desktop/package.json ]]; then
  step "frontend check"
  pnpm -C apps/desktop check
  step "frontend lint"
  pnpm -C apps/desktop lint
  step "npm licenses"
  node scripts/check-npm-licenses.mjs
  if [[ $FAST -eq 0 ]]; then
    step "frontend test"
    pnpm -C apps/desktop test
  fi
else
  echo "No frontend yet, skipping frontend checks"
fi

step "all checks passed"
