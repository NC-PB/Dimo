#!/usr/bin/env bash
# Downloads the pinned PDFium binaries (scripts/pdfium.toml) into vendor/pdfium/<target>/
# and verifies their SHA-256. Idempotent: a target whose checksum stamp matches is skipped.
#
# Usage: ./scripts/fetch-pdfium.sh [--all | <target>...]
#   no argument   the host target (for example mac-arm64)
#   --all         every target in pdfium.toml (for packaging)
#   <target>      one or more targets by name, for example mac-x64 linux-arm64
set -euo pipefail
cd "$(dirname "$0")/.."

CONFIG=scripts/pdfium.toml
VENDOR=vendor/pdfium

# Prints the value of `key` in section `section` ("" for the top level) of pdfium.toml.
toml_get() {
  local section="$1" key="$2"
  awk -v want="$section" -v key="$key" '
    /^[[:space:]]*#/ { next }
    /^\[/ { gsub(/^\[|\][[:space:]]*$/, ""); current = $0; next }
    current == want {
      line = $0
      if (match(line, "^[[:space:]]*" key "[[:space:]]*=[[:space:]]*\"")) {
        value = substr(line, RLENGTH + 1)
        sub(/".*$/, "", value)
        print value
        exit
      }
    }' "$CONFIG"
}

all_targets() {
  sed -n 's/^\[targets\.\(.*\)\][[:space:]]*$/\1/p' "$CONFIG"
}

host_target() {
  local os arch
  case "$(uname -s)" in
    Darwin) os=mac ;;
    Linux) os=linux ;;
    MINGW* | MSYS* | CYGWIN*) os=win ;;
    *) echo "unsupported OS: $(uname -s)" >&2; return 1 ;;
  esac
  case "$(uname -m)" in
    arm64 | aarch64) arch=arm64 ;;
    x86_64 | amd64) arch=x64 ;;
    *) echo "unsupported architecture: $(uname -m)" >&2; return 1 ;;
  esac
  echo "${os}-${arch}"
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

fetch() {
  local target="$1" section="targets.$1"
  local archive sha library base dest stamp
  archive=$(toml_get "$section" archive)
  sha=$(toml_get "$section" sha256)
  library=$(toml_get "$section" library)
  base=$(toml_get "" base_url)
  if [[ -z "$archive" || -z "$sha" || -z "$library" ]]; then
    echo "unknown target '$target' (known: $(all_targets | tr '\n' ' '))" >&2
    return 1
  fi
  dest="$VENDOR/$target"
  stamp="$dest/.sha256"
  if [[ -f "$stamp" && "$(cat "$stamp")" == "$sha" && -f "$dest/$library" ]]; then
    echo "pdfium $target: up to date ($dest)"
    return 0
  fi

  mkdir -p "$VENDOR"
  local tmp
  tmp=$(mktemp -d "$VENDOR/.fetch-$target.XXXXXX")
  # shellcheck disable=SC2064 # expand now: tmp is local
  trap "rm -rf '$tmp'" RETURN

  echo "pdfium $target: downloading $archive"
  curl -fsSL --retry 3 -o "$tmp/$archive" "$base/$archive"
  local actual
  actual=$(sha256_of "$tmp/$archive")
  if [[ "$actual" != "$sha" ]]; then
    echo "pdfium $target: checksum mismatch for $archive" >&2
    echo "  expected $sha" >&2
    echo "  actual   $actual" >&2
    return 1
  fi
  mkdir "$tmp/out"
  tar -xzf "$tmp/$archive" -C "$tmp/out"
  if [[ ! -f "$tmp/out/$library" ]]; then
    echo "pdfium $target: $library missing in $archive" >&2
    return 1
  fi
  echo "$sha" >"$tmp/out/.sha256"
  rm -rf "$dest"
  mv "$tmp/out" "$dest"
  echo "pdfium $target: installed $dest/$library"
}

targets=()
if [[ $# -eq 0 ]]; then
  targets=("$(host_target)")
elif [[ "$1" == "--all" ]]; then
  while IFS= read -r t; do targets+=("$t"); done < <(all_targets)
else
  targets=("$@")
fi

for t in "${targets[@]}"; do
  fetch "$t"
done
