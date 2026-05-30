# Quick search across cached views

Press **`Ctrl+g`** to search issues already loaded in tick’s view caches (built-in tabs and custom JQL views on disk). This does **not** call Jira for new data.

## Flow

1. **`Ctrl+g`** — open search (footer prompt).
2. Type key, summary, or label fragment (case-insensitive).
3. **`j` / `k`** — move in the result list (max 50).
4. **`Enter`** — jump to that issue (switches tab/view if needed).
5. **`Esc`** — cancel.

## What is searched

- Tickets in each built-in view cache (Assigned, Mentions, Watched, Updated, Sprint).
- Tickets in each custom `[[views.custom]]` cache.
- The current table (deduplicated by site + key).

For a **live Jira query**, use the Closed tab (`/`) or a custom view, or `tick search --jql '...'` from the shell.
