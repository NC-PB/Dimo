# CLAUDE.md

@AGENTS.md

## Claude Code specifics

The shared instructions above apply. This section only adds what is specific to Claude Code.

### Session routine

- Start: read `docs/plan/STATUS.md`. If the user just says "continue" or "next", use the `/next-task` skill.
- Finish: use the `/finish-task` skill. It runs the checks, updates STATUS.md and prepares a signed commit.
- Use plan mode for any task that touches more than one crate or adds a dependency.
- Keep the main context lean. Use subagents for broad codebase searches and for reviews.

### Project skills (`.claude/skills/`)

| Skill | Use when |
|---|---|
| `next-task` | Picking up work: finds the next open task and plans it |
| `finish-task` | Closing a task: verify, document, commit |
| `new-adr` | A decision changes architecture, stack or a binding default |
| `tolerance-table` | Creating or editing anything in `data/tolerances/` |
| `corpus-drawing` | Adding a drawing, notes or ground truth to `corpus/` |
| `ipc-command` | Adding a Tauri command, event or tile route end to end |

### Subagents (`.claude/agents/`)

| Agent | Use when |
|---|---|
| `spec-reviewer` | Before finishing a task: checks the diff against FR/NFR/ADR/D-* |
| `clean-room-auditor` | Before a commit that adds docs, UI text, templates or test data |

### Guardrails (hooks, enforced automatically)

- Edits that set a tolerance table to `verified` are blocked (rule 7).
- Edits to `corpus/drawings/`, `LICENSE` and accepted ADRs are blocked (rule 12).
- Rust and frontend files are formatted after every edit.

If a hook blocks you, do not work around it. Explain to the user why you wanted the change.

### Personal overrides

Put machine specific notes in `CLAUDE.local.md` (git ignored).
