# tick — feature overview

Single-page map of everything tick can do. For **step-by-step guides with examples**, use the [feature docs](features/README.md).

| Doc | Purpose |
|-----|---------|
| [USER_GUIDE.md](USER_GUIDE.md) | Setup and daily workflow |
| [KEYBINDINGS.md](KEYBINDINGS.md) | Full keyboard reference |
| [CONFIGURATION.md](CONFIGURATION.md) | `config.toml` |
| [OAUTH.md](OAUTH.md) | OAuth 2.0 |
| [features/](features/README.md) | **Per-feature guides** |

---

## Platform

- **Jira Cloud** (`*.atlassian.net`) — not Server/Data Center
- **Terminal UI** — keyboard-first (k9s-style)
- **Multi-site** — one table, `site` column

---

## Views (tabs `1`–`6`)

| Tab | Key | Guide |
|-----|-----|--------|
| Assigned | `1` | [views-and-tabs.md](features/views-and-tabs.md) |
| Mentions | `2` | |
| Watched | `3` | |
| Updated | `4` | |
| Sprint | `5` | |
| Closed (JQL search) | `6` | [views-and-tabs.md#closed-tab--search-done-tickets](features/views-and-tabs.md) |
| Custom JQL views | `7`–`9`, `v` | [saved-views-templates-columns.md](features/saved-views-templates-columns.md) |

Custom JQL: `[views]` and `[[views.custom]]` in config. Cache: `~/.config/tick/cache/`.

### v0.14 highlights

| Feature | Guide |
|---------|--------|
| Quick search (cached) | [quick-search.md](features/quick-search.md) — `Ctrl+g` |
| CLI search / bulk | [automation.md](features/automation.md) — `tick search`, `tick bulk` |

### v0.13 highlights

| Feature | Guide |
|---------|--------|
| Bulk labels | [bulk-actions.md](features/bulk-actions.md) — `L` with marks |
| Headless CLI | [automation.md](features/automation.md) — `tick issue show`, `tick issue transition` |
| Config check | `tick --check` (offline); `tick --doctor` (live) |

### v0.12 highlights

| Feature | Guide |
|---------|--------|
| Bulk table actions | [bulk-actions.md](features/bulk-actions.md) — `Space`, bulk `t` / `a` / `W` |
| Template description edit | [saved-views-templates-columns.md](features/saved-views-templates-columns.md#template-manager-shifte) — `Shift+E` → `b` |
| Template labels edit | [saved-views-templates-columns.md](features/saved-views-templates-columns.md#template-manager-shifte) — `Shift+E` → `l` |

### v0.10 highlights

| Feature | Guide |
|---------|--------|
| Template edit/delete | [saved-views-templates-columns.md](features/saved-views-templates-columns.md#template-manager-shifte) — `Shift+E` |
| Saved JQL views | `[[views.custom]]`, keys `7`–`9` |
| Closed persist + local filter | `closed_prefs.json`, `f` on Closed tab |
| Custom field columns | `customfield_*` in `columns` |

---

## Table and detail

| Area | Guide |
|------|--------|
| Filter, sort, scroll, columns | [table-and-navigation.md](features/table-and-navigation.md) |
| Detail tabs, ADF display | [detail-pane.md](features/detail-pane.md) |

---

## Jira actions

| Action | Keys | Guide |
|--------|------|--------|
| Transitions | `t` / `T` | [status-transitions.md](features/status-transitions.md) |
| Comment / worklog | `c`, `w` | [comments-and-worklogs.md](features/comments-and-worklogs.md) |
| Edit fields | `S`, `P`, `L`, `D`, `M`, `a`, `u` | [editing-fields.md](features/editing-fields.md) |
| Create / duplicate / templates | `n`, `N`, `C`, `X`, `Shift+E` | [create-duplicate-templates.md](features/create-duplicate-templates.md) |
| Open / multi-site | `o`, `O`, `y`, `!` | [open-and-multi-site.md](features/open-and-multi-site.md) |

---

## Auth, CLI, cache

[auth-cli-cache.md](features/auth-cli-cache.md) — token, OAuth, `--doctor`, refresh, `notify_on_refresh`.

## Platform (v0.11)

[platform.md](features/platform.md) — **`R`** reload config; HTTP **429** footer backoff hint.

---

## Themes

Built-in: `default`, `catppuccin-mocha`, `light`, `tokyo-night`, `dracula`, `gruvbox-dark`, `nord`, `one-dark`, `solarized-dark`, `rose-pine`.

```bash
tick --list-themes
tick --theme dracula
```

Custom: `~/.config/tick/themes/<name>.toml` — see [themes/README.md](../themes/README.md).

---

## Limitations

- Cloud REST only  
- Some transition/create custom fields need Jira UI  
- Transition pickers: numeric shortcuts `1`–`9` only  
- Description: exotic ADF via ` ```adf-json` fences  

---

## Environment

| Variable | Purpose |
|----------|---------|
| `TICK_TOKEN` | API token |
| `TICK_OAUTH_*` | OAuth overrides |

See [auth-cli-cache.md](features/auth-cli-cache.md).
