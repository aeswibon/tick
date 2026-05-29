# tick — complete feature reference

This guide documents **every** capability in tick (Jira Cloud TUI). For a shorter walkthrough, see [USER_GUIDE.md](USER_GUIDE.md).

| Doc | Purpose |
|-----|---------|
| [USER_GUIDE.md](USER_GUIDE.md) | Setup and daily workflow |
| [KEYBINDINGS.md](KEYBINDINGS.md) | Keyboard cheat sheet |
| [CONFIGURATION.md](CONFIGURATION.md) | `config.toml` options |
| [OAUTH.md](OAUTH.md) | OAuth 2.0 setup |
| [../themes/README.md](../themes/README.md) | Theme files |

---

## 1. Platform and scope

- **Jira Cloud only** (Atlassian Cloud `*.atlassian.net` sites).
- **Terminal UI** — keyboard-driven, inspired by k9s.
- **Multi-site** — multiple `[[sites]]` in one table with a **site** column.
- **Not supported:** Jira Server / Data Center (unless you request it).

---

## 2. Authentication

### API token (default)

Set `auth = "token"` or omit `auth`. Token resolution order:

1. `TICK_TOKEN` environment variable  
2. `~/.config/tick/token` (file, mode `600` recommended)  
3. `token = "..."` in `config.toml`

Verify:

```bash
tick auth status    # per-site /myself check
tick --doctor       # JQL + bulk fetch probes
```

### OAuth 2.0 (optional)

```toml
auth = "oauth"

[oauth]
client_id = "..."
redirect_uri = "http://127.0.0.1:8765/callback"
```

```bash
export TICK_OAUTH_CLIENT_SECRET="..."
tick auth login
tick auth status
tick auth logout
```

See [OAUTH.md](OAUTH.md).

---

## 3. CLI commands

| Command / flag | Description |
|----------------|-------------|
| `tick` | Launch the TUI |
| `tick --init` | Create `~/.config/tick/config.toml` template |
| `tick --doctor` | Per site: JQL search, bulk fetch, sprint field candidates, agile boards |
| `tick --debug` | Log HTTP debug lines to stderr |
| `tick --list-themes` | Print built-in and custom theme names |
| `tick --theme NAME` | Override config theme for this run |
| `tick --max-results N` | Override `max_results` (issues per site per fetch) |
| `tick --page-size N` | Override scroll step for `[` / `]` |
| `tick auth login` | OAuth browser login |
| `tick auth status` | API token source + per-site login, or OAuth session |
| `tick auth logout` | Remove `oauth.json` |

---

## 4. Main interface

```
┌─ Header: per-site counts · live/cached/offline status ──────┐
├─ Tabs: Assigned · Updated · Mentions · Watched · Sprint ─────┤
├─ Table (virtualized) or Detail pane (60% table / 40% detail) ┤
├─ (gap) ────────────────────────────────────────────────────┤
└─ Footer: hints · row/total · sort · cache age ─────────────┘
```

Press **`?`** for in-app help. Overlays (transitions, priorities, sprints, site errors, `@` picker) draw on top.

---

## 5. View tabs and JQL

| Tab | Key | Default JQL intent |
|-----|-----|-------------------|
| Assigned | `1` | Your open assignments |
| Updated | `2` | Assigned, updated in last 7 days |
| Mentions | `3` | Comment mentions of you |
| Watched | `4` | Issues you watch |
| Sprint | `5` | Your issues in open sprints |

