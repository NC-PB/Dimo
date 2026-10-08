# ADR 0004: Project file as ZIP container with JSON

Status: accepted

## Context
Projects must be portable, inspectable by other tools, robust across versions, and must contain the exact drawing they refer to.

## Decision
ZIP container with `manifest.json`, `project.json`, `audit.jsonl`, the original drawings stored by SHA-256 hash, and a profile snapshot. JSON Schema generated from the Rust types and published. Optional unzipped folder mode. Caches are never stored in the project.

## Consequences
- Human readable, diffable in folder mode, easy for third party tools.
- Whole file rewrite on save. Acceptable for expected sizes. Autosave uses a separate journal.
- Integer schema version with mandatory migrations and fixtures.
