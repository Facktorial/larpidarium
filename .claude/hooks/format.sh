#!/usr/bin/env bash
# Claude Code PostToolUse hook: format the file Claude just edited.
# Receives the tool call as JSON on stdin. Never blocks the edit: formatting errors are ignored,
# clippy and `just ci` catch real problems later.
set -uo pipefail

file=$(jq -r '.tool_input.file_path // empty')
[[ -n "$file" && -f "$file" ]] || exit 0

case "$file" in
  *.rs)   rustfmt --edition 2024 --quiet "$file" 2>/dev/null ;;
  *.toml) taplo fmt "$file" >/dev/null 2>&1 ;;
esac
exit 0
