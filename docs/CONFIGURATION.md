# Configuration reference

Config file: `~/.config/tick/config.toml`

Generate a template: `tick --init`

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

Example:

```toml
[[sites]]
name = "acme"
base_url = "https://acme.atlassian.net"
sprint_field = "customfield_10020"
board_id = 4
boards = { MOBILE = 12 }
```

## Custom JQL (`[views]`)

| Key | Default tab |
|-----|-------------|
| `assigned` | Assigned (`1`) |
| `updated` | Updated (`2`) |
| `mentions` | Mentions (`3`) |
| `watched` | Watched (`4`) |
| `sprint` | Sprint (`5`) |

Example:

```toml
[views]
assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
sprint = "sprint in openSprints() AND assignee = currentUser() ORDER BY rank"
```

## Table columns

Default: `site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `summary`

All column ids:

`site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `parent`, `labels`, `sprint`, `summary`

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

Per-view JSON cache: `~/.config/tick/cache/{assigned,updated,mentions,watched,sprint}.json`

Each file stores `fetched_at` and tickets for offline startup.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `TICK_TOKEN` | API token |
| `TICK_OAUTH_CLIENT_ID` | OAuth client id |
| `TICK_OAUTH_CLIENT_SECRET` | OAuth client secret |
| `TICK_OAUTH_REDIRECT_URI` | OAuth callback URL |

## Validation rules

- `email` non-empty
- `page_size` 1–500
- At least one `[[sites]]` with `https://` base URL
- OAuth: `oauth.json` must exist after `tick auth login`

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
