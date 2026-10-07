#!/bin/sh
# Claude Code post-edit hook: after a Rust file changes, type-check the CLI and show
# diagnostics only if there are any. Exit code 2 hands them back to Claude.
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in *.rs | */Cargo.toml) ;; *) exit 0 ;; esac
cd "$CLAUDE_PROJECT_DIR" || exit 0
out=$(cargo check --message-format=short 2>&1) && exit 0
echo "$out" | head -40 >&2
exit 2
