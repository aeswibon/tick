# tick

A k9s-inspired Jira TUI for the terminal. Shows your open tickets across multiple Atlassian sites in a real-time dashboard with filtering, sorting, transitions, comments, and work logging.

```
████████╗██╗ ██████╗██╗  ██╗
╚══██╔══╝██║██╔════╝██║ ██╔╝
   ██║   ██║██║     █████╔╝
   ██║   ██║██║     ██╔═██╗
   ██║   ██║╚██████╗██║  ██╗
   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝
```

## Features

- **Multi-site** — Combine tickets from multiple Atlassian instances in one view
- **Live TUI** — Auto-refresh every 3 hours, manual refresh with `r`
- **View tabs** — Assigned, Updated (7d), Mentions, Watched
- **Filter & sort** — Filter by any field with `/`, cycle sort modes with `s`
- **Detail pane** — Press `Enter` for split-screen detail with 3 tabs (Details, Description, Comments)
- **Jira write-back** — Transition status (`t`), add comments (`c`), log work (`w`)
- **Vim keys** — `j`/`k` navigation, `g`/`G` jump to first/last page
- **Themes** — Built-in dark, light, tokyo-night, dracula + custom TOML themes
- **Configurable columns** — Customize the ticket table via `config.toml`
- **Cross-platform** — Browser, clipboard, and config editor helpers on macOS, Linux, and Windows

## Installation

### From source

```bash
git clone https://github.com/YOUR_ORG/tick.git
cd tick
cargo build --release
cp target/release/tick /usr/local/bin/
```

Requires Rust 2021 edition.

### Releases

Download prebuilt binaries from [GitHub Releases](https://github.com/YOUR_ORG/tick/releases), or install via Homebrew after a release is published.

## Configuration

### Quick setup

```bash
tick --init
```

Edit the generated file at `~/.config/tick/config.toml`:

```toml
email = "you@example.com"
token = "your-atlassian-api-token"
max_results = 50
theme = "default"

# Optional: customize table columns (default: site, key, type, status, priority, age, due, assignee, reporter)
# columns = ["site", "key", "summary", "status", "assignee"]

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
```

| Option | Default | Description |
|--------|---------|-------------|
| `email` | — | Your Atlassian account email |
| `token` | — | [Atlassian API token](https://id.atlassian.com/manage-profile/security/api-tokens) |
| `max_results` | `50` | Max tickets to fetch per site |
| `theme` | `"default"` | Theme name (built-in or custom) |
| `columns` | built-in default | Table column ids (see config comment) |
| `sites` | — | List of Jira sites with `name` and `base_url` |

Column ids: `site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `summary`.

### Theme from config

Set `theme = "tokyo-night"` in `config.toml`. The CLI flag `--theme dracula` overrides it.

## Usage

```bash
tick                     # Launch the TUI
tick --theme light       # Light theme
tick --theme dracula     # Dracula theme
tick --theme mycustom    # Load ~/.config/tick/themes/mycustom.toml
tick --max-results 100   # Override max results
tick --debug             # Print API debug info to stderr
tick --doctor            # Test API connectivity and exit
tick --init              # Create default config file
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Navigate up / down |
| `g` / `G` | Go to first / last page |
| `[` / `]` | Previous / next page |
| `Enter` | Toggle detail pane |
| `Esc` | Close pane / help / overlay |
| `?` | Toggle help |
| `/` | Filter tickets |
| `s` | Cycle sort mode (default → age → priority → status → key) |
| `r` | Refresh tickets |
| `y` | Copy ticket key to clipboard |
| `o` | Open ticket in browser |
| `e` | Open config in editor |
| `t` | Transition ticket status |
| `c` | Add comment (detail pane open) |
| `w` | Log work time (detail pane open) |
| `h` / `l` | Switch detail tab |
| `←` / `→` or `Tab` / `Shift+Tab` | Cycle view tab |
| `1` / `2` / `3` / `4` | Jump to Assigned / Updated / Mentions / Watched |
| `q` | Quit |

## Themes

### Built-in themes

```
tick --theme light
tick --theme tokyo-night
tick --theme dracula
```

### Custom themes

Create `~/.config/tick/themes/mycustom.toml` (see existing README theme keys). Use with `--theme mycustom` or `theme = "mycustom"` in config.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports and feature requests: [GitHub Issues](https://github.com/YOUR_ORG/tick/issues).

## License

MIT — see [LICENSE](LICENSE).

## Project structure

```
src/
  main.rs           Entry point, CLI args, TUI event loop
  app.rs            App state (tickets, filter, sort, views, cache)
  config.rs         Config loading from ~/.config/tick/config.toml
  columns.rs        Configurable table columns
  platform.rs       Cross-platform open / clipboard
  fetch_status.rs   Site warnings vs action errors
  theme.rs          Theme struct, built-in themes, TOML loading
  api/              Jira REST client
  ui/               Table, detail, help, transitions, ADF
```

## Tech stack

- Rust, ratatui, crossterm — TUI
- reqwest, serde — HTTP + JSON
- tokio — async runtime
- clap — CLI
- toml — config
