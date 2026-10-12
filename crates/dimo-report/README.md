# dimo-report

Report templating and PDF rendering of inspection reports (D-30, FR-EXP-06).

- `TemplateSet`: a template folder with `template.toml` and Typst files. The bundled
  `dimo-default` set (`data/templates/dimo-default/`) is embedded at compile time. Until T3.6 it
  only holds a placeholder document.
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

Rendering is not cancellable inside Typst; a report of 300 rows by 10 samples takes about
0.2 s in a release build (`spikes/typst-report/RESULTS.md`), so callers cancel between steps.