Override with `[views]` in config — see [CONFIGURATION.md](CONFIGURATION.md#custom-jql-views).

**Per-view disk cache:** `~/.config/tick/cache/{assigned,updated,mentions,watched,sprint}.json`  
On startup, tick loads cache immediately, then refreshes in the background.

---

## 6. Table navigation and display

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Move selection |
| `g` / `G` | First / last row in **filtered** list |
| `[` / `]` | Scroll viewport by `page_size` rows |
| `Enter` | Toggle detail pane |

### Virtualization

The table renders only rows visible in the terminal height. You can set `max_results` to hundreds; scrolling stays responsive.

### Columns

Configurable via `columns = [...]` in config.

| Column id | Aliases | Content |
|-----------|---------|---------|
| `site` | | Config site name |
| `key` | | Issue key |
| `type` | `issuetype` | Issue type |
| `status` | | Status (colored) |
| `priority` | | Priority (colored) |
| `age` | `ageing` | Days since update |
| `due` | `duedate` | Due date |
| `assignee` | | Assignee display name |
| `reporter` | | Reporter |
| `parent` | `epic` | Parent key |
| `labels` | `label` | Comma-separated labels |
| `sprint` | | Sprint name (needs `sprint_field`) |
| `summary` | | Summary text |

Default set: `site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `summary`.

---

## 7. Filtering and sorting

### Filter (`/`)

Incremental filter (case-insensitive) across:

- Issue key  
- Summary  
- Status  
- Assignee  
- Reporter  
- Labels  
- Sprint name  
- Parent key  

`Enter` or `Esc` exits filter mode and resets selection to the first match.

### Sort

| Key | Action |
|-----|--------|
| `s` | Cycle field: **default** → age → priority → status → key |
| `S` | Toggle **ascending ↑** / **descending ↓** (table only; in detail pane `S` edits summary) |

- **default** — preserves JQL/API order  
- Other modes sort the filtered list; footer shows e.g. `age ↑`.

---

## 8. Refresh, cache, and connectivity

### Header status

| Label | Meaning |
|-------|---------|
| `loading` / custom message | Fetch or issue lookup in progress |
| `live · refresh Nm ago` | Last fetch for this view succeeded |
| `cached · …` | Showing disk cache; live fetch pending or failed |
| `offline · …` | All sites failed on last fetch but cached tickets remain |

### Actions

| Key | Action |
|-----|--------|
| `r` | Refresh active view from Jira now |
| (background) | Auto-refresh after startup and after each background cycle |

### Resilience

- **HTTP retry** — exponential backoff on 429, 5xx, and transient network errors.  
- **Partial failure** — per-site warnings; successful sites still contribute tickets.  
- **Failed fetch** — keeps cached tickets instead of clearing the table.  
- **Selection preserved** — after refresh, keeps the same issue key when the list is unchanged.

### Desktop notifications

```toml
notify_on_refresh = true
```

Notifies when a background or scheduled refresh finds **new** issue keys (macOS, Linux, Windows).

---

## 9. Opening issues in the browser

| Key | Context | Behavior |
|-----|---------|----------|
| `o` | Table | Open **selected** row’s Jira URL |
| `O` | Table | Open from **clipboard** or pasted key/URL |

**`O` flow:**

1. Reads clipboard; if it contains a valid key or `/browse/` URL, resolves and opens.  
2. Otherwise enters input mode — paste/type key or URL, `Enter` to open.  
3. **Multi-site:** probes each configured site via Jira API (`GET /rest/api/3/issue/{key}`); footer/header shows `Checking site (2/3)…`; opens the **first match** in config order.  
4. Full browse URLs use the host to pick the site without probing.

| Key | Action |
|-----|--------|
| `y` | Copy selected issue key to clipboard |

---

## 10. Detail pane

Open with **`Enter`** on a row. Close with **`Esc`** or **`Enter`** again.

### Tabs (`h` / `l`)

| Tab | Content |
|-----|---------|
| **Details** | Key, link, type, status, priority, dates, people, labels, sprint, parent |
| **Description** | ADF-rendered body (headings, lists, tables, mentions, media, expand) |
| **Comments** | Thread with ADF rendering |

### Read-only actions (table or detail)

| Key | Action |
|-----|--------|
| `e` | Open `config.toml` in `$EDITOR` / system default |

### Writes (detail pane)

All writes refresh views after success. Errors show in the footer.

| Key | Action | Notes |
|-----|--------|-------|
| `t` / `T` | **Change status** (workflow) | See [§10.1](#101-status-workflow-transitions) |
| `c` | Add comment | Markdown → ADF; `@` user picker |
| `w` | Log work | Jira time format, e.g. `30m`, `1h` |
| `a` | Assign to me | Uses `/myself` account id |
| `u` | Unassign | Clears assignee |
| `S` | Edit summary | Inline text |
| `P` | Change priority | Picker from site priorities |
| `L` | Set labels | Comma-separated; **replaces** all labels |
| `M` | Move sprint / backlog | Needs `board_id` / `boards` |
| `D` | Edit description | Markdown + `@`; existing ADF → markdown on open |

### @mentions

In **comments** and **description** edit:

- Type `@` to open assignable-user search for the current issue.  
- Picker: `j`/`k`, `Enter` to insert, `Esc` to close picker only.  
- Submitted as ADF mention nodes with account IDs.  
- Description and comment tabs render `@Display Name` for mentions.

### Markdown (comments & descriptions)

Supported when editing:

- `#`–`######` headings  
- `-` bullet and `1.` ordered lists  
- `- [ ]` / `- [x]` task lists (Jira checklists)  
- `**bold**`, `*italic*`, `~~strike~~`, `` `code` ``  
- `[label](url)` links  
- `>` blockquotes, `---` rules, ` ```lang ` code fences  
- `@mentions` via picker (existing mentions restored when you press `D`)  

Exotic Jira blocks round-trip via fenced **` ```adf-json`** sections (preserved verbatim on save).

### ADF display (read-only)

Rich Jira content in description/comments:

- Paragraphs, headings, lists, code blocks  
- **Mentions**, links, emoji, hard breaks  
- Tables, media attachments, expand sections  
- Unknown blocks shown with a type label  

### 10.1 Status (workflow transitions)

Jira Cloud does **not** allow setting the status field directly. tick loads your project’s **workflow transitions** and applies one via the REST API.

| Step | What happens |
|------|----------------|
| `t` or `T` | Fetches `GET /rest/api/3/issue/{key}/transitions` for the selected issue |
| Picker | Lists each transition as **action → target status** (e.g. `Start Progress → In Progress`) |
| `Enter` / `1`–`9` | `POST` with `{ "transition": { "id": "…" } }`, then refreshes the view |

**Required workflow fields** (e.g. Resolution on “Done”):

1. When you pick a transition, tick reads `transitions.fields` from Jira (`required: true`).
2. For each required field (typed from Jira’s schema):
   - **Select / resolution / priority** → picker (`j`/`k`, `Enter`, `1`–`9`).
   - **User** (assignee, etc.) → cached list (up to 100) filtered in footer; **R** refreshes from Jira.
   - **Boolean** → Yes / No picker.
   - **Date** → footer input `YYYY-MM-DD`.
   - **Date-time** → footer input `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`.
   - **Number / plain text** → footer input with validation.
3. Values are sent in the transition `POST` under `fields`.
4. If Jira still rejects the transition, tick parses `errors` (e.g. `resolution`) and prompts again for those fields.

Complex field types (rich text, multi-select components, some custom fields) may still need the Jira web UI.

**When something is missing or invalid**, tick shows a footer error instead of silently failing:

| Situation | Message |
|-----------|---------|
| No row selected | Select a ticket to change status |
| Site not in config | Unknown site … — cannot change status |
| API returns no transitions | No workflow transitions … (may be closed or your role cannot move it) |
| Jira rejects the transition | Includes Jira `errorMessages` / field `errors` (e.g. resolution required, required fields) |
| Auth / not found | HTTP context + parsed Jira body when available |

Works from the **table** or **detail pane** (same as `c` / `w`).

---

## 11. Sprint and agile

### Show sprint in table

```toml
sprint_field = "customfield_10020"   # from tick --doctor
columns = [..., "sprint", ...]
```

### Move issues (`M`)

Requires Scrum/Kanban board configuration:

```toml
board_id = 7
boards = { PROJ = 12, WEB = 99 }
```

Picker lists backlog + active/future sprints. `tick --doctor` lists boards per site.

---

## 12. Multi-site

```toml
[[sites]]
name = "acme"
base_url = "https://acme.atlassian.net"

[[sites]]
name = "corp"
base_url = "https://corp.atlassian.net"
boards = { CORP = 3 }
```

- **site** column identifies the instance.  
- Actions use the ticket’s site `base_url`.  
- **`O`** with ambiguous keys probes each site.  
- **`!`** overlay lists per-site errors when some sites fail.

---

## 13. Site errors overlay

When one or more sites fail on fetch:

- Footer shows `N site error(s) — press ! for details`.  
- **`!`** — scrollable overlay with full messages per site.  
- **`j`/`k`** scroll; **`!`** or **`Esc`** close.

---

## 14. Themes

### Built-in

`default`, `catppuccin-mocha`, `light`, `tokyo-night`, `dracula`, `gruvbox-dark`, `nord`, `one-dark`, `solarized-dark`, `rose-pine`

```bash
tick --list-themes
tick --theme dracula
```

### Custom

Copy from [`themes/`](../themes/) to `~/.config/tick/themes/<name>.toml`:

```toml
theme = "my-theme"
```

---

## 15. Files on disk

| Path | Purpose |
|------|---------|
| `~/.config/tick/config.toml` | Main configuration |
| `~/.config/tick/token` | API token file |
| `~/.config/tick/oauth.json` | OAuth tokens |
| `~/.config/tick/cache/*.json` | Per-view ticket cache |
| `~/.config/tick/themes/*.toml` | Custom themes |

Treat cache and tokens as **sensitive** (issue summaries, credentials).

---

## 16. Environment variables

| Variable | Purpose |
|----------|---------|
| `TICK_TOKEN` | API token |
| `TICK_OAUTH_CLIENT_ID` | OAuth client id |
| `TICK_OAUTH_CLIENT_SECRET` | OAuth secret |
| `TICK_OAUTH_REDIRECT_URI` | OAuth redirect |

---

## 17. Limitations and known behavior

- **Jira Cloud API only** — no Server/DC REST variants.  
- **Description edit** — exotic blocks outside markdown use ` ```adf-json` fences; edit those only if you know ADF.  
- **Transition / priority / sprint pickers** — numeric shortcuts `1`–`9` only.  
- **Windows** — clipboard/open/notify use platform tools (`clip`, `cmd`, PowerShell).  
- **Concurrent UI** — long operations show `loading` in header/footer; issue lookup shows site count when probing multiple instances.

---

## 18. Quick troubleshooting

| Problem | Check |
|---------|--------|
| Empty table | `tick auth status`, `tick --doctor`, JQL in `[views]` |
| 401 / auth | Token, email, site `base_url` |
| No sprint column | `sprint_field` from `--doctor` |
| Sprint move fails | `board_id` / `boards.PROJECT` |
| `O` can’t find issue | Issue exists on a configured site; try full browse URL |
| Stale data | Press `r`; check header `cached` vs `live` |

---

*Version: see [CHANGELOG.md](../CHANGELOG.md) and `tick --version` (from `Cargo.toml` at release).*
