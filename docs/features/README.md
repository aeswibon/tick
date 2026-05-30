# Feature guides index

Use these when you want **depth** on one capability. For a single scrollable overview, see [FEATURES.md](../FEATURES.md). For keys only, see [KEYBINDINGS.md](../KEYBINDINGS.md).

## Views and data

- **[views-and-tabs.md](views-and-tabs.md)** — Tab order (`1`–`6`), custom JQL, disk cache, Closed-tab JQL search, `assignee was` history
- **[saved-views-templates-columns.md](saved-views-templates-columns.md)** — Custom JQL tabs (`7`–`9`), template manager (`Shift+E`), Closed persist, custom columns

## Table and reading

- **[table-and-navigation.md](table-and-navigation.md)** — Row selection, filter, sort, `page_size`, column ids
- **[bulk-actions.md](bulk-actions.md)** — Multi-select (`Space`), bulk transition, assign, labels, watch
- **[automation.md](automation.md)** — Headless CLI (`tick issue`, `tick search`, `tick bulk`, `tick --check`)
- **[quick-search.md](quick-search.md)** — Search cached views (`g`)
- **[detail-pane.md](detail-pane.md)** — Detail layout, ADF rendering, markdown round-trip

## Jira write-back

- **[status-transitions.md](status-transitions.md)** — Workflow transitions, resolution, required fields
- **[comments-and-worklogs.md](comments-and-worklogs.md)** — Comments with `@`, worklog time formats
- **[editing-fields.md](editing-fields.md)** — Inline edits, sprint/backlog moves, watch/unwatch, due date
- **[issue-links-subtasks.md](issue-links-subtasks.md)** — Links tab, subtasks, add link (`I`)
- **[create-duplicate-templates.md](create-duplicate-templates.md)** — Create, duplicate, templates, export `X`, manage `Shift+E`

## Workflow extras

- **[open-and-multi-site.md](open-and-multi-site.md)** — Browser open, clipboard lookup, site errors
- **[auth-cli-cache.md](auth-cli-cache.md)** — Auth, CLI flags, refresh, desktop notify
- **[platform.md](platform.md)** — Config reload (`R`), Jira 429 footer UX

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
