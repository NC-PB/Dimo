# 12 Implementation defaults

Decisions that close the remaining gaps, so implementation can proceed without asking. Each default can be changed later by the owner, but until then it is binding.

## Product and repository

| ID | Topic | Decision |
|---|---|---|
| D-01 | Repository | Public from the first commit at `github.com/NC-PB/dimo`. Local working folder stays `project_moonshot`. |
| D-02 | Governance | Single maintainer (owner). Files at repository creation: `LICENSE` (official Apache 2.0 text), `NOTICE`, `CONTRIBUTING.md` (DCO sign-off, clean room policy link), `CODE_OF_CONDUCT.md` (Contributor Covenant), `SECURITY.md` (private reporting via GitHub security advisories). |
| D-03 | Versioning | Semantic versioning, `0.x` until 1.0. Conventional Commits. Changelog generated from commits. |
| D-04 | Releases | GitHub Releases host installers and model files. Model files are referenced by checksum in `data/models/manifest.toml`. |
| D-05 | Updates | Tauri updater against GitHub Releases. Update check is **off by default** and offered once in the first run dialog (consistent with NFR-SEC-03). |
| D-06 | Crash reporting | None. Local rotating log files and a "Copy diagnostics" button that copies logs without drawing content. |
| D-08 | Code signing (Q-08) | **Unsigned for 0.1** on all platforms. macOS builds get an ad-hoc signature (required to run on Apple Silicon), no notarization. The getting started guide explains the Windows SmartScreen warning ("More info", "Run anyway") and the macOS first launch (right click, Open, or allow in Privacy and Security settings). Signing is reconsidered before 1.0. |
| D-07 | Sustainability (Q-07) | Not relevant for implementation. Dimo is free. An optional sponsor link in the About dialog and on cnc-master.xyz/dimo. Business options can be decided after 0.1. |

## Platforms

| ID | Topic | Decision |
|---|---|---|
| D-10 | Minimum OS (Q-09) | Windows 10 22H2 and Windows 11, x64 (ARM64 after 1.0). macOS 12 or later, universal binary. Ubuntu 22.04 or later (AppImage and deb). |
| D-11 | WebView2 | Windows installer uses the WebView2 bootstrapper. The portable build documents the requirement and shows a clear message if WebView2 is missing. |

## Domain defaults

| ID | Topic | Decision |
|---|---|---|
| D-20 | Units (Q-13) | Metric and ISO first. Inch values are parsed, displayed and converted. ASME style decimal place rules are covered by FR-TOL-06. No ASME specific general tolerance tables before 1.0. Units are never inferred from sheet size (metric drawings on inch sheet formats exist in the corpus). |
| D-21 | Numbering (Q-12) | M1: numbers follow placement order, renumber by drag. From M2 the default strategy for new projects is **sheet, then zone, then reading order inside the zone** (top to bottom, left to right). |
| D-22 | Multi-instance | Default `4X` handling: one balloon with quantity 4. Sub-numbering (`5.1`, `5.2`) is a project setting. |
| D-23 | Locked numbering | Locks automatically when the first report is exported as "issued". Default insert policy when locked: next free number. |
| D-24 | Balloon style | Circle, 7 mm diameter on the printed sheet (scaled with sheet size), white fill, 0.35 mm outline in blue `#0057B8`, bold sans number in black, leader line on. Rejected or failed characteristics are shown with shape plus color, never color alone. |
| D-25 | Reference and basic dimensions | Recognized, ballooned only if the user chooses, `inspect = false` by default. |
| D-26 | Classification | Default `none`. Critical marker symbols configurable in profiles. |
| D-27 | Audit user | Operating system user name, editable in settings. |
| D-28 | Autosave | Journal write every 30 s and after each command batch. |

## Reports

| ID | Topic | Decision |
|---|---|---|
| D-30 | 0.1 reports | Dimo's own neutral layouts: **check sheet** (characteristic list with empty result columns, for printing) and **inspection report** (results, pass/fail, sample info, signature fields). Both as PDF (Typst) and XLSX. |
| D-31 | Report forms (Q-05) | First article and initial sample outputs (M7) use **Dimo's own layouts** that carry the same information in a compatible structure. No reproduction of form graphics or text from published standards. Users who need their customer's exact form import it as an XLSX template with placeholders. |
| D-32 | Report language | English and German report templates, chosen per export independent of the UI language. |
| D-33 | Ballooned PDF | Balloons are written as vector content into a copy of the original PDF. The original file is never modified. Optional: balloons as PDF annotations instead (setting). |

## Recognition

| ID | Topic | Decision |
|---|---|---|
| D-40 | OCR models | Most recent PP-OCR text detection and recognition models available under Apache 2.0 at implementation time, converted to ONNX, smallest ("mobile") variant first. Pinned by checksum. |
| D-41 | Large scans (Q-11) | Supported up to A0 at 600 dpi. Processing in tiles of 2048 px with overlap. OCR runs at 300 to 400 dpi equivalent. The 1.5 GB memory budget of NFR-PERF-03 applies. |
| D-43 | Tolerance table values | The agent drafts all table files (ISO 2768-1, ISO 2768-2, ISO 286 hole and shaft deviations, ISO 13920, ISO 9013) from its knowledge of the standards. The **owner verifies every value against his Tabellenbuch**. Each table file carries `status = "draft"` or `status = "verified"` plus `verified_by` and `verified_date`. Each table has a test vector file with spot checks taken by the owner. Rules: draft tables work in development builds but the UI marks derived limits from draft tables with a warning badge. A release build is blocked by CI if any shipped table is still `draft`. |
| D-42 | Test drawings (Q-10) | Open, owner is looking for drawings. Not blocking before M4: until then the synthetic drawing generator and drawings created by the owner are used. Corpus format (`*.truth.json`) is defined in M0. |

## Frontend

| ID | Topic | Decision |
|---|---|---|
| D-50 | Navigation | Single main window, no router library. Views: Drawing, Review, Measure, Export, Settings, switched by a view state store. |
| D-51 | Theme | Light and dark theme following the operating system, switchable in settings. |
| D-52 | Shortcuts | Shortcut map defined in one file, shown in a cheat sheet overlay (`?`). |
