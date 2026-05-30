# Configuration reference

Config file: `~/.config/tick/config.toml`

Generate a template: `tick --init`

Validate structure (offline): `tick --check`  
Probe live Jira APIs: `tick --doctor`

**Examples by feature:** [features/](features/README.md) · **Keys:** [KEYBINDINGS.md](KEYBINDINGS.md)

## Top-level options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `email` | string | required | Atlassian account email |
| `token` | string | — | API token (see [Credentials](#credentials)) |
| `auth` | `"token"` \| `"oauth"` | `token` | Authentication method |
| `max_results` | integer | `50` | Max issues per site per view fetch |
| `page_size` | integer | `10` | Rows to scroll with `[` / `]` |
| `theme` | string | `"default"` | Theme name |
| `notify_on_refresh` | bool | `false` | Desktop notify on new issues |
| `columns` | array | built-in | Table column ids (see below) |

CLI overrides: `--max-results`, `--page-size`, `--theme`

## Credentials

### API token (`auth = "token"` or omitted)

Priority:

1. `TICK_TOKEN` environment variable
2. `~/.config/tick/token` file
3. `token` in config.toml

### OAuth (`auth = "oauth"`)

See [OAUTH.md](OAUTH.md). Requires `tick auth login` and `[oauth]` section.

```toml
auth = "oauth"

[oauth]
client_id = "..."
redirect_uri = "http://127.0.0.1:8765/callback"
```

Plus `TICK_OAUTH_CLIENT_SECRET` in the environment.

## Sites (`[[sites]]`)

| Key | Required | Description |
|-----|----------|-------------|
| `name` | yes | Short label (shown in UI and `site` column) |
| `base_url` | yes | `https://your-domain.atlassian.net` |
| `sprint_field` | no | Jira field id for sprint **column** (`tick --doctor`) |
| `board_id` | no | Default agile **board** for sprint **moves** (`M`) |
| `boards` | no | Per-project board map, e.g. `{ DEMO = 7, WEB = 12 }` |
| `link_types` | no | Jira link type names for add-link (`I`); defaults to Cloud standard names |

Example:

```toml
[[sites]]
name = "acme"
base_url = "https://acme.atlassian.net"
sprint_field = "customfield_10020"
board_id = 4
boards = { MOBILE = 12 }

# Per-site link type names (optional)
[[sites]]
name = "zeta"
base_url = "https://zeta-tm.atlassian.net"
link_types = { relates = "Relates", blocks = "Blocks", blocked_by = "Blocks", epic = "Epic-Story Link" }
```

## Custom JQL (`[views]`)

| Key | Default tab |
|-----|-------------|
| `assigned` | Assigned (`1`) |
| `mentions` | Mentions (`2`) |
| `watched` | Watched (`3`) |
| `updated` | Updated (`4`) |
| `sprint` | Sprint (`5`) |
| `closed` | Closed (`6`) — base JQL without search text |
| `closed_history` | Closed tab when `h` toggles to ever-assigned (`assignee was`) |

Example:

```toml
[views]
assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY rank"
```

## Saved JQL views (`[[views.custom]]`)

Extra tabs beyond `1`–`6`. See [saved-views-templates-columns.md](features/saved-views-templates-columns.md).

```toml
[[views.custom]]
name = "My open bugs"
jql = "project = ENG AND assignee = currentUser() ORDER BY updated DESC"
key = 7

[[views.custom]]
name = "Zeta backlog"
jql = "project = HIN AND status = Backlog ORDER BY rank"
site = "zeta"
key = 8
```

| Field | Description |
|-------|-------------|
| `name` | Tab label |
| `jql` | Full JQL query |
| `site` | Optional: only query this `[[sites]].name` |
| `key` | Optional: `7`, `8`, or `9` (auto-assigned if omitted) |

## Table columns

Default: `site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `summary`

Built-in column ids:

`site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `parent`, `labels`, `sprint`, `summary`

**Custom fields (read-only):** any `customfield_*` id Jira returns on bulk fetch:

```toml
columns = ["site", "key", "customfield_10042", "summary", "status", "assignee"]
```

```toml
columns = ["site", "key", "labels", "sprint", "summary", "status", "assignee"]
```

## Themes

Built-in: `default`, `catppuccin-mocha`, `light`, `tokyo-night`, `dracula`, `gruvbox-dark`, `nord`, `one-dark`, `solarized-dark`, `rose-pine`

Custom: `~/.config/tick/themes/<name>.toml` — copy from [themes/](../themes/)

```toml
theme = "dracula"
```

## Cache

Per-view JSON cache: `~/.config/tick/cache/{assigned,updated,mentions,watched,sprint,closed,custom-*}.json`

Each file stores `fetched_at` and tickets for offline startup.

Closed tab preferences: `~/.config/tick/cache/closed_prefs.json` (`query`, `ever_assigned`).

## Environment variables

| Variable | Purpose |
|----------|---------|
| `TICK_TOKEN` | API token |
| `TICK_OAUTH_CLIENT_ID` | OAuth client id |
| `TICK_OAUTH_CLIENT_SECRET` | OAuth client secret |
| `TICK_OAUTH_REDIRECT_URI` | OAuth callback URL |

## Reloading config in the TUI

After editing `config.toml` (or `e` to open it in your editor), press **`R`** in tick to reload without restarting. tick re-reads sites, views, templates, columns, theme, and auth, then shows a notice — press **`r`** to refresh Jira data.

## Validation rules

- `email` non-empty
- `page_size` 1–500
- At least one `[[sites]]` with `https://` base URL
- OAuth: `oauth.json` must exist after `tick auth login`

## Refresh hooks

Run shell commands after a successful refresh of the **active** view (manual `r` or when a background refresh updates the tab you are on). Not run when Jira returns errors for that fetch.

```toml
[[hooks.on_refresh]]
command = "jq length \"$TICK_JSON_PATH\" -r"   # example; use a script path in practice
views = ["assigned"]                            # optional
timeout_secs = 30                               # optional, default 30
```

| Variable | Meaning |
|----------|---------|
| `TICK_VIEW` | `assigned`, `mentions`, `watched`, `updated`, `sprint`, `closed`, or custom view `name` |
| `TICK_JSON_PATH` | Temp file: JSON array of `{ key, site, summary, status, assignee, labels, url }` |
| `TICK_ISSUE_COUNT` | Issue count |

See [features/automation.md](features/automation.md#refresh-hooks).

### Bulk-complete hooks

```toml
[[hooks.on_bulk_complete]]
command = "~/.local/bin/on-tick-bulk.sh"
timeout_secs = 30
```

| Variable | Meaning |
|----------|---------|
| `TICK_BULK_LABEL` | e.g. `Bulk assign`, `Bulk labels` |
| `TICK_JSON_PATH` | Temp file: `{ label, ok, failed: [{ key, error }] }` |
| `TICK_OK_COUNT` | Success count |
| `TICK_FAIL_COUNT` | Failure count |

Fires after TUI bulk table actions and `tick bulk` (including partial failures). See [features/automation.md](features/automation.md#bulk-complete-hooks).

### Config-reload hooks

```toml
[[hooks.on_config_reload]]
command = "~/.local/bin/on-tick-config-reload.sh"
```

| Variable | Meaning |
|----------|---------|
| `TICK_CONFIG_PATH` | Path to `config.toml` |
| `TICK_JSON_PATH` | Temp file: `[{ "level", "message" }, ...]` from `tick --check`-style validation |
| `TICK_CHECK_ERRORS` | Error count |
| `TICK_CHECK_WARNS` | Warning count |

Runs after **`R`** reload succeeds. See [features/automation.md](features/automation.md#config-reload-hooks).

### Mark hooks

```toml
[[hooks.on_mark]]
command = "~/.local/bin/on-tick-mark.sh"
```

| Variable | Meaning |
|----------|---------|
| `TICK_KEY` | Issue key |
| `TICK_SITE` | Site name |
| `TICK_JSON_PATH` | Temp file: single issue `{ key, site, summary, status, assignee, labels, url }` |

Runs when **Space** adds a bulk mark (not unmark or mark-all). See [features/automation.md](features/automation.md#mark-hooks).

### Editable custom fields

```toml
[[detail.editable_fields]]
id = "customfield_10042"
label = "Story points"
type = "text"          # text | select | user

[[detail.editable_fields]]
id = "customfield_10001"
label = "Environment"
type = "select"
options = ["Dev", "Staging", "Prod"]

[[detail.editable_fields]]
id = "customfield_10002"
label = "Reviewer"
type = "user"
```

Press **`F`** in the detail pane to edit. See [features/custom-fields.md](features/custom-fields.md).

## Example full config

```toml
email = "you@example.com"
max_results = 100
page_size = 15
theme = "tokyo-night"
notify_on_refresh = true

[views]
assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY rank"

[[sites]]
name = "engineering"
base_url = "https://engineering.atlassian.net"
sprint_field = "customfield_10020"
board_id = 7
boards = { ENG = 7, WEB = 12 }

columns = ["site", "key", "sprint", "summary", "status", "priority", "assignee"]
```
