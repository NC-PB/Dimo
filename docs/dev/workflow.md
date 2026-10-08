# Workflow

## Branches and commits

- `main` is always green. Work on short lived branches `feat/<topic>`, `fix/<topic>` when using PRs;
  direct commits to `main` are acceptable while the owner is the only maintainer.
- Conventional Commits, signed off: `git commit -s`. Types: feat, fix, perf, refactor, test,
  docs, build, ci, chore. Scope is the crate or area.
- One logical change per commit. Requirement IDs in the body when relevant.

## CI

`.github/workflows/ci.yml` runs on every push and pull request:

- `check`: `./scripts/check.sh` on Ubuntu, macOS and Windows (fmt, clippy, tests, `cargo deny`,
  frontend check, lint, npm licenses, tests).
- `npm-licenses`: `node scripts/check-npm-licenses.mjs`. The allow list mirrors `deny.toml`; the
  accepted build time exceptions are listed in the script and in the STATUS decision log.
- `dco`: on pull requests, `./scripts/check-dco.sh <base> <head>` fails if a commit lacks
  `Signed-off-by`. Run it locally with `./scripts/check-dco.sh origin/main`.

Actions are pinned to full commit SHAs of releases at least 14 days old. Release workflows come
with 0.1.

## Task lifecycle

1. Task is listed in `docs/plan/M<n>.md` with acceptance criteria.
2. Mark it `in progress` in `docs/plan/STATUS.md`.
3. Plan, implement, test, verify (see AGENTS.md "How to work").
4. Tick the acceptance criteria, set the task `done` in STATUS.md, add notes and decisions.
5. Commit.

## Milestones

A milestone is done when every task is done and its exit criteria from
[09 Roadmap](../spec/09-roadmap.md) are measured and recorded. The agent then drafts
`docs/plan/M<n+1>.md` from the roadmap and requirements, and the owner reviews it before work starts.

## Spikes

Throwaway experiments go to `spikes/<name>/` with a README stating the question and the answer.
Spikes are never imported by crates. Findings move into docs or code, then the spike can be deleted.
