# 09 Roadmap

Each milestone ends with something usable. Manual work is made fast first, automation is layered on top. This keeps the tool valuable even when recognition fails, and gives real projects for building the test corpus.

| MS | Name | Deliverable | Exit criteria |
|---|---|---|---|
| **M0** | Foundations | Repo, workspace, CI with license checks, Tauri shell, pdfium tiles over custom protocol, viewport with pan and zoom | 50 sheet A0 PDF opens and pans smoothly on reference hardware |
| **M1** | Manual ballooning MVP | Place balloons, characteristic table synced with viewport, numbering and renumbering, styles, undo, project save and load, ballooned PDF and CSV and XLSX export, EN and DE UI | A real drawing can be ballooned and exported faster than with a PDF editor and a spreadsheet |
| **M2** | Assisted capture | Box select with PDF text, callout parser, tolerance engine (ISO 2768 first, then fits, decimal rules, custom tables), explanations, numbering strategies | Box select on a vector PDF yields correct limits for 99 percent of common callouts |
| **M3** | Simple shop reports | Samples, measurement grid, pass/fail, simple check sheet and generic inspection report (PDF and XLSX), one bundled default template | A one person shop goes from customer PDF to a finished report in one sitting |
| **0.1** | **First public release** | Unsigned installers (see D-08), getting started guide including first launch instructions, demo drawing, all shipped tolerance tables verified (D-43) | Usable by small shops without help |
| **M4** | Auto detection on vector PDFs | Layout analysis, title block, zone grid, token grouping, proposals, confidence, review queue, coverage view, balloon placement | Recall of 98 percent on vector_text corpus, review faster than manual capture |
| **M5** | OCR | ONNX OCR backend, rotated text, preprocessing for scans, outlined vector sheets, policy file | Recall of 90 percent on clean scan corpus |
| **M6** | Geometric tolerances and notes | Feature control frames, symbol classifier, basic and reference dimensions, notes and flag notes, variable tables, composite characteristics | Frames parsed completely including datums on 95 percent of corpus frames |
| **M7** | Advanced reports | CSV and spreadsheet result import with mapping profiles, custom templates, first article forms, initial sample report, customer profiles, template packs | A complete first article package can be produced end to end |
| **M8** | Revisions | Revision import, visual comparison, carry-over of characteristics | Revision update of a 200 characteristic drawing in under 15 minutes |
| **M9** | Automation and interchange | CLI, JSON output, QIF export and import | Batch run of a folder of drawings from the command line |
| **1.0** | Release | Documentation complete, sample projects, signed installers for all platforms (patent review already done) | All "M" requirements met, NFR targets met |

Ordering rationale: the primary audience (small shops, one person companies) needs a fast manual workflow, automatic ISO 2768 tolerances and a clean simple report far more than automation or aerospace forms. That path is complete after M3 and ships as 0.1. Automation and heavier formats follow.

## After 1.0 (ideas, not commitments)

- 3D model based definition (STEP AP242 with semantic PMI) as an additional source, bypassing OCR entirely.
- Plugin system for custom exporters and parsers (sandboxed WebAssembly components).
- Watch folder and integration recipes for automation tools.
- Optional local vision language model assist for complex notes.
- Basic statistics across samples (not a full SPC system).
- Additional UI languages (French, Italian for Switzerland, then others).

## First steps

1. Create the repository skeleton from [05 Architecture](05-architecture.md).
2. Spike: pdfium-render in a Tauri 2 app serving tiles over a custom protocol, Svelte viewport with SVG overlay. Measure NFR-PERF-01 and -02 early, because everything depends on the viewport feeling good.
3. Spike: extract text runs with geometry from a handful of real CAD exported PDFs to learn how often text is outlined.
4. Start the synthetic drawing generator and the corpus format in parallel, since every recognition milestone depends on it.
