# Table and navigation

The main screen is a **virtualized** ticket table: only visible rows are rendered, so large `max_results` stays fast.

## Moving the selection

| Key | Action |
|-----|--------|
| `j` / `k` | Previous / next row in the **filtered** list |
| `↑` / `↓` | Same as `j` / `k` |
| `g` | Jump to **first** filtered row |
| `G` | Jump to **last** filtered row |
| `[` | Scroll viewport up by `page_size` rows |
| `]` | Scroll viewport down by `page_size` rows |
| `Enter` | Open or close **detail pane** |

### Example

```text
page_size = 15   # in config.toml
]                # scroll down 15 rows without moving selection edge cases
j j j            # move selection three rows down
```

Footer shows `row/total` and sort mode when applicable.

## Local filter (`/`)

**On tabs 1–5 only.** Case-insensitive substring match across:

- Issue key  
- Summary  
- Status  
- Assignee  
- Reporter  
- Labels  
- Sprint name (if column configured)  
- Parent key  

### Example

```text
/ PROD-          # keys starting with PROD-
/ john           # assignee or reporter containing "john"
/ blocked        # summary or labels
Enter            # exit filter mode, selection at first match
Esc              # exit filter, keep filter text until you clear with backspace in filter mode
```

**Closed tab (`6`):** `/` runs a **Jira search** instead — see [views-and-tabs.md](views-and-tabs.md#closed-tab--search-done-tickets).

## Sort

| Key | Context | Action |
|-----|---------|--------|
| `s` | Table (detail closed) | Cycle sort field |
| `S` | Table (detail closed) | Toggle ascending ↑ / descending ↓ |
| `S` | Detail open | Edit **summary** (not sort) |

Sort fields cycle: **default** → **age** → **priority** → **status** → **key** → default.

- **default** — keeps JQL/API order (recommended for Assigned tab).
- Other modes sort the **filtered** list client-side.

### Example

```text
s s s            # sort by key
S                # flip to descending
```

Footer: `key ↓` when not default.

## Columns

```toml
columns = ["site", "key", "labels", "sprint", "summary", "status", "assignee"]
```

| Column id | Aliases | Shows |
|-----------|---------|--------|
| `site` | | Config site name |
| `key` | | Issue key |
| `type` | `issuetype` | Issue type |
| `status` | | Status (themed color) |
| `priority` | | Priority (themed color) |
| `age` | `ageing` | Days since last update |
| `due` | `duedate` | Due date |
| `assignee` | | Assignee name |
| `reporter` | | Reporter |
| `parent` | `epic` | Parent issue key |
| `labels` | `label` | Labels |
| `sprint` | | Sprint name (needs `sprint_field`) |
| `summary` | | Summary |

Sprint column requires:

```toml
sprint_field = "customfield_10020"   # from tick --doctor
```

## Refresh

| Key | Action |
|-----|--------|
| `r` | Fetch active view from Jira now |

On **Closed**, `r` needs a prior search (`/` + `Enter`); otherwise the footer prompts you to search first.

## Related

- [detail-pane.md](detail-pane.md)
- [CONFIGURATION.md](../CONFIGURATION.md)
