#!/usr/bin/env bash
# Claude Code PostToolUse hook: format, lint and test after a Rust file changes.
# Exit 2 sends the errors back to the agent.
f=$(jq -r '.tool_input.file_path // .tool_response.filePath // empty')
case "$f" in *.rs) ;; *) exit 0 ;; esac
cd "$(dirname "$0")/.." || exit 0
cargo fmt --all 2>/dev/null
if ! out=$(cargo clippy --all-targets -q --message-format short -- -D warnings 2>&1); then
  echo "$out" | grep -E '^(error|warning)|: (error|warning)' | head -40 >&2
  exit 2
fi
if ! out=$(cargo test -q 2>&1); then
  echo "$out" | tail -40 >&2
  exit 2
fi
