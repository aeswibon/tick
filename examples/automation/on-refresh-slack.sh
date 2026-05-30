#!/usr/bin/env sh
# Example [[hooks.on_refresh]] handler.
# Replace the echo with curl to Slack, ntfy, etc.

set -eu

: "${TICK_VIEW:?TICK_VIEW not set}"
: "${TICK_ISSUE_COUNT:?TICK_ISSUE_COUNT not set}"

echo "[tick refresh] view=${TICK_VIEW} issues=${TICK_ISSUE_COUNT}"

if command -v jq >/dev/null 2>&1 && [ -n "${TICK_JSON_PATH:-}" ] && [ -f "$TICK_JSON_PATH" ]; then
  jq -r '.[0:3][]? | "  \(.key): \(.summary)"' "$TICK_JSON_PATH" 2>/dev/null || true
fi
