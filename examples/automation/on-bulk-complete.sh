#!/usr/bin/env sh
# Example [[hooks.on_bulk_complete]] handler.
# Logs summary; exits 1 if any issue failed (optional — remove exit for notify-only).

set -eu

: "${TICK_JSON_PATH:?TICK_JSON_PATH not set}"
: "${TICK_BULK_LABEL:?TICK_BULK_LABEL not set}"

label="${TICK_BULK_LABEL}"
ok="${TICK_OK_COUNT:-0}"
fail="${TICK_FAIL_COUNT:-0}"

echo "[tick bulk] ${label}: ${ok} ok, ${fail} failed"

if command -v jq >/dev/null 2>&1; then
  jq -r '.failed[]? | "  \(.key): \(.error)"' "$TICK_JSON_PATH" 2>/dev/null || true
fi

if [ "${fail}" != "0" ] && [ "${fail}" != "" ]; then
  exit 1
fi
