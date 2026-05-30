#!/usr/bin/env sh
# Example [[hooks.on_config_reload]] handler (after R in the TUI).
# Logs validation findings from tick --check-style rules.

set -eu

: "${TICK_CONFIG_PATH:?TICK_CONFIG_PATH not set}"
: "${TICK_JSON_PATH:?TICK_JSON_PATH not set}"

errors="${TICK_CHECK_ERRORS:-0}"
warns="${TICK_CHECK_WARNS:-0}"

echo "[tick config] reloaded ${TICK_CONFIG_PATH}: ${errors} error(s), ${warns} warning(s)"

if command -v jq >/dev/null 2>&1; then
  jq -r '.[] | "  [\(.level)] \(.message)"' "$TICK_JSON_PATH" 2>/dev/null || true
fi

if [ "$errors" != "0" ] && [ -n "$errors" ]; then
  exit 1
fi
