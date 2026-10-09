#!/usr/bin/env bash
# Tests the D-43 release gate scripts/check-table-status.sh with fixtures. Run by check.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

gate=scripts/check-table-status.sh
fixtures=scripts/fixtures/table-status
failures=0

# expect <exit code> <description> <env assignments...> -- <dir>
expect() {
  local want=$1 what=$2
  shift 2
  local vars=()
  while [[ $1 != "--" ]]; do
    vars+=("$1")
    shift
  done
  shift
  local got=0
  env -u DIMO_RELEASE -u GITHUB_REF_TYPE ${vars[@]+"${vars[@]}"} "$gate" "$1" >/dev/null 2>&1 || got=$?
  if [[ $got -ne $want ]]; then
    echo "FAIL: $what: exit $got, expected $want" >&2
    failures=$((failures + 1))
  fi
}

expect 0 "draft table in a normal build only warns" -- "$fixtures/draft"
expect 1 "draft table with DIMO_RELEASE=1 fails" DIMO_RELEASE=1 -- "$fixtures/draft"
expect 1 "draft table on a tag build fails" GITHUB_REF_TYPE=tag -- "$fixtures/draft"
expect 0 "draft table on a branch build only warns" GITHUB_REF_TYPE=branch -- "$fixtures/draft"
expect 0 "no draft table with DIMO_RELEASE=1 passes" DIMO_RELEASE=1 -- "$fixtures/verified"
expect 0 "no draft table on a tag build passes" GITHUB_REF_TYPE=tag -- "$fixtures/verified"
expect 1 "table without status fails" -- "$fixtures/no-status"
expect 1 "folder without tables fails" -- "$fixtures/empty"

if [[ $failures -ne 0 ]]; then
  echo "release gate tests: $failures failed" >&2
  exit 1
fi
echo "release gate tests passed"
