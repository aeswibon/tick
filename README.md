# tick

A k9s-inspired Jira Cloud TUI for the terminal. Multi-site ticket dashboard with filtering, sorting, transitions, comments, worklogs, field edits, sprint moves, create/duplicate, and templates — without leaving the keyboard.

```
████████╗██╗ ██████╗██╗  ██╗
╚══██╔══╝██║██╔════╝██║ ██╔╝
   ██║   ██║██║     █████╔╝
   ██║   ██║██║     ██╔═██╗
   ██║   ██║╚██████╗██║  ██╗
   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝
```

## Features

- **Multi-site** — Several Atlassian Cloud instances in one table
- **Six views** — Assigned, Mentions, Watched, Updated, Sprint, Closed (JQL search)
- **Virtualized table** — Scroll hundreds of issues at terminal height
- **Detail pane** — Summary, description, comments; edit fields in place
- **Jira write-back** — Transitions, comments, worklogs, summary, priority, labels, description, sprint/backlog
- **Create & templates** — `n` / `N` / `C` / `X` (export template from ticket)
- **Auth** — API token by default; optional [OAuth 2.0](docs/OAUTH.md)
- **Offline-friendly** — Per-view disk cache with staleness indicators
- **Optional notify** — Desktop alert when refresh finds new issues
- **Themes** — Built-in + custom TOML ([`themes/`](themes/))

## Documentation

| Guide | Description |
|-------|-------------|
| **[docs/USER_GUIDE.md](docs/USER_GUIDE.md)** | Start here — setup, workflow, tips |
| **[docs/features/](docs/features/README.md)** | **Per-feature guides** with examples |
| **[docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)** | Complete keyboard reference |
| **[docs/CONFIGURATION.md](docs/CONFIGURATION.md)** | Full `config.toml` reference |
| **[docs/FEATURES.md](docs/FEATURES.md)** | One-page feature map |
| **[docs/OAUTH.md](docs/OAUTH.md)** | OAuth app setup and `tick auth` |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [ROADMAP.md](ROADMAP.md) | Plans |

## Quick start

```bash
tick --init
# Add token: TICK_TOKEN, ~/.config/tick/token, or config.toml — see docs/USER_GUIDE.md
tick --doctor
tick
```

## Installation

### From source

```bash
git clone https://github.com/aeswibon/tick.git
cd tick
cargo build --release
cp target/release/tick /usr/local/bin/
```

### Releases / Homebrew

- [GitHub Releases](https://github.com/aeswibon/tick/releases)
- `brew tap aeswibon/tick && brew install tick` — see [releases](https://github.com/aeswibon/tick/releases) for the formula

## CLI

```bash
tick                      # Launch TUI
tick --init               # Create ~/.config/tick/config.toml
tick --doctor             # Test API, sprint fields, agile boards
tick auth login           # OAuth browser login
tick auth status          # Auth summary
tick --list-themes        # List themes
```

## Keybindings (summary)

| Keys | Action |
|------|--------|
| `j`/`k`, `g`/`G`, `[`/`]` | Navigate / scroll |
| `/`, `s`, `S` | Filter / sort field / sort direction |
| `o`, `O` | Open selected / open from clipboard or key |
| `n`, `N`, `C`, `X` | New / template / duplicate / export template |
| `Enter`, `h`/`l` | Detail pane / tabs |
| `t`/`T`, `c`, `w` | Status (workflow), comment, worklog |
| `S`, `P`, `L`, `M`, `D` | Edit summary, priority, labels, sprint, description |
| `1`–`6` | View tabs (see docs) |
| `?` | Help |

Full list: [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)

## Configuration (minimal)

```toml
email = "you@example.com"
max_results = 50

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for columns, JQL, sprint fields, boards, OAuth, templates, and notifications.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) · [GitHub Issues](https://github.com/aeswibon/tick/issues)

## License

MIT — [LICENSE](LICENSE)
