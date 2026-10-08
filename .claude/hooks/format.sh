#!/usr/bin/env bash
# PostToolUse formatter. Never fails the tool call.
input=$(cat)
path=$(jq -r '.tool_input.file_path // ""' <<<"$input")
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
[[ -f "$path" ]] || exit 0
case "$path" in
  *.rs)
    command -v rustfmt >/dev/null 2>&1 && rustfmt --edition 2024 --config-path "$root/rustfmt.toml" "$path" >/dev/null 2>&1 ;;
  *.ts|*.js|*.svelte|*.css|*.json)
    if [[ -x "$root/node_modules/.bin/prettier" ]]; then
      "$root/node_modules/.bin/prettier" --write --log-level silent "$path" >/dev/null 2>&1
    fi ;;
esac
exit 0
