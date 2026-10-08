# AGENTS.md

Instructions for AI coding agents (Claude Code, Codex, Cursor and others) working on Dimo.
Humans should read [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md) first.

## What Dimo is

Open-source, local-first desktop app for **drawing ballooning and inspection planning**.
It reads engineering drawings (PDF, scans), finds every inspectable characteristic, numbers it
with a balloon, interprets its tolerance, and produces inspection reports.
Primary users: small machine shops and one person companies. Simple beats complete.

Stack: Tauri 2, thick Rust core, thin Svelte 5 + TypeScript frontend. License Apache 2.0 with DCO.

## Where the truth lives

The specification is complete and binding. Read before you design anything.

| Need | File |
|---|---|
| Current milestone, progress, next task | [docs/plan/STATUS.md](docs/plan/STATUS.md) |
| Task breakdown of the current milestone | `docs/plan/M<n>.md` (start: [M0](docs/plan/M0.md)) |
| Requirements (FR-*, NFR-*) | [docs/spec/03](docs/spec/03-functional-requirements.md), [docs/spec/04](docs/spec/04-non-functional-requirements.md) |
| Architecture, crate layout, IPC | [docs/spec/05-architecture.md](docs/spec/05-architecture.md) |
| Libraries and versions | [docs/spec/06-tech-stack.md](docs/spec/06-tech-stack.md) |
| Data model, project file | [docs/spec/07-data-model.md](docs/spec/07-data-model.md) |
| Recognition pipeline | [docs/spec/08-recognition-pipeline.md](docs/spec/08-recognition-pipeline.md) |
| Milestones | [docs/spec/09-roadmap.md](docs/spec/09-roadmap.md) |
| Binding defaults (D-*) | [docs/spec/12-implementation-defaults.md](docs/spec/12-implementation-defaults.md) |
| Architecture decisions | [docs/adr/](docs/adr/) |
| Coding conventions | [docs/dev/](docs/dev/) (rust, frontend, testing, workflow, setup) |

Precedence when documents disagree: ADR (accepted) > spec 12 defaults > other spec files > this file.
If something is genuinely unspecified, choose the simplest option consistent with the spec,
write it into the decision log in `docs/plan/STATUS.md`, and tell the owner in your summary.
Do not stop work to ask about things the spec already answers.

## Hard rules

1. **Clean room.** Never name, imitate or reference other ballooning or inspection products
   anywhere (code, comments, docs, commits, test names). Never fetch or use their docs,
   screenshots, file formats or report layouts. Derive features from standards and user problems.
   See [docs/spec/00-clean-room-policy.md](docs/spec/00-clean-room-policy.md).
2. **Rust owns the document.** All business logic lives in `crates/dimo-*`. The frontend holds
   view state only and changes data exclusively through commands (ADR 0001, NFR-MNT-02).
3. **Proposals, not mutations.** Automation returns `Proposal`s. Only an explicit undoable
   command turns them into characteristics (ADR 0006).
4. **One coordinate system.** All geometry in sheet space: PDF user units, origin top left, per sheet.
5. **Exact numbers.** Nominals, deviations and limits use `rust_decimal`, never `f32`/`f64`.
   Floats are fine for screen geometry only.
6. **Generated IPC types.** TypeScript types for commands and events come from tauri-specta.
   Never hand write a duplicate type.
7. **Tolerance tables are drafts until the owner verifies them.** You may create and edit table
   files with `status = "draft"`. You must never set `status = "verified"`, `verified_by` or
   `verified_date`. Only the owner does that after checking his Tabellenbuch (D-43).
8. **Corpus provenance.** Never add a drawing to `corpus/` without a `PROVENANCE.md` entry.
   Never download drawings from the internet into the corpus.
9. **Local first.** No network calls in the default build, no telemetry, no new Tauri
   capabilities beyond what a feature needs (NFR-SEC-01).
10. **Licenses.** New dependencies must be MIT, Apache 2.0, BSD, ISC, Zlib or similar.
    `cargo deny check` must pass. Pin exact versions, at least 14 days old.
11. **Determinism.** Same project and app version produce byte identical exports (FR-EXP-11).
    No timestamps or random IDs in exports unless they come from the project.
12. **Never** modify files under `corpus/drawings/`, the `LICENSE` text, or accepted ADRs.
    Supersede an ADR with a new one instead.

## Commands

Run from the repository root. Some exist only after M0 tasks are done; check `docs/plan/STATUS.md`.

```sh
# Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p dimo-notation            # one crate, much faster
cargo insta review                     # accept snapshot changes after checking them
cargo deny check                       # licenses, advisories, duplicates

# Frontend (apps/desktop)
pnpm install
pnpm -C apps/desktop check             # svelte-check + tsc
pnpm -C apps/desktop lint
pnpm -C apps/desktop test              # vitest
pnpm -C apps/desktop tauri dev         # run the app

# Everything CI runs
./scripts/check.sh                     # fmt, clippy, tests, deny, frontend checks (skips parts that do not exist yet)
```

Prefer running the smallest relevant test first, the full suite before you finish.

## How to work

1. Read `docs/plan/STATUS.md`, then the task in the milestone file. Read the spec sections it cites.
2. For anything bigger than a small fix: write a short plan first (files, types, tests), then implement.
3. Work in small vertical steps. Each step compiles, is tested, and could be committed alone.
4. Write the test with the code. Parser and tolerance code get property or table driven tests,
   output formats get `insta` snapshots, every bug fix gets a regression test.
5. Verify: fmt, clippy, tests, and for UI work actually run the app and check the behavior.
   Never claim something works without having run it.
6. Update `docs/plan/STATUS.md` (task state, notes, decisions) and user docs in the same change
   (NFR-UX-06). Tick acceptance criteria in the milestone file.
7. Commit with Conventional Commits and DCO sign-off: `git commit -s -m "feat(dimo-pdf): ..."`.
   Scopes are crate or area names: `dimo-core`, `desktop`, `ci`, `docs`, `corpus`, `data`.

Requirement IDs: reference them in doc comments, tests and commits where relevant
(`// FR-TOL-01: precedence`), so traceability stays visible.

## When stuck

- Spec unclear: decide per "Where the truth lives" and log it.
- A spec value seems technically wrong (for example a performance target is impossible with the
  chosen library): do not silently deviate. Implement the closest option, document the measurement
  in STATUS.md and flag it for the owner.
- Library behaves unexpectedly: write a minimal failing test or spike under `spikes/`, not in a crate.
- Same failure three times in a row: stop, summarize what you tried, and ask.

## Style of written text

English for code, comments, docs and commits. Plain, flat technical language. No marketing words.
Do not use em dashes or en dashes as sentence breaks in docs; use commas, colons or full stops.
