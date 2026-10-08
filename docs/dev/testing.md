# Testing

| Layer | Tool | Where |
|---|---|---|
| Rust unit and integration | `cargo test` | next to the code, `tests/` per crate |
| Parsers | `proptest` round trip (generate, print, parse) | `dimo-notation` |
| Tolerance tables | table driven tests from `data/tolerances/*.test.toml` | `dimo-tolerance` |
| Output formats (CSV, XLSX, PDF structure, project JSON) | `insta` snapshots | `dimo-io`, `dimo-report` |
| Generated JSON schemas (`docs/schema/`) | test compares with the types; regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-core --test corpus_truth` | `dimo-core/tests/` |
| Project file migrations | one fixture per schema version | `dimo-io/tests/fixtures/` |
| Recognition quality | corpus evaluation (recall, precision, field accuracy, calibration) | from M4, gated in CI (NFR-REC-05) |
| Frontend units | Vitest | `apps/desktop/src/**/*.test.ts` |
| End to end | WebdriverIO with tauri-driver | `apps/desktop/e2e/`, from M1 |
| Performance | benchmark harness with synthetic PDFs | `crates/dimo-pdf/benches/`, results in `docs/perf/` |

## Rules

- Every bug fix comes with a test that fails without the fix.
- Snapshot changes are reviewed, never blindly accepted.
- Tests must not need the network. PDFium is loaded from `vendor/pdfium/`; tests that need it
  are skipped with a clear message if it is missing, and CI always provides it. With `CI=true` or
  `DIMO_REQUIRE_PDFIUM=1` a missing library fails these tests instead of skipping them.
- Corpus based tests reference drawings by file name and verify the SHA-256 from `PROVENANCE.md`.
- Ground truth files `corpus/truth/*.truth.json` are validated against `docs/schema/truth.schema.json`
  and checked for consistency (`dimo_core::truth`). Expected limits come from the drawing and the
  tolerance rules, never from recognizer output.
- `corpus/drawings/test_drawing_1.pdf` is a required regression test: it has a real text layer
  (74 text objects) that some libraries fail to see. Sheet classification must report `vector_text`.
