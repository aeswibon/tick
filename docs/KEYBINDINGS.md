# Keybindings reference

Complete keyboard map for tick. Keys are **case-sensitive** (`S` ≠ `s`).

**In-app help:** press **`?`** in the TUI.

For workflows and examples, see the [feature guides](features/README.md).

---

## Global — table focused (detail closed)

| Key | Action | Notes |
|-----|--------|-------|
| `j` / `k` | Previous / next row | Moves within **filtered** list |
| `↑` / `↓` | Same as `j` / `k` | |
| `g` / `G` | First / last row | Filtered list |
| `[` / `]` | Page scroll | Step = `page_size` in config (default 10) |
| `Enter` | Toggle detail pane | |
| `Esc` | Close help, detail, overlays, cancel input | |
| `?` | Help overlay | |
| `/` | **Filter** table (tabs 1–5) | Substring match; see below |
| `/` | **JQL search** (tab **Closed** only) | `Enter` fetches from Jira |
| `s` | Cycle sort field | default → age → priority → status → key |
| `S` | Toggle sort ↑ / ↓ | Table only |
| `r` | Refresh active view | Closed: needs prior search |
| `y` | Copy issue key | Clipboard |
| `o` | Open selected in browser | |
| `O` | Open from clipboard or key | Multi-site: probes API |
| `e` | Open config file | `$EDITOR` / system default |
| `t` / `T` | Workflow transition picker | Not status field directly |
| `!` | Site errors overlay | When warnings exist |
| `←` / `→` | Previous / next tab | |
| `Tab` / `Shift+Tab` | Next / previous tab | |
| `1` | **Assigned** tab | |
| `2` | **Mentions** tab | |
| `3` | **Watched** tab | |
| `4` | **Updated** tab | |
| `5` | **Sprint** tab | |
| `6` | **Closed** tab | On-demand search |
| `h` | **Closed tab only:** toggle assignee / ever-assigned | Re-runs search if query set |
| `n` | New issue wizard | |
| `N` | New from template | |
| `C` | Duplicate selected | |
| `X` | Export selection as template | |
| `q` | Quit | |

### Local filter (`/` on tabs 1–5)

Matches (case-insensitive): key, summary, status, assignee, reporter, labels, sprint, parent.

```text
/ blocked
Enter    # lock filter, jump to first match
Esc      # exit filter mode
```

### Closed search (`/` on tab 6)

```text
6
/ refund api
Enter    # JQL: ... AND text ~ "refund api" ...
h        # toggle assignee was currentUser()
r        # repeat search
```

---

## Global — detail pane open

| Key | Action | Notes |
|-----|--------|-------|
| `h` / `l` | Prev / next detail tab | Details · Description · Comments |
| `Enter` / `Esc` | Close detail | |
| `c` | Add comment | `@` mention picker |
| `w` | Log work | e.g. `30m`, `1h` |
| `a` | Assign to me | |
| `u` | Unassign | |
| `S` | Edit summary | **Not** table sort |
| `P` | Priority picker | |
| `L` | Labels (comma-separated) | Replaces all labels |
| `M` | Sprint / backlog picker | Needs `board_id` |
| `D` | Edit description | Markdown + `@` |
| `t` / `T` | Transitions | Same as table |

---

## Overlays — pickers (status, priority, sprint, create)

| Key | Action |
|-----|--------|
| `j` / `k` | Move selection |
| `Enter` | Confirm |
| `1`–`9` | Pick by number |
| `Esc` | Cancel |

Status labels look like: `Start Progress → In Progress`.

---

## Overlays — transition required fields

| Field type | Keys |
|------------|------|
| Picker (resolution, etc.) | `j`/`k`, `Enter`, `1`–`9` |
| User | Type in footer to filter; `j`/`k` pick; **⌘R** (macOS) or **Ctrl+R** load more users |
| Boolean | Yes / No picker |
| Date / text / number | Type in footer, `Enter` submit |

Plain **`r`** / **`R`** in user footer = type letter, **not** load more.

---

## Overlays — @ mention picker (comment / description)

| Key | Action |
|-----|--------|
| `j` / `k` | Move in user list |
| `Enter` | Insert mention |
| `Esc` | Close picker only |
| Continue typing | After closing picker |

---

## Input modes (footer)

Active after `c`, `w`, `S`, `L`, `D`, `O`, create wizard, template name, Closed search, transition text fields.

| Key | Action |
|-----|--------|
| Characters | Append to buffer |
| `Backspace` | Delete |
| `Enter` | Submit |
| `Esc` | Cancel |

| Mode | Started by |
|------|------------|
| Comment | `c` |
| Worklog | `w` |
| Summary | `S` (detail) |
| Labels | `L` |
| Description | `D` |
| Open ticket | `O` (no clipboard match) |
| Template export name | `X` wizard step 3 |
| Closed search | `/` on tab 6 |
| Create summary / fields | `n` / `N` / `C` wizard |

---

## Create wizard (`n`, `N`, `C`)

| Key | Action |
|-----|--------|
| `j` / `k` | Move in site/project/type/template pickers |
| `Enter` | Confirm pick |
| `p` | Re-open project picker |
| `t` | Re-open issue type picker |
| `Esc` | Cancel entire wizard |

---

## Template export wizard (`X`)

| Step | Keys |
|------|------|
| Fields to include | `Space` toggle, `j`/`k`, `Enter` next |
| Clear values | `Space` on included fields, `Enter` next |
| Template name | Type name, `Enter` save |

---

## Site errors overlay

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll messages |
| `!` or `Esc` | Close |

---

## Key conflicts (intentional)

| Key | Table | Detail | Closed tab | Create wizard |
|-----|-------|--------|------------|---------------|
| `S` | Sort direction | Edit summary | — | — |
| `h` | — | Prev tab | History scope | — |
| `t` | Transitions | Transitions | — | Issue type picker |
| `/` | Local filter | Local filter | JQL search | — |

---

## Platform notes

- **macOS:** ⌘R works for “load more users” when the terminal reports Command (keyboard enhancement flags enabled).
- **Windows:** Clipboard and notifications use OS tools; same key map.
