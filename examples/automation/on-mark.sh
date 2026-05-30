#!/usr/bin/env sh
# Example [[hooks.on_mark]] handler (Space marks a row for bulk).
# Replace echo with your notifier or script.

set -eu

: "${TICK_KEY:?TICK_KEY not set}"
: "${TICK_SITE:?TICK_SITE not set}"

summary=""
if command -v jq >/dev/null 2>&1 && [ -n "${TICK_JSON_PATH:-}" ] && [ -f "$TICK_JSON_PATH" ]; then
  summary=$(jq -r '.summary // ""' "$TICK_JSON_PATH" 2>/dev/null || true)
fi

echo "[tick mark] ${TICK_SITE}/${TICK_KEY} ${summary}"
