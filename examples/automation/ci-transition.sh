#!/usr/bin/env sh
# CI helper: transition one issue by transition name.
# Usage: ci-transition.sh ISSUE-KEY "Transition Name" [--site NAME]
# Exit 0 on success; tick stderr on failure.

set -eu

key="${1:?issue key required}"
shift
to="${1:?transition name required}"
shift

site=""
while [ $# -gt 0 ]; do
  case "$1" in
    --site)
      site="${2:?}"
      shift 2
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

if [ -n "$site" ]; then
  tick issue transition "$key" --to "$to" --site "$site"
else
  tick issue transition "$key" --to "$to"
fi
