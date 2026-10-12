# dimo-report

Report templating and PDF rendering of inspection reports (D-30, FR-EXP-06).

- `ReportModel::build(&Project, &ReportOptions)` (T3.6): the data of a check sheet or an
  inspection report from a project and its evaluation (`dimo_core::measurement_state`). Every
  printed value is text: numbers with their typed decimals and the decimal separator of the
  report language, dates `YYYY-MM-DD`, verdicts and rules as codes the template translates.
  Format and schema: `data/templates/README.md`, `docs/schema/report-data.schema.json`
  (`data_schema_json`, regenerate with `DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-report --test
  report`).
- `render_report(&TemplateSet, &ReportModel)`: renders the matching document (`check-sheet`,
  `inspection-report`) in the model's language with the report date as PDF creation date.
- `TemplateSet`: a template folder with `template.toml`, a label file and Typst files. The
  bundled `dimo-default` set (`data/templates/dimo-default/`) is embedded at compile time.
- `render_pdf(&TemplateSet, &RenderRequest)`: compiles one document with Typst 0.15 and returns
  PDF bytes. The request carries the document name, the data as a JSON value (the template reads
  it with `json("/data.json")`), the language (`sys.inputs.lang`, `en` or `de`) and the creation
  date written into the PDF.
- Sandbox (NFR-SEC-05): the Typst world serves only the template files, the data file and the
  bundled Open Sans fonts from memory. Files outside the template, packages, plugins loaded from
  disk and the clock are refused with an error.
- Determinism (FR-EXP-11): the PDF identifier is a SHA-256 of template, document, language, date
  and data; no clock is read. Two renders give the same bytes (`tests/render.rs`).
- Errors from Typst come back as `ReportError::Compile` with file, line and column.

Tests: `tests/report.rs` (data JSON snapshots in English and German, schema, column blocks,
300 rows by 10 samples, byte identical renders; `DIMO_REPORT_SAMPLES_DIR=<dir>` keeps the
rendered PDFs), `tests/render.rs` (renderer and sandbox). The extracted PDF text snapshots need
PDFium and live in `crates/dimo-cli/tests/report_text.rs`.

Rendering is not cancellable inside Typst; a report of 300 rows by 10 samples takes about
0.2 s in a release build (`spikes/typst-report/RESULTS.md`), so callers cancel between steps.
