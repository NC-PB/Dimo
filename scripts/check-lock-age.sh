#!/usr/bin/env bash
# Lists crates in Cargo.lock that were published less than N days ago (AGENTS.md rule 10).
# Needs network (crates.io API), so it is not part of check.sh. Usage: ./scripts/check-lock-age.sh [days]
set -euo pipefail
cd "$(dirname "$0")/.."
DAYS="${1:-14}"
cutoff=$(date -u -v-"${DAYS}"d +%Y-%m-%d 2>/dev/null || date -u -d "-${DAYS} days" +%Y-%m-%d)
status=0
while read -r name version; do
  created=$(curl -fsS -A "dimo-lock-age-check (github.com/NC-PB/dimo)" \
    "https://crates.io/api/v1/crates/${name}/${version}" | jq -r '.version.created_at[0:10]')
  if [[ "$created" > "$cutoff" ]]; then
    echo "TOO NEW: ${name} ${version} published ${created}"
    status=1
  fi
  sleep 1 # crates.io crawler policy: at most one request per second
done < <(awk '/^name = /{n=$3} /^version = /{v=$3} /^source = "registry/{gsub(/"/,"",n); gsub(/"/,"",v); print n, v}' Cargo.lock)
[[ $status -eq 0 ]] && echo "all crates are at least ${DAYS} days old (cutoff ${cutoff})"
exit $status
