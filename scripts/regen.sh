#!/usr/bin/env bash
# Regenerates every committed generated file in one go, then shows what changed:
# the JSON schemas in docs/schema/, the TypeScript bindings, the viewport balloon layout fixture
# and THIRD_PARTY_LICENSES. Run it after changing doc comments or fields of IPC, project, report,
# truth or tolerance table types, or after a dependency change; review the diff and commit it with
# the change. Golden files and migration fixtures are not regenerated here on purpose (their tests
# print their own command).
# Usage: ./scripts/regen.sh
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$*"; }

step "JSON schemas"
for target in "dimo-core corpus_truth" "dimo-io schema" "dimo-report report" "dimo-tolerance schema"; do
  read -r crate test <<<"$target"
  DIMO_UPDATE_SCHEMA=1 cargo test -q -p "$crate" --test "$test" committed_schema_is_current
done

step "TypeScript bindings and balloon layout fixture"
# These tests rewrite their file when it is out of date and fail once; the second run passes.
for test in bindings balloon_layout; do
  cargo test -q -p dimo-desktop --test "$test" >/dev/null 2>&1 \
    || cargo test -q -p dimo-desktop --test "$test"
done

step "third party license list"
if compgen -G "vendor/pdfium/*/licenses" >/dev/null; then
  node scripts/third-party-licenses.mjs
else
  echo "PDFium not fetched, skipping (run ./scripts/fetch-pdfium.sh)" >&2
fi

step "changed generated files"
git status --short -- docs/schema apps/desktop/src/lib/ipc apps/desktop/src/lib/viewport THIRD_PARTY_LICENSES
