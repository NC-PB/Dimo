#!/usr/bin/env bash
# Checks that the app version is the same in Cargo.toml, tauri.conf.json and package.json, and,
# when a tag is given, that the tag names that version (R.13 release gate). A pre-release tag such
# as v0.1.0-rc.1 names version 0.1.0.
#
# Usage: ./scripts/check-version.sh [tag]   for example: ./scripts/check-version.sh v0.1.0
# Prints the version on success.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
tauri=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' apps/desktop/src-tauri/tauri.conf.json)
npm=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' apps/desktop/package.json)

if [[ -z "$cargo" || "$cargo" != "$tauri" || "$cargo" != "$npm" ]]; then
  echo "version mismatch: Cargo.toml '$cargo', tauri.conf.json '$tauri', package.json '$npm'" >&2
  exit 1
fi
if [[ $# -ge 1 && "$1" != "v$cargo" && "$1" != "v$cargo"-* ]]; then
  echo "tag '$1' does not match the app version $cargo (expected v$cargo)" >&2
  exit 1
fi
echo "$cargo"
