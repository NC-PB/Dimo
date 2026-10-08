#!/usr/bin/env bash
# DCO check: every commit in a range has a Signed-off-by trailer (CONTRIBUTING.md).
# Usage: ./scripts/check-dco.sh <base-ref> [head-ref]   e.g. ./scripts/check-dco.sh origin/main
# Merge commits are skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

base="${1:?usage: check-dco.sh <base-ref> [head-ref]}"
head="${2:-HEAD}"
status=0
count=0
while read -r sha; do
  count=$((count + 1))
  if ! git show -s --format=%B "$sha" | grep -Eq '^Signed-off-by: .+ <.+>[[:space:]]*$'; then
    echo "missing Signed-off-by: $(git show -s --format='%h %s' "$sha")"
    status=1
  fi
done < <(git rev-list --no-merges "${base}..${head}")

if [[ $status -ne 0 ]]; then
  echo "Fix with: git rebase --signoff ${base}  (then force push your branch)" >&2
else
  echo "DCO ok (${count} commits)"
fi
exit $status
