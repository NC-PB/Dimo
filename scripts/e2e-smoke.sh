#!/usr/bin/env bash
# End to end smoke test of the M1 main flow (T1.10). Starts the real desktop app (debug build) and
# drives it with the development hooks of `apps/desktop/src-tauri/src/dev.rs`:
#
#   1. Main flow: new project from the corpus drawing, place five balloons (two by box select,
#      one of them accepted from the proposal card without typing, T2.6), edit values in the
#      table, reorder, rotate the sheet, set scale and unit, save as, export PDF, CSV and XLSX
#      "as issued", save again. The results are checked from the files the app wrote.
#   2. Reopen: the saved project opens again with the same balloons, rotation and numbering lock.
#   3. Crash recovery: an unsaved project is killed with SIGKILL and restored at the next start.
#   4. Numbering strategies: place five balloons, draw a zone grid frame and a view, preview and
#      apply "sheet, zone, reading order" and "clockwise per view", undo.
#
# Not part of check.sh: it opens the app window, needs a desktop session and takes about
# 30 seconds after the build. tauri-driver cannot drive WKWebView on macOS, so this is not a
# WebDriver test (see docs/dev/testing.md). Native dialogs are replaced by DIMO_DEV_SAVE_AS and
# DIMO_DEV_EXPORT_DIR; settings, autosave and caches go to a temporary DIMO_DEV_HOME, so your own
# settings and unsaved work are not touched. Other things the app cannot do without a person
# (native dialogs, close prompt, pinch zoom feel) are in docs/dev/manual-test-m1.md.
#
# Usage: ./scripts/e2e-smoke.sh [--keep]
#   --keep   keep the work folder also when all checks pass (it is always kept on failure)
#
# Needs: PDFium (scripts/fetch-pdfium.sh or DIMO_PDFIUM_PATH), pnpm with installed packages, jq,
# unzip, curl, a free port 1420 (stop `tauri dev` first) and a desktop session.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD

KEEP=0
case "${1:-}" in
  "") ;;
  --keep) KEEP=1 ;;
  *) echo "usage: $0 [--keep]" >&2; exit 2 ;;
esac

TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
APP="$TARGET/debug/dimo-desktop"
DRAWING="$ROOT/corpus/drawings/test_drawing_1.pdf"
SCRIPTS="$ROOT/scripts/e2e"
PORT=1420
TIMEOUT=120

step() { printf '\n==> %s\n' "$*"; }
FAILED=0
fail() { echo "FAIL: $*" >&2; FAILED=$((FAILED + 1)); }
pass() { echo "ok:   $*"; }
# expect <description> <expected> <actual>
expect() {
  if [[ "$2" == "$3" ]]; then pass "$1"; else fail "$1: expected '$2', got '$3'"; fi
}

for tool in jq unzip curl pnpm cargo; do
  command -v "$tool" >/dev/null || { echo "$tool is required" >&2; exit 1; }
done
if [[ -z "${DIMO_PDFIUM_PATH:-}" ]]; then
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) pdfium="vendor/pdfium/mac-arm64/lib/libpdfium.dylib" ;;
    Darwin-x86_64) pdfium="vendor/pdfium/mac-x64/lib/libpdfium.dylib" ;;
    Linux-x86_64) pdfium="vendor/pdfium/linux-x64/lib/libpdfium.so" ;;
    Linux-aarch64) pdfium="vendor/pdfium/linux-arm64/lib/libpdfium.so" ;;
    *) pdfium="" ;;
  esac
  [[ -n "$pdfium" && -f "$ROOT/$pdfium" ]] && export DIMO_PDFIUM_PATH="$ROOT/$pdfium"
fi
[[ -n "${DIMO_PDFIUM_PATH:-}" ]] || { echo "PDFium not found, run scripts/fetch-pdfium.sh" >&2; exit 1; }
[[ -d apps/desktop/node_modules ]] || { echo "run pnpm install first" >&2; exit 1; }
if curl -sf "http://localhost:$PORT/" >/dev/null 2>&1; then
  echo "port $PORT is in use (tauri dev running?), stop it first" >&2
  exit 1
fi

step "build"
pnpm -C apps/desktop i18n >/dev/null
cargo build -q -p dimo-desktop
cargo build -q -p dimo-pdf --example text_runs
TEXT_RUNS="$TARGET/debug/examples/text_runs"

