---
name: new-adr
description: Write a new Architecture Decision Record in docs/adr when a decision changes architecture, the tech stack, the project file format, a binding default (D-*) or supersedes an earlier ADR.
---

# New ADR

1. Find the highest number in `docs/adr/` and use the next one: `docs/adr/NNNN-short-kebab-title.md`.
2. Use exactly this structure, matching the existing ADRs:

```markdown
# ADR NNNN: Title

Status: proposed

## Context
Problem, forces, constraints. Reference requirement IDs and measurements.

## Decision
What we do. One paragraph or a few bullets.

## Consequences
- positive and negative effects
- what becomes harder
- Supersedes ADR XXXX (if any)
```

3. Status is always `proposed`. Only the owner sets `accepted`. Accepted ADRs are never edited
   (a hook blocks it); if one is superseded, tell the owner which line to change in the old file.
4. Add the decision to the decision log in `docs/plan/STATUS.md` with a link to the ADR.
5. If the ADR changes something in `docs/spec/`, update the spec in the same commit and mention it.
6. Keep it short. Plain language, no dashes as sentence breaks.
