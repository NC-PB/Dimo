# Status

Single place for progress. Agents update this file at the end of every task.

## Current

- Milestone: **M0 Foundations** ([plan](M0.md))
- Next task: **T0.2 Tauri 2 shell with Svelte 5**
- Release target: 0.1 after M3

## Tasks M0

| Task | State | Notes |
|---|---|---|
| T0.1 Cargo workspace skeleton | done | Dependency direction enforced by `crates/dimo-cli/tests/dependency_direction.rs` |
| T0.2 Tauri 2 shell with Svelte 5 | todo | |
| T0.3 Typed IPC with tauri-specta | todo | |
| T0.4 Checks and CI | todo | |
| T0.5 PDFium integration | todo | |
| T0.6 Text runs with geometry | todo | |
| T0.7 Tile protocol and cache | todo | |
| T0.8 Viewport with SVG overlay | todo | |
| T0.9 Performance harness | todo | |
| T0.10 Corpus truth format | todo | |
| T0.11 Synthetic generator (stretch) | todo | |

States: `todo`, `in progress`, `review` (waits for owner), `done`, `blocked` (reason in notes).

## Waiting for the owner

| Item | Since |
|---|---|
| Tolerance tables to verify against Tabellenbuch (D-43) | from M2 |
| More corpus drawings, especially scans and outlined text (Q-10) | before M4 |
| Trademark search Swissreg / EUIPO (Q-01) | before 0.1 |

## Decision log

Decisions made during implementation that are not in the spec. Newest first.
Format: date, decision, reason, who. Decisions that change architecture become ADRs.

| Date | Decision | Reason | By |
|---|---|---|---|
| 2026-10-08 | Internal dependency allow list: notation, pdf, vision, io, report may use core; tolerance may use core and notation; detect as in rust.md. Widen only with a log entry | rust.md leaves these edges open; core is a pure leaf, so no cycles | T0.1, agent |
| 2026-10-08 | `unwrap`/`expect`/`print_stdout`/`print_stderr` warn workspace wide, allowed in tests via `clippy.toml`; `PDFium` is a valid doc identifier | Enforces rust.md rules mechanically | T0.1, agent |
| 2026-10-08 | Code repository at `~/Repositories/dimo` (public `github.com/NC-PB/dimo`). Spec, ADRs and corpus were copied from the planning folder `project_moonshot`; from now on this repository is the source of truth | One self contained repo for agents and contributors | owner, setup |
| 2026-10-08 | pnpm (via corepack) as package manager for the frontend | Fast, strict, standard with Tauri; spec left it open | setup, owner may change |
| 2026-10-08 | `scripts/check.sh` as the single local and CI entry point | Same checks everywhere, no extra task runner dependency | setup |

## Log

Short entries, newest first: date, task, what changed, anything the next session must know.

- 2026-10-08: T0.1 done. Workspace with 8 library stubs and `dimo-cli` (clap 4.6.7). `cargo deny` not run locally (not installed); all locked crates checked by hand: at least 14 days old, MIT/Apache/Unicode licenses.
- 2026-10-08: Repository bootstrapped: agent instructions, conventions, M0 plan, governance files, guard hooks. No code yet.
