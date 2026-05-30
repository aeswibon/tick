#!/usr/bin/env sh
# Cron example: print open issues assigned to currentUser().
# Usage: cron-assignee-report.sh [--site NAME]
# Requires: tick on PATH, jq, config.toml for the site.

set -eu

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

jql='assignee = currentUser() AND resolution = Unresolved ORDER BY updated DESC'

if [ -n "$site" ]; then
  json=$(tick search --jql "$jql" --site "$site")
else
  json=$(tick search --jql "$jql")
fi

count=$(echo "$json" | jq '.issues | length')
echo "Open issues ($(date +%Y-%m-%d)): $count"
echo "$json" | jq -r '.issues[] | "  \(.key)  \(.summary)  [\(.status // "?")]"'
