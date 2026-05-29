# tick

A k9s-inspired Jira Cloud TUI for the terminal. Multi-site ticket dashboard with filtering, sorting, transitions, comments, worklogs, field edits, and sprint moves — without leaving the keyboard.

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
- **Five views** — Assigned, Updated, Mentions, Watched, Sprint (custom JQL each)
- **Virtualized table** — Scroll hundreds of issues at terminal height
- **Detail pane** — Summary, description, comments; edit fields in place
- **Jira write-back** — Transitions, comments, worklogs, summary, priority, labels, description, sprint/backlog
- **Auth** — API token by default; optional [OAuth 2.0](docs/OAUTH.md)
- **Offline-friendly** — Per-view disk cache with staleness indicators
- **Optional notify** — Desktop alert when refresh finds new issues
- **Themes** — Built-in + custom TOML ([`themes/`](themes/))

## Documentation

| Guide | Description |
|-------|-------------|
| **[docs/USER_GUIDE.md](docs/USER_GUIDE.md)** | Start here — setup, workflow, tips |
| **[docs/CONFIGURATION.md](docs/CONFIGURATION.md)** | Full `config.toml` reference |
| **[docs/OAUTH.md](docs/OAUTH.md)** | OAuth app setup and `tick auth` |
| **[docs/KEYBINDINGS.md](docs/KEYBINDINGS.md)** | Complete keyboard reference |
| [ROADMAP.md](ROADMAP.md) | Release history and plans |

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
tick auth status          # OAuth session info
tick --list-themes        # List themes
```

## Keybindings (summary)

| Keys | Action |
|------|--------|
| `j`/`k`, `g`/`G`, `[`/`]` | Navigate / scroll |
| `/`, `s` | Filter / sort |
| `Enter`, `h`/`l` | Detail pane / tabs |
| `t`, `c`, `w` | Transition, comment, worklog |
| `S`, `P`, `L`, `M`, `D` | Edit summary, priority, labels, sprint, description |
| `1`–`5` | View tabs |
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

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for columns, JQL, sprint fields, boards, OAuth, and notifications.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) · [GitHub Issues](https://github.com/aeswibon/tick/issues)

## License

MIT — [LICENSE](LICENSE)
