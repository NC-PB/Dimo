# ADR 0005: PDFium in Rust as the single rendering and text source

Status: accepted

## Context
The viewer, the OCR input and the text layer must agree on coordinates exactly. Rendering in the webview (a JavaScript PDF library) and separately in Rust for OCR risks mismatches and doubles the work.

## Decision
PDFium through pdfium-render renders all tiles and OCR images and provides text runs with geometry. Tiles reach the frontend through a custom URI protocol. All geometry is stored in sheet space.

## Consequences
- One coordinate system, pixel exact overlays.
- PDFium binaries must be bundled per platform and architecture.
- Tile caching and prefetching are our responsibility.
