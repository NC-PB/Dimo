#!/usr/bin/env bash
# End to end smoke test of the M1 main flow (T1.10). Starts the real desktop app (debug build) and
# drives it with the development hooks of `apps/desktop/src-tauri/src/dev.rs`:
#
#   1. Main flow: new project from the corpus drawing, place five balloons (two by box select,
#      one of them accepted from the proposal card without typing, T2.6; two typed right after
#      the click, before the value field has the focus, T2.7a), edit values in the
#      table, reorder, rotate the sheet, set scale and unit, save as, export PDF, CSV and XLSX
#      "as issued", save again. The results are checked from the files the app wrote.
#   2. Reopen: the saved project opens again with the same balloons, rotation and numbering lock.
#   3. Crash recovery: an unsaved project is killed with SIGKILL and restored at the next start.
#   4. Numbering strategies: place five balloons, draw a zone grid frame and a view, preview and
#      apply "sheet, zone, reading order" and "clockwise per view", undo.
#   5. Tolerance settings (T2.10): a box select without a general tolerance is discarded with
#      Escape, the project gets ISO 2768-1 class m (general tolerance field on the Home tab,
#      T3.16), the same box is accepted with the general limits, and a typed balloon is read again with "Update limits".
#   6. Project and report details (T3.4): part, revision and report fields are typed into
#      File > Info (T3.18) and saved; the company name comes from the settings of the user.
#   7. Measurement grid (T3.5): five characteristics from a dev script, three samples added, one
#      column typed with the keyboard only, a 5 by 3 block pasted, undone and redone in one step
#      each, saved; verdicts from the app and results from project.json.
#   8. Report export (T3.8): the saved measurement project is opened twice, each time the
#      inspection report is exported as PDF and XLSX; both runs give byte identical files.
#      The PDF text is read back with PDFium (T3.10): overall verdict, summary counts and the
#      verdict of each sample; the XLSX holds the verdict words. A third run cancels the export
#      and leaves no file.
#   9. Start screen and setup pane (T3.17): without a project the start screen card creates the
#      project (DIMO_DEV_NEW_DRAWING replaces the file dialog), the setup pane sets general
#      tolerance, unit, scale, part number and customer, and Start ballooning turns on placing.
#  10. Error messages (MF F3): in the German UI a broken company table and a text file as new
#      drawing are refused; the messages are German, without Rust's English text.
#  11. Newer project (NFR-REL-04): a project whose manifest has a schema version above the
#      supported one is opened from the recent list on the start screen; the app refuses it with
#      the newer version message and leaves the file untouched.
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
# zip, unzip, curl, a free port 1420 (stop `tauri dev` first) and a desktop session.
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

for tool in jq zip unzip curl pnpm cargo; do
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
# DIMO_E2E=1: no file watching and no hot reload (vite.config.ts), so a check run in the same
# checkout cannot reload the page and end a running UI script.
(cd apps/desktop && DIMO_E2E=1 exec node_modules/.bin/vite) >"$WORK/vite.log" 2>&1 &
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
    # A reloaded page does not play the script again (dev.rs), so waiting longer cannot help.
    if grep -q "ui: page loaded again" "$WORK/$name.log"; then
      fail "$name: the page reloaded while the UI script ran, see $WORK/$name.log"
      break
    fi
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
  # The script's pointer has its own ID, so moving the mouse over the window does not change its
  # gestures; the count only helps to explain an odd run.
  local real
  real=$({ grep -o 'real pointer events during the script: [0-9]*' "$WORK/$name.log" || true; } | grep -o '[0-9]*$' || true)
  if [[ -n "$real" && "$real" != 0 ]]; then
    echo "note: $name: $real real pointer events while the UI script ran"
  fi
}

