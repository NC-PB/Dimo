#!/usr/bin/env bash
# PreToolUse guard. Enforces the hard rules from AGENTS.md that must never be broken.
# Exit code 2 blocks the tool call and shows the message to Claude.
set -euo pipefail
input=$(cat)
tool=$(jq -r '.tool_name // ""' <<<"$input")
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"

block() { echo "BLOCKED by .claude/hooks/guard.sh: $*" >&2; exit 2; }

case "$tool" in
  Edit|Write|MultiEdit)
    path=$(jq -r '.tool_input.file_path // ""' <<<"$input")
    rel="${path#"$root"/}"
    new=$(jq -r '[.tool_input.content, .tool_input.new_string, ((.tool_input.edits // [])[] | .new_string)]
                 | map(select(. != null)) | join("\n")' <<<"$input")
    case "$rel" in
      corpus/drawings/*)
        block "corpus/drawings is read only (AGENTS.md rule 12). Ask the owner to add or change drawings." ;;
      LICENSE)
        block "LICENSE holds the official Apache 2.0 text and must not change." ;;
      docs/adr/*.md)
        if [[ -f "$path" ]] && grep -qiE '^Status:[[:space:]]*\**accepted' "$path"; then
          block "Accepted ADRs are immutable. Write a new ADR that supersedes it (skill new-adr); the owner updates the old one."
        fi ;;
      data/tolerances/*)
        if grep -qE '(status[[:space:]]*=[[:space:]]*"verified"|verified_by|verified_date)' <<<"$new"; then
          block "Only the owner marks tolerance tables as verified (D-43, AGENTS.md rule 7). Keep status = \"draft\"."
        fi ;;
    esac
    ;;
  Bash)
    cmd=$(jq -r '.tool_input.command // ""' <<<"$input")
    if grep -qE '(^|[;&|[:space:]])git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+commit' <<<"$cmd"; then
      if ! grep -qE '([[:space:]]--signoff|[[:space:]]-[a-zA-Z]*s[a-zA-Z]*)([[:space:]]|$)' <<<"$cmd"; then
        block "Commits need a DCO sign-off. Use: git commit -s -m \"type(scope): message\""
      fi
    fi
    if grep -qE 'corpus/drawings' <<<"$cmd" && grep -qE '(^|[;&|[:space:]])(rm|mv|cp|sed[[:space:]]+-i|truncate|tee)([[:space:]]|$)' <<<"$cmd"; then
      block "corpus/drawings is read only (AGENTS.md rule 12)."
    fi
    if grep -qE 'git[[:space:]]+push.*(--force|[[:space:]]-f([[:space:]]|$))' <<<"$cmd"; then
      block "Force push is not allowed."
    fi
    ;;
esac
exit 0
