#!/usr/bin/env bash
# Performance harness for M0 (T0.9): builds the 50 sheet A0 stress PDF, runs the tile benchmark
# in release mode and writes summary.json and summary.md. Not part of check.sh (takes minutes).
#
# Usage: ./scripts/perf.sh [--sheets N] [--seed N] [--out DIR]
#
# Output (default $CARGO_TARGET_DIR/perf, else target/perf; nothing is committed):
#   stress_<seed>_<sheets>.pdf  generated once, deterministic
#   harness.json                raw numbers of crates/dimo-pdf/examples/perf.rs
#   time.txt                    /usr/bin/time output (peak resident set size of the process)
#   summary.json, summary.md    harness numbers, machine, peak memory, comparison with the NFRs
#
# Needs PDFium (scripts/fetch-pdfium.sh or DIMO_PDFIUM_PATH) and jq. Frame times of the webview
# (NFR-PERF-02) are measured separately with `tauri dev`, see docs/perf/M0.md.
set -euo pipefail
cd "$(dirname "$0")/.."

SHEETS=50
SEED=1
TARGET="${CARGO_TARGET_DIR:-target}"
OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sheets) SHEETS="$2"; shift 2 ;;
    --seed) SEED="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
OUT="${OUT:-$TARGET/perf}"
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }
mkdir -p "$OUT"

echo "==> build (release)"
cargo build --release -q -p dimo-synth
cargo build --release -q -p dimo-pdf --example perf

PDF="$OUT/stress_${SEED}_${SHEETS}.pdf"
if [[ ! -f "$PDF" ]]; then
  echo "==> generate $PDF"
  "$TARGET/release/dimo-synth" --stress --seed "$SEED" --sheets "$SHEETS" --out "$OUT" >/dev/null
fi

echo "==> run harness"
BIN="$TARGET/release/examples/perf"
case "$(uname -s)" in
  Darwin) TIME=(/usr/bin/time -l) ;;
  Linux) TIME=(/usr/bin/time -v) ;;
  *) TIME=() ;;
esac
if [[ ${#TIME[@]} -gt 0 ]]; then
  "${TIME[@]}" "$BIN" "$PDF" --json "$OUT/harness.json" 2> "$OUT/time.txt"
else
  "$BIN" "$PDF" --json "$OUT/harness.json"
  : > "$OUT/time.txt"
fi

# Peak resident set size and (macOS) peak memory footprint of the whole process, in MiB.
peak_rss_mib=0
peak_footprint_mib=0
if grep -q "maximum resident set size" "$OUT/time.txt"; then
  bytes=$(awk '/maximum resident set size/{print $1}' "$OUT/time.txt")
  peak_rss_mib=$((bytes / 1048576))
  bytes=$(awk '/peak memory footprint/{print $1}' "$OUT/time.txt")
  peak_footprint_mib=$((${bytes:-0} / 1048576))
elif grep -q "Maximum resident set size" "$OUT/time.txt"; then
  kib=$(awk -F: '/Maximum resident set size/{gsub(/ /,"",$2); print $2}' "$OUT/time.txt")
  peak_rss_mib=$((kib / 1024))
fi

case "$(uname -s)" in
  Darwin)
    cpu=$(sysctl -n machdep.cpu.brand_string)
    ram_gib=$(($(sysctl -n hw.memsize) / 1073741824))
    cores=$(sysctl -n hw.ncpu)
    os="macOS $(sw_vers -productVersion)"
    ;;
  Linux)
    cpu=$(awk -F: '/model name/{gsub(/^ /,"",$2); print $2; exit}' /proc/cpuinfo)
    ram_gib=$(($(awk '/MemTotal/{print $2}' /proc/meminfo) / 1048576))
    cores=$(nproc)
    os="$(uname -sr)"
    ;;
  *) cpu=unknown; ram_gib=0; cores=0; os="$(uname -sr)" ;;
esac

jq -n \
  --arg date "$(date -u +%Y-%m-%d)" \
  --arg cpu "$cpu" --argjson ram "$ram_gib" --argjson cores "$cores" --arg os "$os" \
  --arg rustc "$(rustc --version)" \
  --argjson rss "$peak_rss_mib" --argjson footprint "$peak_footprint_mib" \
  --slurpfile h "$OUT/harness.json" \
  '{date: $date,
    machine: {cpu: $cpu, ram_gib: $ram, cores: $cores, os: $os, rustc: $rustc},
    process_peak_mib: {rss: $rss, footprint: $footprint},
    harness: $h[0]}' > "$OUT/summary.json"

jq -r '
  def f1: . * 10 | round / 10;
  . as $s | .harness as $h |
  ($h.read_ms + $h.open_ms) as $open |
  "# Performance run \($s.date)\n\n" +
  "Machine: \($s.machine.cpu), \($s.machine.cores) cores, \($s.machine.ram_gib) GiB, \($s.machine.os)\n\n" +
  "Document: \($h.file), \($h.sheets) sheets, \($h.file_mib | f1) MiB\n\n" +
  "| Measure | Value | Target |\n|---|---|---|\n" +
  "| Open (read, hash, parse) | \($open | f1) ms | |\n" +
  "| First tile of sheet 1 | \($h.first_sheet.first_tile_ms) ms | |\n" +
  "| First sheet visible (open + all \($h.first_sheet.tiles) tiles) | \($h.first_sheet.read_open_view_ms | f1) ms | < 2000 ms (NFR-PERF-01) |\n" +
  "| Tile latency, one at a time, 100 percent level | mean \($h.serial.mean) ms, p95 \($h.serial.p95) ms | |\n" +
  "| Pan at 60 steps/s: late frames | \($h.pan.late_frames) of \($h.pan.frames), longest stall \($h.pan.longest_stall_frames) frames | tile side of NFR-PERF-02 |\n" +
  "| Pan: tile latency | p50 \($h.pan.tile_latency_ms.p50) ms, p95 \($h.pan.tile_latency_ms.p95) ms | |\n" +
  "| Scroll through all sheets: fit view | p50 \($h.scroll.fit_view_ms.p50) ms, max \($h.scroll.fit_view_ms.max) ms | |\n" +
  "| Scroll through all sheets: detail view | p50 \($h.scroll.detail_view_ms.p50) ms, max \($h.scroll.detail_view_ms.max) ms | |\n" +
  "| Peak resident set size (process) | \($s.process_peak_mib.rss) MiB | < 1536 MiB (NFR-PERF-03) |\n" +
  "| Peak memory footprint (macOS) | \($s.process_peak_mib.footprint) MiB | < 1536 MiB (NFR-PERF-03) |\n" +
  "| Resident set size sampled, peak | \($h.memory_mib.peak_sampled) MiB | |\n"
' "$OUT/summary.json" > "$OUT/summary.md"

echo "==> done"
cat "$OUT/summary.md"
