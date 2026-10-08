# Workflow

## Branches and commits

- `main` is always green. Work on short lived branches `feat/<topic>`, `fix/<topic>` when using PRs;
  direct commits to `main` are acceptable while the owner is the only maintainer.
- Conventional Commits, signed off: `git commit -s`. Types: feat, fix, perf, refactor, test,
  docs, build, ci, chore. Scope is the crate or area.
- One logical change per commit. Requirement IDs in the body when relevant.

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
