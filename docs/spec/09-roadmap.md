# 09 Roadmap

Each milestone ends with something usable. Manual work is made fast first, automation is layered on top. This keeps the tool valuable even when recognition fails, and gives real projects for building the test corpus.

| MS | Name | Deliverable | Exit criteria | Status (2026-10-14) |
|---|---|---|---|---|
| **M0** | Foundations | Repo, workspace, CI with license checks, Tauri shell, pdfium tiles over custom protocol, viewport with pan and zoom | 50 sheet A0 PDF opens and pans smoothly on reference hardware | Implemented, CI green on macOS, Windows, Linux, exit criterion measured on the owner's Mac ([perf](../perf/M0.md)). Owner reviews open |
| **M1** | Manual ballooning MVP | Place balloons, characteristic table synced with viewport, numbering and renumbering, styles, undo, project save and load, ballooned PDF and CSV and XLSX export, EN and DE UI | A real drawing can be ballooned and exported faster than with a PDF editor and a spreadsheet | Implemented, smoke test passes. Owner: manual test and exit criterion ([procedure](../perf/M1-exit.md)) |
| **M2** | Assisted capture | Box select with PDF text, callout parser, tolerance engine (ISO 2768 first, then fits, decimal rules, custom tables), explanations, numbering strategies | Box select on a vector PDF yields correct limits for 99 percent of common callouts | Implemented ([plan](../plan/M2.md)), strict evaluation 100 % on draft tables. Owner: table verification, truth file review, confirmation on a real drawing |
| **M3** | Simple shop reports | Samples, measurement grid, pass/fail, simple check sheet and generic inspection report (PDF and XLSX), one bundled default template | A one person shop goes from customer PDF to a finished report in one sitting | In progress ([plan](../plan/M3.md)): reports built; UI rework (Office-style shell, brand colors, light and dark) before the exit run |
| **MF** | Foundation | Drawing source abstraction (PDF or image, schema 4), domain orchestration out of the shell, structured errors, field-level update commands, one command table, logging, frontend foundation for dialogs and the image editor ([plan](../plan/MF.md)) | App behaves as before, exports byte identical, documented touch points for new sources, dialogs and commands | Done (2026-10-15) |
| **0.1** | **First public release** | Unsigned installers (see D-08), getting started guide including first launch instructions, demo drawing, all shipped tolerance tables verified (D-43), drawings from images and phone photos with rotate, trim, four-corner correction and paper format, ballooned by hand ([plan](../plan/R01.md)) | Usable by small shops without help | Planned |
| **M4** | Automatic recognition | Layout analysis, title block, zone grid, token grouping, proposals, confidence, review queue, coverage view, balloon placement on vector PDFs; feature control frames, basic and reference dimensions, notes and flag notes, variable tables, composite characteristics from PDF text | Recall of 98 percent on vector_text corpus, review faster than manual capture, frames parsed completely including datums on 95 percent of corpus frames | Planned |
| **M5** | Scans and machine learning | ONNX OCR backend, rotated text, preprocessing for scans and photos, outlined vector sheets, symbol classifier (geometric tolerance, surface texture, welding, datum symbols), model files and policy file | Recall of 90 percent on clean scan corpus | Planned |
| **M6** | Interchange, advanced reports and revisions | CSV and spreadsheet result import with mapping profiles, custom templates, first article forms, initial sample report, customer profiles, template packs; revision import, visual comparison, carry-over; CLI, JSON output, QIF export and import | A complete first article package can be produced end to end; revision update of a 200 characteristic drawing in under 15 minutes; batch run of a folder of drawings from the command line | Planned |
| **1.0** | Release | Documentation complete, sample projects, signed installers for all platforms (patent review already done) | All "M" requirements met, NFR targets met | Planned |

Milestones after 0.1 were merged from six (M4 to M9) into three on 2026-10-14 because they share code ([ADR 0009](../adr/0009-roadmap-consolidation.md)); each new milestone keeps the exit criteria of its parts and ships as its own release (0.2, 0.3, 0.4).

Progress and task details live in [docs/plan/STATUS.md](../plan/STATUS.md) and the milestone files
`docs/plan/M<n>.md`. The status column is updated when a milestone changes state.

Ordering rationale: the primary audience (small shops, one person companies) needs a fast manual workflow, automatic ISO 2768 tolerances and a clean simple report far more than automation or aerospace forms. That path is complete after M3 and ships as 0.1. Automation and heavier formats follow.

## After 1.0 (ideas, not commitments)

- 3D model based definition (STEP AP242 with semantic PMI) as an additional source, bypassing OCR entirely.
- Plugin system for custom exporters and parsers (sandboxed WebAssembly components).
- Watch folder and integration recipes for automation tools.
- Optional local vision language model assist for complex notes.
- Basic statistics across samples (not a full SPC system).
- Additional UI languages (French, Italian for Switzerland, then others).

## First steps

All four were done in M0 (T0.1, T0.5 to T0.8, T0.6, T0.10 and T0.11). Findings: the viewport meets
NFR-PERF-01 to 03 on the owner's Mac (`docs/perf/M0.md`); the first corpus drawing has real text
whose font maps codes to Unicode only by coincidence (`corpus/notes/test_drawing_1.md`); outlined
text frequency still needs more corpus drawings (Q-10).

1. Create the repository skeleton from [05 Architecture](05-architecture.md).
2. Spike: pdfium-render in a Tauri 2 app serving tiles over a custom protocol, Svelte viewport with SVG overlay. Measure NFR-PERF-01 and -02 early, because everything depends on the viewport feeling good.
3. Spike: extract text runs with geometry from a handful of real CAD exported PDFs to learn how often text is outlined.
4. Start the synthetic drawing generator and the corpus format in parallel, since every recognition milestone depends on it.
