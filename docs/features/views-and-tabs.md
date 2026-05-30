# Views and tabs

tick organizes tickets into **six views** (tabs). Each view runs a Jira **JQL** query per configured site, merges results into one table, and caches them on disk.

## Tab order and keys

| Key | Tab | Default intent |
|-----|-----|----------------|
| `1` | **Assigned** | Open work assigned to you |
| `2` | **Mentions** | Open issues where you are mentioned in comments |
| `3` | **Watched** | Open issues you watch |
| `4` | **Updated** | Your assignments updated in the last 7 days |
| `5` | **Sprint** | Your issues in open sprints |
| `6` | **Closed** | Done issues — **search on demand** (not auto-fetched) |

Also: **`←` / `→`** or **`Tab` / `Shift+Tab`** cycle tabs when the detail pane is closed.

### Default JQL (overridable)

```jql
-- Assigned (1)
assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC

-- Mentions (2)
comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC

-- Watched (3)
watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC

-- Updated (4)
assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC

-- Sprint (5)
sprint in openSprints() AND assignee = currentUser() ORDER BY updated DESC
```

## Custom JQL in config

```toml
[views]
assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
mentions = "comment ~ currentUser() AND statusCategory != Done ORDER BY updated DESC"
watched = "watcher = currentUser() AND statusCategory != Done ORDER BY updated DESC"
updated = "assignee = currentUser() AND statusCategory != Done AND updated >= -7d ORDER BY updated DESC"
sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY rank"
```

See [CONFIGURATION.md](../CONFIGURATION.md#custom-jql-views).

## Cache behavior

| Path | Content |
|------|---------|
| `~/.config/tick/cache/assigned.json` | Last Assigned fetch |
| `.../mentions.json` | Mentions |
| `.../watched.json` | Watched |
| `.../updated.json` | Updated |
| `.../sprint.json` | Sprint |
| `.../closed.json` | Last Closed search |

- **Startup:** tick loads the active tab’s cache immediately, then refreshes tabs `1`–`5` in the background.
- **Closed (`6`):** not prefetched; empty until you search.
- **Header:** `live` vs `cached` vs `offline` — see [auth-cli-cache.md](auth-cli-cache.md).

## Closed tab — search done tickets

Use when you need **resolved** work (regression lookup, audit, copy from old tickets).

### Workflow

1. Press **`6`** (Closed tab).
2. Press **`/`** — footer becomes a JQL search prompt (not the local table filter).
3. Type words, e.g. `payment refund`.
4. Press **`Enter`** — tick queries Jira and fills the table.

### Scope toggle (`h` on Closed tab, table only)

| Mode | JQL assignee clause | Meaning |
|------|---------------------|---------|
| Default | `assignee = currentUser()` | You were assignee when the issue was **done** |
| After **`h`** | `assignee was currentUser()` | You were **ever** assignee (history) |

Footer shows which mode is active. If you already ran a search, **`h`** toggles scope and **re-fetches**.

### Example JQL tick runs

Search `payment`, default scope:

```jql
assignee = currentUser() AND statusCategory = Done AND text ~ "payment" ORDER BY updated DESC
```

Same search, ever-assigned (`h`):

```jql
assignee was currentUser() AND statusCategory = Done AND text ~ "payment" ORDER BY updated DESC
```

### Config overrides (bases only)

Do not put `text ~` or `ORDER BY` in config — tick adds those from your search.

```toml
[views]
closed = "assignee = currentUser() AND statusCategory = Done"
closed_history = "assignee was currentUser() AND statusCategory = Done"
```

### Keys on Closed tab

| Key | Action |
|-----|--------|
| `/` | Edit search text (`Enter` to fetch) |
| `h` | Toggle assignee vs ever-assigned |
| `r` | Re-run last search |
| `j`/`k`, `g`/`G` | Navigate results (after fetch) |

**Note:** On tabs `1`–`5`, **`/`** is the **local** filter (key, summary, labels, etc.). Only on **Closed** does **`/`** trigger server-side JQL search.

## Related

- [table-and-navigation.md](table-and-navigation.md) — local filter on non-Closed tabs
- [KEYBINDINGS.md](../KEYBINDINGS.md) — full key map
