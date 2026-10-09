#!/usr/bin/env bash
# D-43 release gate: a release must not ship a tolerance table that is still a draft.
# Usage: ./scripts/check-table-status.sh [dir]   (default: data/tolerances)
#
# Release mode is on with DIMO_RELEASE=1 or on a tag build in CI (GITHUB_REF_TYPE=tag).
# In release mode any draft table fails the check. Otherwise drafts only print a warning.
# A table file without a status in its [table] header always fails.
# Tested by scripts/test-check-table-status.sh with the fixtures in scripts/fixtures/table-status.
set -euo pipefail

dir="${1:-$(dirname "$0")/../data/tolerances}"
release=0
if [[ "${DIMO_RELEASE:-}" == "1" || "${GITHUB_REF_TYPE:-}" == "tag" ]]; then
  release=1
fi

shopt -s nullglob
drafts=()
count=0
for file in "$dir"/*.toml; do
  [[ "$file" == *.test.toml ]] && continue
  count=$((count + 1))
  name=$(basename "$file")
  # The status key of the [table] section. Tolerates CRLF line ends.
  status=$(awk '
    /^\[table\][[:space:]]*$/ { in_table = 1; next }
    /^\[/ { in_table = 0 }
    in_table && /^[[:space:]]*status[[:space:]]*=/ {
      value = $0
      sub(/^[^=]*=[[:space:]]*"/, "", value)
      sub(/".*$/, "", value)
      print value
      exit
    }' "$file")
  case "$status" in
    draft) drafts+=("$name") ;;
    verified) ;;
    *)
      echo "ERROR: $name has no valid status in its [table] header" >&2
      exit 1
      ;;
  esac
done

if [[ $count -eq 0 ]]; then
  echo "ERROR: no tolerance tables in $dir" >&2
  exit 1
fi
if [[ ${#drafts[@]} -eq 0 ]]; then
  echo "tolerance tables: all $count verified"
  exit 0
fi
if [[ $release -eq 1 ]]; then
  echo "ERROR (D-43): release blocked, these tolerance tables are drafts: ${drafts[*]}" >&2
  echo "Only the owner verifies a table against the printed source." >&2
  exit 1
fi
echo "WARNING (D-43): draft tolerance tables, a release build would fail: ${drafts[*]}" >&2
