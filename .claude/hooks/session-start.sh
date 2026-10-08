#!/usr/bin/env bash
# SessionStart: puts the current status into context so every session starts oriented.
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" || exit 0
echo "## Dimo session context"
if [[ -f docs/plan/STATUS.md ]]; then
  awk '/^## Current/{p=1} /^## Tasks/{p=0} p' docs/plan/STATUS.md
fi
echo "### git"
git branch --show-current 2>/dev/null
git status --short 2>/dev/null | head -20
git log --oneline -5 2>/dev/null
exit 0
