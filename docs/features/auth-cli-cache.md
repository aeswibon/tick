# Authentication, CLI, cache, and notifications

## API token (default)

Resolution order:

1. `TICK_TOKEN` environment variable  
2. `~/.config/tick/token` (file mode `600` recommended)  
3. `token = "..."` in `config.toml`

```bash
tick auth status    # token source + per-site /myself
tick --doctor       # JQL, bulk fetch, sprint fields, boards
```

## OAuth (optional)

See [OAUTH.md](../OAUTH.md).

```toml
auth = "oauth"
[oauth]
client_id = "..."
```

```bash
export TICK_OAUTH_CLIENT_SECRET="..."
tick auth login
tick auth logout
```

## CLI flags

| Flag / command | Purpose |
|----------------|---------|
| `tick` | Launch TUI |
| `tick --init` | Write default `config.toml` |
| `tick --doctor` | Connectivity and field discovery per site |
| `tick --debug` | HTTP debug on stderr |
| `tick --list-themes` | List themes |
| `tick --theme NAME` | Override theme for one run |
| `tick --max-results N` | Cap issues per site per fetch |
| `tick --page-size N` | `[` / `]` scroll step |
| `tick auth login` | OAuth |
| `tick auth status` | Auth summary |
| `tick auth logout` | Remove OAuth tokens |

## Header status labels

| Label | Meaning |
|-------|---------|
| `loading` / custom | Fetch or multi-site probe in progress |
| `live · refresh Nm ago` | Last fetch for this view succeeded |
| `cached · …` | Disk cache shown; live refresh pending or failed |
| `offline · …` | All sites failed but cached rows remain |

## Refresh

| When | What |
|------|------|
| Startup | Load cache → background refresh tabs 1–5 |
| `r` | Refresh active view now |
| ~3 hours | Auto refresh while TUI runs |
| After edits | Views refresh after successful write |

**Closed tab:** only refreshes when you run a search (`/` + `Enter`) or `r` after a search.

## Desktop notifications

```toml
notify_on_refresh = true
```

Alerts when a background refresh finds **new** issue keys (macOS, Linux, Windows).

## Files on disk

| Path | Purpose |
|------|---------|
| `~/.config/tick/config.toml` | Main config |
| `~/.config/tick/token` | API token |
| `~/.config/tick/oauth.json` | OAuth tokens |
| `~/.config/tick/cache/*.json` | Per-view caches |
| `~/.config/tick/themes/*.toml` | Custom themes |

Treat cache and tokens as **sensitive**.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `TICK_TOKEN` | API token |
| `TICK_OAUTH_CLIENT_ID` | OAuth client id |
| `TICK_OAUTH_CLIENT_SECRET` | OAuth secret |
| `TICK_OAUTH_REDIRECT_URI` | OAuth redirect |

## Related

- [CONFIGURATION.md](../CONFIGURATION.md)
- [USER_GUIDE.md](../USER_GUIDE.md)
