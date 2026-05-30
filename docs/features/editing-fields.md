# Editing issue fields

Most edits require the **detail pane** open. After success, tick refreshes views; failures show in the footer.

## Summary (`S`)

| Key | Action |
|-----|--------|
| `S` | Footer input — new summary |
| `Enter` | Save |
| `Esc` | Cancel |

**Table vs detail:** `S` sorts the table when detail is **closed**; edits summary when detail is **open**.

## Priority (`P`)

| Key | Action |
|-----|--------|
| `P` | Open priority picker |
| `j`/`k`, `Enter`, `1`–`9` | Select |
| `Esc` | Cancel |

## Labels (`L`)

Replaces **all** labels on the issue (not merge).

```text
L
bug, triage, team-platform
Enter
```

Comma-separated, trimmed. Empty string clears labels (if Jira allows).

## Description (`D`)

| Key | Action |
|-----|--------|
| `D` | Edit description as **markdown** in footer (multi-line) |
| `@` | User mention picker (same as comments) |
| `Enter` | Save (markdown → ADF) |

Existing ADF is converted to markdown when you open edit; prior `@mentions` are restored for save.

### Preserve exotic Jira blocks

Blocks tick cannot edit as markdown appear as:

````markdown
```adf-json
{ ... raw ADF ... }
```
````

Leave those fences intact unless you know ADF.

## Sprint and backlog (`M`)

Requires agile board config — see `tick --doctor`.

```toml
[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
board_id = 7
boards = { ENG = 12, WEB = 99 }
```

| Key | Action |
|-----|--------|
| `M` | Picker: Backlog + sprints for the issue’s project |
| `j`/`k`, `Enter` | Select destination |

## Assign / unassign

| Key | Action |
|-----|--------|
| `a` | Assign to **you** (`/myself` account id) |
| `u` | Unassign |

## Due date (`d`)

Detail pane only.

| Key | Action |
|-----|--------|
| `d` | Edit due date — `YYYY-MM-DD` in footer |
| `Enter` | Save (empty input clears due date) |
| `Esc` | Cancel |

## Watch / unwatch

Works from the **table** or **detail pane**.

| Key | Action |
|-----|--------|
| `W` | Add yourself as watcher |
| `Shift+W` | Remove yourself as watcher |

## Related

- [status-transitions.md](status-transitions.md)
- [CONFIGURATION.md](../CONFIGURATION.md) — `sprint_field`, boards
