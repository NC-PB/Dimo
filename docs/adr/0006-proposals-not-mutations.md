# ADR 0006: Automation produces proposals, not data

Status: accepted

## Context
Users distrust tools that change data on their own, and recognition is never perfect.

## Decision
Recognition jobs return proposals. Proposals become characteristics only through an explicit, undoable accept command, individually, in bulk, or by an explicit auto accept rule stored in the project settings.

## Consequences
- The user is always in control and can see what the machine suggested.
- Jobs work on immutable snapshots and never conflict with edits.
- Requires a good review UI, which is a core feature anyway.