mkdir -p "$TARGET"
WORK=$(mktemp -d "$TARGET/e2e-XXXXXX")
VITE_PID=""
APP_PID=""
cleanup() {
  [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
  [[ -n "$VITE_PID" ]] && kill "$VITE_PID" 2>/dev/null || true
  if [[ $FAILED -eq 0 && $KEEP -eq 0 ]]; then
    rm -rf "$WORK"
  else
    echo "work folder kept: $WORK"
  fi
}
trap cleanup EXIT

step "start the frontend dev server"
(cd apps/desktop && exec node_modules/.bin/vite) >"$WORK/vite.log" 2>&1 &
VITE_PID=$!
for _ in $(seq 1 60); do
  curl -sf "http://localhost:$PORT/" >/dev/null 2>&1 && break
  sleep 1
done
curl -sf "http://localhost:$PORT/" >/dev/null || { echo "dev server did not start, see $WORK/vite.log" >&2; exit 1; }

# run_app <name> <ui script> <how to end: term|kill> [VAR=value ...]
# Starts the app with its own DIMO_DEV_HOME (the folder of <home> below $WORK), waits until the
# UI script is done and ends the app. The terminal output is $WORK/<name>.log.
run_app() {
  local name="$1" script="$2" ending="$3" home="$4"
  shift 4
  mkdir -p "$WORK/$home/config"
  [[ -f "$WORK/$home/config/settings.json" ]] || echo '{"locale":"en"}' >"$WORK/$home/config/settings.json"
  env DIMO_DEV_HOME="$WORK/$home" DIMO_DEV_UI_SCRIPT="$script" "$@" "$APP" >"$WORK/$name.log" 2>&1 &
  APP_PID=$!
  local waited=0
  until grep -q "ui: script done" "$WORK/$name.log"; do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
      local status=0
      wait "$APP_PID" 2>/dev/null || status=$?
      fail "$name: the app ended early (exit status $status), see $WORK/$name.log"
      APP_PID=""
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
    if [[ $waited -ge $TIMEOUT ]]; then
      fail "$name: the UI script did not finish in ${TIMEOUT}s, see $WORK/$name.log"
      break
    fi
  done
  if [[ "$ending" == kill ]]; then kill -9 "$APP_PID" 2>/dev/null || true; else kill "$APP_PID" 2>/dev/null || true; fi
  wait "$APP_PID" 2>/dev/null || true
  APP_PID=""
  if grep -q "ui: step .* failed" "$WORK/$name.log"; then
    fail "$name: a UI script step failed: $(grep -m1 'ui: step .* failed' "$WORK/$name.log")"
  fi
}

# report <name> <label>: the last "report" line of that label from the terminal output.
report() { { grep "ui: $2:" "$WORK/$1.log" || true; } | tail -1 | sed 's/^.*ui: //'; }

step "1. main flow"
mkdir -p "$WORK/out"
run_app main "$SCRIPTS/main-flow.json" term home1 \
  DIMO_DEV_OPEN="$DRAWING" DIMO_DEV_EXPORT_DIR="$WORK/out" DIMO_DEV_SAVE_AS="$WORK/smoke.dimo"

PROJECT="$WORK/smoke.dimo"
CSV="$WORK/out/smoke_characteristics.csv"
XLSX="$WORK/out/smoke_characteristics.xlsx"
PDF="$WORK/out/smoke_ballooned.pdf"
for f in "$PROJECT" "$CSV" "$XLSX" "$PDF"; do
  if [[ -s "$f" ]]; then pass "written: $(basename "$f")"; else fail "missing or empty: $f"; fi
done
if [[ ! -e "$PROJECT.journal" ]]; then
  pass "no journal left next to the saved project"
else
  fail "journal left next to the saved project"
fi

yesno() { if "$@"; then echo yes; else echo no; fi; }

# The saved project: manifest, project.json, audit.jsonl and the drawing in one zip file.
JSON=""
if [[ -s "$PROJECT" ]]; then
  JSON=$(unzip -p "$PROJECT" project.json)
  jqp() { jq -r "$1" <<<"$JSON"; }
  expect "project: characteristics in number order" \
    "1:Ø8 c10|2:Ø8 f7|3:Ø30 H7 +0.0203 -0|4:100 +0 -0.6|5:Ø8 h6" \
    "$(jqp '[.characteristics | sort_by(.number | tonumber)[] | "\(.number):\(.requirement_text)"] | join("|")')"
  expect "project: one balloon per characteristic" "5" "$(jqp '.balloons | length')"
  expect "project: nominal and deviations typed in the table, limits worked out" \
    "8.0 0.02 -0.05 8.02 7.95 mm" \
    "$(jqp '.characteristics[] | select(.number == "2") | "\(.nominal) \(.upper_dev) \(.lower_dev) \(.upper_limit) \(.lower_limit) \(.unit)"')"
  expect "project: comment typed in the table" "check twice" \
    "$(jqp '.characteristics[] | select(.number == "4") | .comment')"
  expect "project: box select accepted with the printed limits and rule (T2.6)" \
    "Ø30 H7 +0.0203 -0 30.0203 30 explicit box_select" \
    "$(jqp '.characteristics[] | select(.number == "3") | "\(.requirement_text) \(.upper_limit) \(.lower_limit) \(.derivation.rule.rule) \(.origin)"')"
  expect "project: sheet rotation, unit and scale" "deg90 in 2:1" \
    "$(jqp '.revisions[0].sheets[0] | "\(.rotation) \(.unit) \(.scale.drawing):\(.scale.actual)"')"
  expect "project: numbering locked by the issued export" "issued_report 5" \
    "$(jqp '.numbering.lock | "\(.reason) \(.highest_number)"')"
  audit=$(unzip -p "$PROJECT" audit.jsonl | wc -l | tr -d ' ')
  expect "project: audit log has entries" "yes" "$(yesno test "$audit" -gt 10)"
fi

# CSV: header, one row per characteristic in number order, exact digits.
if [[ -s "$CSV" ]]; then
  expect "csv: header" "No,Kind,Requirement,Nominal,Upper deviation" "$(head -1 "$CSV" | cut -d, -f1-5)"
  expect "csv: rows" "6" "$(wc -l <"$CSV" | tr -d ' ')"
  expect "csv: number and requirement of every row" \
    "1,Ø8 c10 2,Ø8 f7 3,Ø30 H7 +0.0203 -0 4,100 +0 -0.6 5,Ø8 h6" \
    "$(tail -n +2 "$CSV" | cut -d, -f1,3 | tr '\n' ' ' | sed 's/ $//')"
  expect "csv: nominal, deviations and limits with their digits, unit" "8.0,0.02,-0.05,8.02,7.95,mm" \
    "$(sed -n 3p "$CSV" | cut -d, -f4-9)"
  expect "csv: comment" "check twice" "$(sed -n 5p "$CSV" | cut -d, -f20)"
fi

# XLSX: a zip with one worksheet of six rows, texts in the shared strings.
if [[ -s "$XLSX" ]]; then
  expect "xlsx: rows (header and five characteristics)" "6" \
    "$(unzip -p "$XLSX" xl/worksheets/sheet1.xml | grep -o '<row ' | wc -l | tr -d ' ')"
  expect "xlsx: requirement text present" "yes" \
    "$(yesno grep -q 'Ø30 H7 +0.0203 -0' < <(unzip -p "$XLSX" xl/sharedStrings.xml))"
fi

# PDF: the original runs plus one bold number run per balloon, at the balloon position.
if [[ -s "$PDF" && -n "$JSON" ]]; then
  "$TEXT_RUNS" "$DRAWING" >"$WORK/runs-before.txt"
  "$TEXT_RUNS" "$PDF" >"$WORK/runs-after.txt"
  before=$(head -1 "$WORK/runs-before.txt" | sed -E 's/.*: ([0-9]+) runs/\1/')
  after=$(head -1 "$WORK/runs-after.txt" | sed -E 's/.*: ([0-9]+) runs/\1/')
  expect "pdf: five new text runs (the balloon numbers)" "5" "$((after - before))"
  expect "pdf: the new runs are the numbers 1 to 5 in the balloon font" "1 2 3 4 5" \
    "$(awk '$5 == "OpenSans-Bold" { print $6 }' "$WORK/runs-after.txt" | sort | tr '\n' ' ' | sed 's/ $//')"
  # Balloon centers from the project; a number run must sit at the center (tolerance 2 units).
  jq -r '. as $p | .balloons[] | . as $b
         | ($p.characteristics[] | select(.id == $b.characteristic) | .number) as $n
         | "\($n) \($b.position.x) \($b.position.y)"' <<<"$JSON" >"$WORK/balloons.txt"
  off=$(awk 'NR == FNR { x[$1] = $2; y[$1] = $3; next }
             $5 == "OpenSans-Bold" {
               dx = ($1 + $3 / 2) - x[$6]; dy = ($2 + $4 / 2) - y[$6]
               if (dx < 0) dx = -dx
               if (dy < 0) dy = -dy
               if (dx > 2 || dy > 2) bad++
             }
             END { print bad + 0 }' "$WORK/balloons.txt" "$WORK/runs-after.txt")
  expect "pdf: every number sits at its balloon" "0" "$off"
fi

# The proposal card of the box select, before Enter accepted it (T2.6).
expect "app: proposal card shows the printed limits with rule explicit" \
  'card ["Ø30 H7 +0.0203 -0" 30.0203/30 explicit]' \
  "$({ grep -o 'card \[[^]]*\]' <<<"$(report main proposed)" || true; } | head -1)"

# What the app itself reported at the end of the flow.
final=$(report main final)
expect "app: all three exports finished" "yes" \
  "$(yesno test "$(grep -c 'exports \[ballooned_pdf done smoke_ballooned.pdf, csv done smoke_characteristics.csv, xlsx done smoke_characteristics.xlsx\]' <<<"$final")" -eq 1)"

step "2. reopen the saved project"
run_app reopen "$SCRIPTS/report.json" term home2 DIMO_DEV_OPEN="$PROJECT"
# Balloons (numbers, positions, texts), rotation and numbering lock of a report line.
state() { { grep -oE '[0-9]+ balloons \[[^]]*\]|rotation [0-9]+|locked [a-z_]+' <<<"$1" || true; } | tr '\n' ';'; }
reopened=$(report reopen reopened)
expect "reopened project: same balloons, rotation and numbering lock" "$(state "$final")" "$(state "$reopened")"
expect "reopened project: no recovery notice" "yes" "$(yesno grep -q 'restored false' <<<"$reopened")"

step "3. crash and recovery"
run_app crash "$SCRIPTS/crash-place.json" kill home3 DIMO_DEV_OPEN="$DRAWING"
placed=$(report crash "before crash")
expect "two balloons placed before the crash" "2" "$(grep -oE '[0-9]+ balloons' <<<"$placed" | cut -d' ' -f1)"
run_app recovered "$SCRIPTS/report.json" term home3
restored=$(report recovered reopened)
expect "unsaved project restored with the same balloons" "$(state "$placed")" "$(state "$restored")"
expect "restored project carries the recovery notice" "yes" "$(yesno grep -q 'restored true' <<<"$restored")"

step "4. numbering strategies (T2.7)"
run_app numbering "$SCRIPTS/numbering.json" term home4 DIMO_DEV_OPEN="$DRAWING"
# "P1:3 P2:4 ..." from the balloons of a report line, sorted by requirement text.
numbers_of() {
  { grep -oE '#[0-9.A-Z]+ [a-z]+ at [0-9.,-]+ "P[0-9]"' <<<"$1" || true; } |
    sed -E 's/^#([0-9.A-Z]+) .*"(P[0-9])"$/\2:\1/' | sort | tr '\n' ' ' | sed 's/ $//'
}
ghosts_of() { { grep -oE 'ghosts \[[^]]*\]' <<<"$1" || true; } | sed -E 's/ghosts \[(.*)\]/\1/'; }
expect "numbering: five balloons in placement order" "P1:1 P2:2 P3:3 P4:4 P5:5" \
  "$(numbers_of "$(report numbering placed)")"
expect "numbering: preview of sheet, zone, reading order (current>new)" "5>1 3>2 1>3 2>4 4>5" \
  "$(ghosts_of "$(report numbering "zone preview")")"
zones=$(report numbering "zones applied")
expect "numbering: apply gives the previewed numbers" "P1:3 P2:4 P3:2 P4:5 P5:1" "$(numbers_of "$zones")"
expect "numbering: preview of clockwise per view" "5>1 4>2 1>3 2>4 3>5" \
  "$(ghosts_of "$(report numbering "view preview")")"
expect "numbering: clockwise per view applied" "P1:5 P2:2 P3:4 P4:1 P5:3" \
  "$(numbers_of "$(report numbering "views applied")")"
expect "numbering: undo restores the zone numbers in one step" "$(numbers_of "$zones")" \
  "$(numbers_of "$(report numbering undone)")"

step "result"
if [[ $FAILED -eq 0 ]]; then
  echo "all checks passed"
else
  echo "$FAILED check(s) failed"
fi
exit $((FAILED > 0))
