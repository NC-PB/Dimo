# 04 Non-functional requirements

Reference hardware: a four year old office laptop (4 cores, 16 GB RAM, integrated graphics), Windows 11.

## Performance (NFR-PERF)

| ID | Target |
|---|---|
| NFR-PERF-01 | First sheet of a 50 sheet A0 PDF visible in under 2 s |
| NFR-PERF-02 | Pan and zoom at 60 fps with 500 balloons on screen |
| NFR-PERF-03 | Memory below 1.5 GB for a 50 sheet A0 project |
| NFR-PERF-04 | Box select recognition result in under 500 ms |
| NFR-PERF-05 | Auto detection of one A1 vector sheet under 5 s, raster A1 sheet at 300 dpi under 30 s |
| NFR-PERF-06 | UI stays responsive during any background job, every job is cancellable |

## Quality of recognition (NFR-REC)

| ID | Target for 1.0, measured on the reference corpus |
|---|---|
| NFR-REC-01 | Characteristic recall on vector PDFs with text layer at least 98 percent |
| NFR-REC-02 | Characteristic recall on clean scans at least 90 percent |
| NFR-REC-03 | Field accuracy (nominal, limits) of accepted proposals at least 99 percent after validation |
| NFR-REC-04 | Calibrated confidence: items above the high threshold are correct at least 99 percent of the time |
| NFR-REC-05 | Metrics run in CI on every change, regressions block merge |

The headline metric is **time to a verified characteristic list**, not raw detection rate.

## Security and privacy (NFR-SEC)

| ID | Requirement |
|---|---|
| NFR-SEC-01 | No network access in the default build permissions |
| NFR-SEC-02 | No telemetry. Optional crash reports only after explicit consent, stripped of drawing content |
| NFR-SEC-03 | Optional online features (update check, external AI endpoints) opt in and disabled by organization policy file |
| NFR-SEC-04 | Updates signed, update channel can be disabled for air gapped installs |
| NFR-SEC-05 | Project files contain no executable content. Template rendering is sandboxed |

## Reliability (NFR-REL)

| ID | Requirement |
|---|---|
| NFR-REL-01 | Autosave and crash recovery, no more than 30 s of work lost |
| NFR-REL-02 | Unlimited undo within a session, persistent audit log in the project |
| NFR-REL-03 | Project file schema versioned with forward migrations and tests for every version |
| NFR-REL-04 | Opening a project written by a newer version shows a clear message, never corrupts it |

## Usability (NFR-UX)

| ID | Requirement |
|---|---|
| NFR-UX-01 | Every common action reachable by keyboard, shortcuts discoverable |
| NFR-UX-02 | Guided first run with a bundled demo drawing |
| NFR-UX-03 | Bundled sample projects for aerospace, automotive and generic workflows |
| NFR-UX-04 | Every interpreted tolerance shows its rule in plain language |
| NFR-UX-05 | High contrast balloon styles, color blind safe pass/fail colors (shape plus color) |
| NFR-UX-06 | User documentation updated in the same pull request as the feature |

## Portability (NFR-PORT)

| ID | Requirement |
|---|---|
| NFR-PORT-01 | Windows 10/11 (primary, most quality departments), macOS, Linux |
| NFR-PORT-02 | Installer and portable build for Windows, no admin rights needed for the portable build |
| NFR-PORT-03 | Projects are portable between operating systems |

## Maintainability (NFR-MNT)

| ID | Requirement |
|---|---|
| NFR-MNT-01 | Domain logic in Rust crates without UI dependency, testable headless |
| NFR-MNT-02 | Frontend holds no business rules, only view state |
| NFR-MNT-03 | Generated TypeScript bindings for all IPC types, no hand written duplicates |
| NFR-MNT-04 | All dependencies license compatible with Apache 2.0, checked in CI |
