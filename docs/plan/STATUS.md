# Status

Single place for progress. Agents update this file at the end of every task.

## Current

- Milestone: **M0 Foundations** ([plan](M0.md))
- Next task: **T0.3 Typed IPC with tauri-specta**
- Release target: 0.1 after M3

## Tasks M0

| Task | State | Notes |
|---|---|---|
| T0.1 Cargo workspace skeleton | done | Dependency direction enforced by `crates/dimo-cli/tests/dependency_direction.rs` |
| T0.2 Tauri 2 shell with Svelte 5 | review | Owner: confirm the decisions marked "owner" in the log below |
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
| 2026-10-08 | TypeScript 6.0.3 instead of 7.x | svelte-check 4.7 and typescript-eslint 8.70 support TypeScript only up to 6.0 | T0.2, agent |
| 2026-10-08 | pnpm 11.27.1 instead of 12.x | corepack 0.34 (bundled with Node 22.20) cannot start pnpm 12 (`bin/pnpm.mjs`) | T0.2, agent |
| 2026-10-08 | Tauri app identifier `io.github.nc-pb.dimo` | Determines the app data folder, hard to change after release. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | Theme follows the OS only. The settings switch of D-51 comes with the Settings view | Settings view has no content yet. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | UI language: stored in localStorage, else OS language, else English. Switching reloads the window | Paraglide default, every message rerenders without per message reactivity | T0.2, agent |
| 2026-10-08 | Paraglide message format plugin loaded from `node_modules`, not from a CDN | Offline, reproducible builds (rule 9). Package has no license field, its LICENSE file is MIT | T0.2, agent |
| 2026-10-08 | Accepted build time npm licenses outside rule 10 list: MPL-2.0 (`lightningcss`, used by Tailwind and Vite), unlabeled `@lix-js/sdk-<platform>` binaries (repository `opral/lix` is MIT, used by Paraglide at build time), BlueOak-1.0.0 (`minimatch`), 0BSD (`tslib`). None of the MPL or unlabeled code ships in the app bundle | Needed for the T0.4 npm license check. **Owner: confirm** | T0.2, agent |
| 2026-10-08 | `deny.toml` ignores six "unmaintained" advisories (proc-macro-error, unic-*) that come through Tauri 2.11, no upgrade path | No vulnerability, only maintenance status. Remove when Tauri drops them | T0.2, agent |
| 2026-10-08 | Cargo deps checked for the 14 day rule with `scripts/check-lock-age.sh`; tauri sibling crates pinned to the 2.11.6 release set | Cargo has no stable min-publish-age yet; tauri 2.11.6 breaks with tauri-runtime 2.12 | T0.2, agent |
| 2026-10-08 | Internal dependency allow list: notation, pdf, vision, io, report may use core; tolerance may use core and notation; detect as in rust.md. Widen only with a log entry | rust.md leaves these edges open; core is a pure leaf, so no cycles | T0.1, agent |
| 2026-10-08 | `unwrap`/`expect`/`print_stdout`/`print_stderr` warn workspace wide, allowed in tests via `clippy.toml`; `PDFium` is a valid doc identifier | Enforces rust.md rules mechanically | T0.1, agent |
| 2026-10-08 | Code repository at `~/Repositories/dimo` (public `github.com/NC-PB/dimo`). Spec, ADRs and corpus were copied from the planning folder `project_moonshot`; from now on this repository is the source of truth | One self contained repo for agents and contributors | owner, setup |
| 2026-10-08 | pnpm (via corepack) as package manager for the frontend | Fast, strict, standard with Tauri; spec left it open | setup, owner may change |
| 2026-10-08 | `scripts/check.sh` as the single local and CI entry point | Same checks everywhere, no extra task runner dependency | setup |

## Log

Short entries, newest first: date, task, what changed, anything the next session must know.

- 2026-10-08: T0.2 in review. Tauri shell with Svelte 5, Tailwind 4 tokens, Bits UI, Paraglide (en, de), view store. `tauri dev` opened the native window (checked by window capture). View and language switch tested in the browser pane against the same dev server; clicking inside the native window was not possible with the available tools.
- 2026-10-08: T0.1 done. Workspace with 8 library stubs and `dimo-cli` (clap 4.6.7). `cargo deny check` passes (wildcard paths allowed for unpublished internal crates); all locked crates are at least 14 days old.
- 2026-10-08: Repository bootstrapped: agent instructions, conventions, M0 plan, governance files, guard hooks. No code yet.