# The dev server must never reload the page during the run: a reload ends a running UI script.
# DIMO_E2E bundles the dependencies up front (vite.config.ts); this makes a reload visible.
check_no_reload() {
  if grep -q "reloading" "$WORK/vite.log"; then
    fail "the dev server reloaded the page: $(grep -m1 reloading "$WORK/vite.log")"
  else
    pass "the dev server never reloaded the page"
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
  expect "csv: comment" "check twice" "$(sed -n 5p "$CSV" | cut -d, -f21)"
  # T2.8: the rule column, values typed in the table are manual, box select reads explicit.
  expect "csv: rule column" "Rule" "$(head -1 "$CSV" | cut -d, -f11)"
  expect "csv: rules of rows 2 and 3" "manual explicit" \
    "$(sed -n '3p;4p' "$CSV" | cut -d, -f11 | tr '\n' ' ' | sed 's/ $//')"
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

step "5. tolerance settings, box select and re-interpret (T2.10)"
run_app tolerance "$SCRIPTS/tolerance.json" term home5 \
  DIMO_DEV_OPEN="$DRAWING" DIMO_DEV_SAVE_AS="$WORK/tolerance.dimo"
card_of() { { grep -o 'card \[[^]]*\]' <<<"$1" || true; } | head -1; }
expect "box select without a general tolerance: rule no_tolerance_defined, no limits" \
  'card ["50" -/- no_tolerance_defined]' \
  "$(card_of "$(report tolerance 'proposed without general')")"
discarded=$(report tolerance discarded)
expect "Escape discards the card and adds nothing" "card [] 1 balloons" \
  "$({ grep -oE 'card \[[^]]*\]|[0-9]+ balloons' <<<"$discarded" || true; } | head -2 | tr '\n' ' ' | sed 's/ $//')"
expect "box select with ISO 2768-1 class m: general limits" \
  'card ["50" 50.3/49.7 general]' \
  "$(card_of "$(report tolerance 'proposed with general')")"
TOL="$WORK/tolerance.dimo"
if [[ -s "$TOL" ]]; then
  TJSON=$(unzip -p "$TOL" project.json)
  tjq() { jq -r "$1" <<<"$TJSON"; }
  expect "tolerance: general tolerance stored in the project settings" "iso-2768-1 m" \
    "$(tjq '.settings.tolerance.general | "\(.table.id) \(.class)"')"
  expect "tolerance: accepted box select has the general limits from the verified table" \
    "50 50.3 49.7 general false box_select" \
    "$(tjq '.characteristics[] | select(.requirement_text == "50") | "\(.requirement_text) \(.upper_limit) \(.lower_limit) \(.derivation.rule.rule) \(.derivation.draft) \(.origin)"')"
  expect "tolerance: typed value read again with the new setting" \
    "Ø8 8.2 7.8 general" \
    "$(tjq '.characteristics[] | select(.requirement_text == "Ø8") | "\(.requirement_text) \(.upper_limit) \(.lower_limit) \(.derivation.rule.rule)"')"
else
  fail "missing or empty: $TOL"
fi

step "6. project and report details (T3.4)"
# Company name and user name are set in the settings of this run only; the new project starts
# with them (M3 decision 15).
mkdir -p "$WORK/home6/config"
echo '{"locale":"en","company_name":"Muster GmbH","user_name":"Tester"}' >"$WORK/home6/config/settings.json"
run_app details "$SCRIPTS/details.json" term home6 \
  DIMO_DEV_OPEN="$DRAWING" DIMO_DEV_SAVE_AS="$WORK/details.dimo"
DET="$WORK/details.dimo"
if [[ -s "$DET" ]]; then
  DJSON=$(unzip -p "$DET" project.json)
  djq() { jq -r "$1" <<<"$DJSON"; }
  expect "details: part data stored" "4711-B Muster AG" \
    "$(djq '"\(.info.part_number) \(.info.customer)"')"
  expect "details: revision label stored trimmed" "C" "$(djq '.revisions[0].label')"
  expect "details: report fields stored, company name and inspector from the settings" \
    "IR-7 Ben Tester Muster GmbH" \
    "$(djq '"\(.report_info.report_number) \(.report_info.approved_by) \(.report_info.inspected_by) \(.report_info.company_name)"')"
else
  fail "missing or empty: $DET"
fi

step "7. measurement grid (T3.5)"
run_app measure "$SCRIPTS/measure.json" term home7 \
  DIMO_DEV_OPEN="$DRAWING" DIMO_DEV_SCRIPT="$SCRIPTS/measure-setup.json" \
  DIMO_DEV_SAVE_AS="$WORK/measure.dimo"
verdicts_of() { { grep -oE 'samples [0-9]+ results [0-9]+ verdicts \[[^]]*\] overall [a-z]+' <<<"$1" || true; } | head -1; }
expect "measure: three samples added, no results" \
  "samples 3 results 0 verdicts [1:--- 2:--- 3:--- 4:--- 5:---] overall open" \
  "$(verdicts_of "$(report measure 'samples added')")"
expect "measure: column typed with the keyboard: pass, OK, fail, not evaluated, pass on the limit" \
  "samples 3 results 5 verdicts [1:p-- 2:p-- 3:f-- 4:n-- 5:p--] overall fail" \
  "$(verdicts_of "$(report measure 'column 1')")"
pasted='samples 3 results 15 verdicts [1:ppf 2:pfp 3:ppf 4:nnn 5:ppf] overall fail'
expect "measure: 5 by 3 block pasted" "$pasted" "$(verdicts_of "$(report measure pasted)")"
expect "measure: undo removes the whole paste" \
  "samples 3 results 5 verdicts [1:p-- 2:p-- 3:f-- 4:n-- 5:p--] overall fail" \
  "$(verdicts_of "$(report measure 'paste undone')")"
expect "measure: redo brings it back" "$pasted" "$(verdicts_of "$(report measure 'paste redone')")"
MEA="$WORK/measure.dimo"
if [[ -s "$MEA" ]]; then
  MJSON=$(unzip -p "$MEA" project.json)
  mjq() { jq -r "$1" <<<"$MJSON"; }
  expect "measure: serials and result count stored" "1,2,3 15" \
    "$(mjq '"\([.samples[].serial] | join(",")) \(.results | length)"')"
  expect "measure: values of sample 1 as typed or pasted, in characteristic order" \
    "10.01|ok|20|6.9|29.9" \
    "$(mjq '.samples[0].id as $s | [.results[] | select(.sample == $s) | .value.value] | join("|")')"
  expect "measure: result types pinned or set (decision 4)" "variable attribute variable variable variable" \
    "$(mjq '[.characteristics | sort_by(.number | tonumber)[] | .result_type] | join(" ")')"
else
  fail "missing or empty: $MEA"
fi

step "8. report export (T3.8)"
# Two runs with their own settings and export folders; the same project must give the same
# bytes (FR-EXP-11, dates from the project, M3 decision 14).
for run in a b; do
  mkdir -p "$WORK/out8$run"
  if [[ -s "$MEA" ]]; then
    run_app "report-$run" "$SCRIPTS/report-export.json" term "home8$run" \
      DIMO_DEV_OPEN="$MEA" DIMO_DEV_EXPORT_DIR="$WORK/out8$run"
  fi
done
for ext in pdf xlsx; do
  first="$WORK/out8a/measure_inspection-report.$ext"
  second="$WORK/out8b/measure_inspection-report.$ext"
  if [[ -s "$first" && -s "$second" ]]; then
    pass "report: inspection report $ext written in both runs"
    if cmp -s "$first" "$second"; then
      pass "report: inspection report $ext byte identical on re-export"
    else
      fail "report: inspection report $ext differs between the two exports"
    fi
  else
    fail "report: missing or empty inspection report $ext ($first, $second)"
  fi
done
expect "report: PDF file header" "%PDF" "$(head -c 4 "$WORK/out8a/measure_inspection-report.pdf" 2>/dev/null || true)"
expect "app: both report exports finished" "yes" \
  "$(yesno grep -q 'inspection_report_pdf done measure_inspection-report.pdf, inspection_report_xlsx done measure_inspection-report.xlsx' <<<"$(report report-a exported)")"
# The text of the report PDF holds the verdicts (T3.10). Verdicts of single cells are drawn as
# marks, so the words are read where the text has them: the overall verdict, the counts of the
# summary and the verdict of each sample. Project of step 7: sample 1 passes every limit but has
# a value without limits (open), samples 2 and 3 each have a failing value; over 5
# characteristics and 3 samples that is 8 pass, 4 fail, 3 not evaluated and 0 missing.
RPDF="$WORK/out8a/measure_inspection-report.pdf"
if [[ -s "$RPDF" ]]; then
  # text_runs prints "x y width height font text" per run; keep x, y, height, font, text.
  "$TEXT_RUNS" "$RPDF" | tail -n +2 |
    awk '{ t = $6; for (i = 7; i <= NF; i++) t = t " " $i; print $1 "\t" $2 "\t" $4 "\t" $5 "\t" t }' \
      >"$WORK/report-text.tsv"
  # The overall verdict is the one run of about 10 pt height in the summary block (the page title
  # is taller); the counts share its line.
  expect "report pdf: overall verdict" "Fail" \
    "$(awk -F'\t' '$3 > 9.5 && $3 < 12 { print $5; exit }' "$WORK/report-text.tsv")"
  overall_y=$(awk -F'\t' '$3 > 9.5 && $3 < 12 { print $2; exit }' "$WORK/report-text.tsv")
  expect "report pdf: summary counts (characteristics, samples, pass, fail, not evaluated, missing)" \
    "5 3 8 4 3 0" \
    "$(awk -F'\t' -v y="$overall_y" '$4 == "OpenSans-Regular" && $2 - y < 3 && y - $2 < 3 { print $1 "\t" $5 }' \
         "$WORK/report-text.tsv" | sort -n | cut -f2 | tr '\n' ' ' | sed 's/ $//')"
  # The sample block lies above the characteristic table, whose header starts with "No.".
  table_y=$(awk -F'\t' '$5 == "No." { print $2; exit }' "$WORK/report-text.tsv")
  expect "report pdf: verdict of each sample (open, fail, fail)" "Open Fail Fail" \
    "$(awk -F'\t' -v ty="$table_y" '$2 < ty && $4 == "OpenSans-Regular" && ($5 == "Pass" || $5 == "Fail" || $5 == "Open") { print $2 "\t" $5 }' \
         "$WORK/report-text.tsv" | sort -n | cut -f2 | tr '\n' ' ' | sed 's/ $//')"
  expect "report pdf: entered values and the OK result are printed" "yes" \
    "$(yesno test "$(awk -F'\t' '$5 == "10.01" || $5 == "OK" || $5 == "29.9" { n++ } END { print n + 0 }' "$WORK/report-text.tsv")" -ge 3)"
else
  fail "missing or empty: $RPDF"
fi
# The XLSX report holds the verdict and result words as text (marks and words in one string).
RXLSX="$WORK/out8a/measure_inspection-report.xlsx"
if [[ -s "$RXLSX" ]]; then
  strings=$(unzip -p "$RXLSX" xl/sharedStrings.xml)
  for word in "✗ Fail" "✓ OK" "✗ Not OK"; do
    expect "report xlsx: '$word' present" "yes" "$(yesno grep -q "$word" <<<"$strings")"
  done
fi
# Cancel (NFR-PERF-06): each job step waits 1.5 s, the export is cancelled during the first.
mkdir -p "$WORK/out8c"
if [[ -s "$MEA" ]]; then
  run_app report-cancel "$SCRIPTS/report-cancel.json" term home8c \
    DIMO_DEV_OPEN="$MEA" DIMO_DEV_EXPORT_DIR="$WORK/out8c" DIMO_DEV_EXPORT_DELAY_MS=1500
fi
expect "report: cancelled export reported" "yes" \
  "$(yesno grep -q 'inspection_report_pdf cancelled' <<<"$(report report-cancel cancelled)")"
expect "report: cancelled export left no file" "0" "$(find "$WORK/out8c" -type f | wc -l | tr -d ' ')"

step "9. start screen and setup pane (T3.17)"
# No project at startup: the UI script clicks the start screen card, DIMO_DEV_NEW_DRAWING
# replaces the file dialog, and the setup pane answers go into the project.
run_app setup "$SCRIPTS/start-setup.json" term home9 \
  DIMO_DEV_NEW_DRAWING="$DRAWING" DIMO_DEV_SAVE_AS="$WORK/setup.dimo"
expect "setup: Start ballooning switches on Place balloon" "yes" \
  "$(yesno grep -q 'tool place' <<<"$(report setup 'after setup')")"
SET="$WORK/setup.dimo"
if [[ -s "$SET" ]]; then
  SJSON=$(unzip -p "$SET" project.json)
  sjq() { jq -r "$1" <<<"$SJSON"; }
  expect "setup: general tolerance, unit, scale, part number and customer stored" \
    "iso-2768-1 m in 2:1 BR-2041 Muster AG" \
    "$(sjq '"\(.settings.tolerance.general.table.id) \(.settings.tolerance.general.class) \(.revisions[0].sheets[0].unit) \(.revisions[0].sheets[0].scale.drawing):\(.revisions[0].sheets[0].scale.actual) \(.info.part_number) \(.info.customer)"')"
else
  fail "missing or empty: $SET"
fi

step "10. error messages in German (MF F3)"
# German UI. A company table that is no TOML and a "new project" from a text file are refused;
# the messages must be the German translations of the error reasons, with Rust's English text
# only in the collapsed Details (not rendered, so not part of the alert text).
mkdir -p "$WORK/home10/config"
echo '{"locale":"de"}' >"$WORK/home10/config/settings.json"
printf 'this is = = not a table\n' >"$WORK/broken.toml"
printf 'plain text, not a drawing\n' >"$WORK/notes.txt"
run_app errors "$SCRIPTS/errors.json" term home10 \
  DIMO_DEV_OPEN="$DRAWING" DIMO_DEV_IMPORT_TABLE="$WORK/broken.toml" \
  DIMO_DEV_NEW_DRAWING="$WORK/notes.txt"
table_alerts=$(report errors "table refused")
drawing_alerts=$(report errors "drawing refused")
expect "errors: refused table in German with its reason" "yes" \
  "$(yesno grep -q 'Tabelle abgelehnt.*Die Datei entspricht nicht dem Tabellenformat' <<<"$table_alerts")"
expect "errors: non-PDF drawing refused in German" "yes" \
  "$(yesno grep -q 'Die Datei ist keine Zeichnung, die DimoMark lesen kann' <<<"$drawing_alerts")"
expect "errors: no English error text in the German messages" "no" \
  "$(yesno grep -qiE '\.toml|\.txt|cannot|expected|not a PDF|error|failed' <<<"$table_alerts $drawing_alerts")"

step "11. project from a newer DimoMark (NFR-REL-04)"
# A manifest with schema version 999 is above any supported version; dimo-io checks the
# manifest before anything else, so the file needs nothing more. The recent list in the
# settings folder of this run lists it, and the UI script clicks it on the start screen (the
# button step matches the path in the title attribute).
NEWER="$WORK/newer.dimo"
mkdir -p "$WORK/newer" "$WORK/home11/config"
cat >"$WORK/newer/manifest.json" <<'JSON'
{
  "format": "dimo-project",
  "schema_version": 999,
  "app_version": "99.0.0",
  "created": "2030-01-01T00:00:00Z",
  "modified": "2030-01-01T00:00:00Z"
}
JSON
(cd "$WORK/newer" && zip -q -X "$NEWER" manifest.json)
jq -n --arg p "$NEWER" '{ projects: [$p] }' >"$WORK/home11/config/recent-projects.json"
before_sum=$(cksum <"$NEWER")
jq --arg p "$NEWER" 'map(if .button == "__NEWER_PROJECT__" then .button = $p else . end)' \
  "$SCRIPTS/newer-version.json" >"$WORK/newer-version.json"
run_app newer "$WORK/newer-version.json" term home11
expect "newer project: refused with the newer version message" "yes" \
  "$(yesno grep -q 'written by a newer DimoMark version (file format 999, this version reads up to' <<<"$(report newer 'newer refused')")"
expect "newer project: file left untouched" "$before_sum" "$(cksum <"$NEWER")"

step "dev server"
check_no_reload

step "result"
if [[ $FAILED -eq 0 ]]; then
  echo "all checks passed"
else
  echo "$FAILED check(s) failed"
fi
exit $((FAILED > 0))
