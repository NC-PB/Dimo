# ADR 0001: Tauri with a thick Rust core

Status: accepted

## Context
Tauri is a fixed requirement. The application has heavy processing (PDF rendering, OCR, parsing, report generation) and must also run headless from a command line.

## Decision
All domain logic lives in Rust crates without UI dependencies. The frontend renders state and sends commands. The Rust core owns the document, the undo stack and the audit log. A CLI links the same crates.

## Consequences
- One source of truth, identical behavior in GUI and CLI.
- Business logic is testable without a webview.
- The frontend framework is replaceable.
- Every user action crosses IPC once. Gestures are handled optimistically and committed on release to hide latency.
