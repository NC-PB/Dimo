#!/usr/bin/env bash
# Stages the pinned PDFium library and its licenses for an installer build (R.13, ADR 0005).
#
# Writes apps/desktop/src-tauri/bundle-staging/ (git ignored), which the release configs
# tauri.release.<os>.json bundle:
#   pdfium/<library>              the library (libpdfium.dylib, pdfium.dll or libpdfium.so)
#   licenses/pdfium/LICENSE       the PDFium license
#   licenses/pdfium/third_party/  the licenses of the code bundled in PDFium
#   licenses/pdfium/VERSION       the PDFium build
#
# Usage: ./scripts/stage-pdfium.sh <target>
#   <target>        a target of scripts/pdfium.toml, for example win-x64 or linux-x64
#   mac-universal   mac-arm64 and mac-x64 joined into one library with lipo (macOS only)
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <target|mac-universal>" >&2
  exit 2
fi
target="$1"
STAGE=apps/desktop/src-tauri/bundle-staging

library_of() {
  case "$1" in
    win-*) echo "bin/pdfium.dll" ;;
    mac-*) echo "lib/libpdfium.dylib" ;;
    *) echo "lib/libpdfium.so" ;;
  esac
}

rm -rf "$STAGE"
mkdir -p "$STAGE/pdfium" "$STAGE/licenses/pdfium"

if [[ "$target" == "mac-universal" ]]; then
  ./scripts/fetch-pdfium.sh mac-arm64 mac-x64
  lipo -create \
    vendor/pdfium/mac-arm64/lib/libpdfium.dylib \
    vendor/pdfium/mac-x64/lib/libpdfium.dylib \
    -output "$STAGE/pdfium/libpdfium.dylib"
  lipo -info "$STAGE/pdfium/libpdfium.dylib"
  licenses_from=vendor/pdfium/mac-arm64
else
  ./scripts/fetch-pdfium.sh "$target"
  library=$(library_of "$target")
  cp "vendor/pdfium/$target/$library" "$STAGE/pdfium/"
  licenses_from="vendor/pdfium/$target"
fi

cp "$licenses_from/LICENSE" "$licenses_from/VERSION" "$STAGE/licenses/pdfium/"
cp -R "$licenses_from/licenses" "$STAGE/licenses/pdfium/third_party"
echo "staged PDFium for $target in $STAGE"
