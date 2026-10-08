# 06 Tech stack

Fixed requirement: **Tauri**. Everything else is chosen below. All listed dependencies use licenses compatible with Apache 2.0 (MIT, Apache 2.0, BSD). Versions are pinned when the repository is created.

## Summary

| Layer | Choice | License | Why |
|---|---|---|---|
| App shell | Tauri 2 | MIT / Apache 2.0 | Required. Small bundles, Rust backend, capability based security |
| Frontend framework | **Svelte 5** (runes) + TypeScript | MIT | See comparison below |
| Build | Vite | MIT | Standard for Tauri frontends |
| Styling | Tailwind CSS 4 with design tokens | MIT | Fast iteration, consistent theming, dark mode |
| UI primitives | Bits UI (headless components) | MIT | Accessible dialogs, menus, popovers without imposing a look |
| Data grid | TanStack Table + TanStack Virtual | MIT | Headless, virtualized, works with Svelte |
| Drawing overlay | Plain SVG, own components | | Full control, sheet space coordinates, no heavy canvas library needed |
| IPC typing | specta + tauri-specta | MIT | Generated TypeScript types for every command and event |
| i18n | Paraglide JS | MIT / Apache 2.0 | Compile time, type safe messages |
| Async / jobs | tokio, rayon | MIT | Orchestration and CPU parallelism |
| Errors / logs | thiserror, anyhow, tracing | MIT | Standard |
| PDF read and render | pdfium-render with prebuilt PDFium binaries | MIT / Apache 2.0, PDFium BSD | Robust renderer, text with glyph positions, page objects, writing support |
| PDF write (balloon overlay) | pdfium-render page objects, lopdf as fallback | MIT | Keep original vector content, add vector balloons |
| Image processing | image, imageproc | MIT | Deskew, binarize, morphology. No OpenCV build pain |
| ML inference | ort (ONNX Runtime) | MIT / Apache 2.0 | CPU first, optional GPU, cross platform |
| OCR models | PP-OCR text detection and recognition models in ONNX format | Apache 2.0 | Good on rotated and small text, permissive |
| OCR alternative | Tesseract via feature flag | Apache 2.0 | Fallback and comparison baseline |
| Symbol classifier | Own small CNN, trained on synthetic data, ONNX | Apache 2.0 (ours) | GD&T and drawing symbols that generic OCR does not know |
| Parser | winnow | MIT | Fast parser combinators with good error reporting |
| Spreadsheet write | rust_xlsxwriter | MIT / Apache 2.0 | Clean new workbooks |
| Spreadsheet templates | umya-spreadsheet | MIT | Read, modify, write existing templates with formatting |
| Spreadsheet / CSV read | calamine, csv | MIT | Measurement imports |
| Reports to PDF | Typst as a library | Apache 2.0 | Text based, versionable templates, high quality PDF, sandboxed rendering |
| XML (QIF) | quick-xml + serde | MIT | QIF plans and results |
| Project container | zip + serde_json, schemars for JSON Schema | MIT | Documented, diffable format |
| Testing | cargo test, insta (snapshots), proptest (parser), Vitest, WebdriverIO with tauri-driver | MIT | Unit, property, snapshot, end to end |
| License check | cargo-deny, license-checker for npm | MIT / Apache 2.0 | Enforce NFR-MNT-04 |
| CI / release | GitHub Actions, Tauri bundler, signed updater | | Windows MSI and portable, macOS universal DMG, Linux AppImage and deb |

## Frontend framework comparison

What the frontend has to do:

1. A large interactive viewport: tiled raster, hundreds of balloons, leader lines, selection, drag, proposal ghosts.
2. A virtualized, inline editable table with hundreds of rows, synchronized with the viewport (select in one, highlight in the other).
3. Frequent small updates from patches.
4. No business logic (that is in Rust).

| Criterion | Svelte 5 | React 19 | SolidJS | Vue 3 | Rust UI (Leptos / Dioxus) |
|---|---|---|---|---|---|
| Fine grained updates for many overlay elements | Excellent, runes update only what changed | Good with discipline (memo, selectors), easy to get wrong | Excellent | Very good | Good |
| Boilerplate | Low | Medium to high | Low | Low | Medium |
| Bundle size and startup | Very small | Larger | Very small | Small | Wasm bundle, larger |
| SVG authored as markup | Natural | Natural | Natural | Natural | Possible |
| Ecosystem (grids, headless UI) | Good and growing | Largest | Small | Large | Small |
| Contributor pool for an open-source project | Medium | Largest | Small | Large | Small |
| Tauri templates and docs | First class | First class | First class | First class | Supported |
| Learning curve for you | Low | Medium | Medium | Low | Low for Rust, but immature tooling |

### Decision: Svelte 5 + TypeScript (Vite, no server side rendering)

- The viewport and table are update heavy. Svelte 5 runes give fine grained reactivity without memoization discipline, which keeps the code simple and fast by default.
- Because Rust owns all domain logic, the frontend is a comparatively small codebase. The ecosystem advantage of React matters less here than usual. The two libraries that matter most (headless table, headless UI primitives) exist for Svelte.
- Small bundles and fast startup suit a desktop tool that is opened many times a day.

**Accepted trade-off:** a smaller pool of potential contributors than React. Mitigation: strict separation (generated IPC types, no business rules in TypeScript) keeps the frontend replaceable, so the choice is not a lock in. See [ADR 0002](../adr/0002-frontend-svelte.md).

**Runner up:** React, if attracting frontend contributors becomes the main bottleneck.

**Not chosen:** a Rust UI framework. Attractive for a single language codebase, but editors with complex viewports, accessible components and grids are far less mature there.

## Platform specifics

- **Windows** is the primary target (WebView2). PDFium binary bundled per platform and architecture.
- **macOS** is the main development platform. Universal binary.
- **Linux** supported through WebKitGTK. Tested on Ubuntu LTS.
- **Model files** (OCR, symbol classifier) are downloaded at build time from release assets with checksums and bundled into installers. They are not committed to git.

## Optional, opt in

- **Operating system OCR backends** (Apple Vision on macOS, Windows OCR) as additional `OcrEngine` implementations. Useful for comparison, never required.
- **Local vision language model** through a user configured local endpoint for interpreting complex notes. Off by default, disabled by organization policy.
