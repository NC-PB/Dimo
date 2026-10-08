---
name: next-task
description: Pick up the next open Dimo task from docs/plan/STATUS.md, load the spec it cites, and produce an implementation plan. Use when the user says "continue", "next", "next task" or starts a work session without a specific request.
---

# Next task

1. Read `docs/plan/STATUS.md`. Find the first task in state `in progress`, otherwise the first `todo`
   whose predecessors are `done` or `review`. Skip `blocked` tasks and mention them.
2. Read the task section in `docs/plan/M<n>.md`, including its acceptance criteria.
3. Read every spec, ADR and convention file the task cites under "Refs". Read only the relevant
   sections of long files. If the task touches code that exists, explore it with a subagent
   instead of reading many files into the main context.
4. Write a plan and show it to the user:
   - files to create or change
   - public types and functions with signatures
   - dependencies to add (name, exact version, license, release date at least 14 days ago)
   - tests that prove each acceptance criterion
   - risks and open points, with the default you will take for each
5. Set the task to `in progress` in STATUS.md.
6. If the user approves or already asked you to proceed, implement in small steps, running the
   narrowest relevant tests after each step. Finish with the `finish-task` skill.

Do not ask questions the spec answers. If the milestone has no open task left, propose drafting the
next milestone file from `docs/spec/09-roadmap.md` and `docs/spec/03-functional-requirements.md`.
