# tick user guide

tick is a terminal UI for **Jira Cloud**. It aggregates issues from one or more sites, lets you triage with filters and sorts, and perform common actions without opening the browser.

| Doc | Use when |
|-----|----------|
| [features/README.md](features/README.md) | Deep dive on one capability (examples) |
| [FEATURES.md](FEATURES.md) | One-page map of all features |
| [KEYBINDINGS.md](KEYBINDINGS.md) | Every key, by context |
| [CONFIGURATION.md](CONFIGURATION.md) | `config.toml` options |

## First-time setup

```bash
tick --init
```

Edit `~/.config/tick/config.toml`:

```toml
email = "you@example.com"
max_results = 50
page_size = 10

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
```

### Credentials (API token — default)

tick uses **API token** authentication unless you set `auth = "oauth"` in config. Choose **one** token source:

1. `export TICK_TOKEN="your-api-token"` ([create token](https://id.atlassian.com/manage-profile/security/api-tokens))
2. `echo 'token' > ~/.config/tick/token && chmod 600 ~/.config/tick/token`
3. `token = "..."` in config (least preferred)

Verify connectivity:

```bash
tick auth status   # confirms API token (default) or OAuth session
tick --doctor      # JQL + sprint/board probes per site
```

Launch:

```bash
tick
```

For OAuth instead of API tokens, see [OAUTH.md](OAUTH.md).

## Interface overview

```
┌─ Header: site counts · live/cached status ─────────────────┐
├─ Tabs: Assigned · Mentions · Watched · Updated · Sprint · Closed ┤
├─ Ticket table (virtualized — uses full terminal height) ────┤
├─ Footer: key hints · row/total · sort · cache age ──────────┤
└─────────────────────────────────────────────────────────────┘
```

Press **`Enter`** on a row to open the **detail pane** (table shrinks to 60%, details on the right).

## View tabs

| Tab | Key | Default JQL focus |
|-----|-----|-------------------|
| Assigned | `1` | Your open assignments |
| Mentions | `2` | Issues where you are mentioned in comments |
| Watched | `3` | Issues you watch |
| Updated | `4` | Assigned, updated last 7 days |
| Sprint | `5` | Your work in open sprints |
| Closed | `6` | Done issues — press `/` to search (`h` = ever-assigned history) |

Override any view with `[views]` in config — see [CONFIGURATION.md](CONFIGURATION.md#custom-jql-views).

Each view is **cached** under `~/.config/tick/cache/`. On startup, tick loads cache immediately, then refreshes open tabs in the background (Closed is fetched when you search).

## Filtering and sorting

- **`/`** — incremental filter across key, summary, status, assignee, reporter, labels, sprint, parent
- **`s`** — cycle sort modes (default preserves JQL order, or sort by age, priority, status, key)
- **`S`** — toggle sort ascending ↑ / descending ↓ (table only)

## Refresh and offline behavior

| Indicator | Meaning |
|-----------|---------|
| `live · refresh Nm ago` | Last fetch succeeded for active view |
| `cached · Nh ago` | Showing disk cache; fetch pending or partial failure |
| `offline · Nh ago` | All sites failed but cached tickets are still shown |

- **`r`** — refresh active view now
- Background refresh runs after startup and repeats after each cycle
- **`notify_on_refresh = true`** — desktop notification when new issues appear (macOS/Linux/Windows)

If Jira is unreachable, tick **keeps the last cached tickets** instead of clearing the table.

## Detail pane workflow

1. **`Enter`** on a ticket
2. Use **`h`/`l`** for Details / Description / Comments tabs
3. Common actions:

| Goal | Key |
|------|-----|
| Change status | `t` → pick transition |
| Comment | `c` — type `@` to tag users (picker); mentions render in Description/Comments |
| Log work | `w` |
| Assign to yourself | `a` |
| Unassign | `u` |
| Edit title | `S` |
| Change priority | `P` |
| Set labels | `L` (comma-separated) |
| Move to sprint/backlog | `M` |
| Edit description | `D` (markdown: `#` headings, `-` lists, `**bold**`, `@` mentions) |
| Open in browser | `o` (selected row) |
| Open by key/clipboard | `O` (multi-site: checks each instance) |
| Copy key | `y` |

After writes, tick refreshes all views.

## Sprint column and moves

**Display** (optional):

```toml
sprint_field = "customfield_10020"   # tick --doctor lists candidates
columns = ["site", "key", "sprint", "summary", "status"]
```

**Moves** require a Scrum/Kanban board:

```toml
board_id = 7
boards = { PROJ = 12 }   # per-project override
```

Use **`M`** in the detail pane. Run **`tick --doctor`** to list boards and sprint fields.

## Multi-site

Add multiple `[[sites]]` blocks. The **site** column shows which instance each issue belongs to. Site name in config must match ticket `site` for actions.

## Themes

```bash
tick --list-themes
tick --theme dracula
```

See [themes/README.md](../themes/README.md) and [CONFIGURATION.md](CONFIGURATION.md#themes).

## CLI reference

```bash
tick                      # TUI
tick --init               # Create default config
tick --doctor             # Test JQL, bulk fetch, sprint fields, boards
tick --max-results 200    # Override fetch limit
tick --page-size 25       # Scroll step for [ ]
tick auth status          # API token + OAuth login status (per site)
tick auth login           # OAuth only (see OAUTH.md)
```

## Tips

1. **Raise `max_results`** (e.g. `100`–`500`) if you have many open issues; the table virtualizes rendering.
2. Use **custom JQL** per view to match your team’s workflow.
3. Run **`tick --doctor`** after config changes for sprint/board fields.
4. Keep **`~/.config/tick/token`** or **`oauth.json`** permissions tight (`chmod 600`).
5. Press **`?`** anytime for in-app keybindings.

## Getting help

- [FEATURES.md](FEATURES.md) — complete feature reference
- [KEYBINDINGS.md](KEYBINDINGS.md) — full key list
- [CONFIGURATION.md](CONFIGURATION.md) — all options
- [GitHub Issues](https://github.com/aeswibon/tick/issues)
