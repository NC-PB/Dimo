# ADR 0002: Svelte 5 as frontend framework

Status: accepted

## Context
The frontend needs a fast, update heavy drawing viewport with an SVG overlay and a virtualized editable table, kept in sync. Business rules are in Rust. Candidates: Svelte 5, React, SolidJS, Vue, Rust UI frameworks. Full comparison in [06 Tech stack](../spec/06-tech-stack.md).

## Decision
Svelte 5 with runes, TypeScript, Vite, no server side rendering. Headless libraries for the table (TanStack Table and Virtual) and UI primitives (Bits UI).

## Consequences
- Fine grained updates by default, small bundles, low boilerplate.
- Smaller contributor pool than React. Mitigated by generated IPC types and no business logic in the frontend.
- Revisit if contributor acquisition becomes the main bottleneck.
