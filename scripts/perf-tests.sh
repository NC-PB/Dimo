#!/usr/bin/env bash
# Performance tests with timing assertions (MF F9). They are `#[ignore]` and named `perf_*`, so
# the default `cargo test` run never depends on machine load. This script runs only them, one at
# a time, in the same debug profile the bounds were set for:
#   perf_measurement_state_under_20_ms_for_300_by_10   (dimo-desktop, fastest of 30 runs)
#   perf_box_select_under_50_ms                        (dimo-desktop)
#   perf_large_report_under_5_s                        (dimo-report, XLSX 300 x 10)
#
# Usage: ./scripts/perf-tests.sh [extra cargo test arguments]
# The desktop tests need PDFium (scripts/fetch-pdfium.sh or DIMO_PDFIUM_PATH); without it they
# return early unless DIMO_REQUIRE_PDFIUM is set.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo test --workspace "$@" -- --ignored --test-threads=1 --nocapture perf_
