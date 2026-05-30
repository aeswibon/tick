# Feature guides index

Use these when you want **depth** on one capability. For a single scrollable overview, see [FEATURES.md](../FEATURES.md). For keys only, see [KEYBINDINGS.md](../KEYBINDINGS.md).

## Views and data

- **[views-and-tabs.md](views-and-tabs.md)** — Tab order (`1`–`6`), custom JQL, disk cache, Closed-tab JQL search, `assignee was` history

## Table and reading

- **[table-and-navigation.md](table-and-navigation.md)** — Row selection, filter, sort, `page_size`, column ids
- **[detail-pane.md](detail-pane.md)** — Detail layout, ADF rendering, markdown round-trip

## Jira write-back

- **[status-transitions.md](status-transitions.md)** — Workflow transitions, resolution, required fields
- **[comments-and-worklogs.md](comments-and-worklogs.md)** — Comments with `@`, worklog time formats
- **[editing-fields.md](editing-fields.md)** — Inline edits, sprint/backlog moves, watch/unwatch, due date
- **[issue-links-subtasks.md](issue-links-subtasks.md)** — Links tab, subtasks, add link (`I`)
- **[create-duplicate-templates.md](create-duplicate-templates.md)** — Create, duplicate, templates, export `X`

## Workflow extras

- **[open-and-multi-site.md](open-and-multi-site.md)** — Browser open, clipboard lookup, site errors
- **[auth-cli-cache.md](auth-cli-cache.md)** — Auth, CLI flags, refresh, desktop notify

## Example: triage morning routine

```text
tick                          # launch
1                             # Assigned tab
r                             # refresh
/ regression                  # local filter (any tab except Closed)
Enter                         # open detail
t                             # transition → In Progress
c                             # comment: "Investigating"
Esc                           # close detail
6                             # Closed tab
/ payment gateway             # JQL search in done issues
Enter                         # fetch from Jira
h                             # toggle to "ever assigned" and re-search
```
