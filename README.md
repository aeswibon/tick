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
- **View tabs** — Assigned, Updated (7d), Mentions, Watched, Sprint (open sprints)
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
git clone https://github.com/aeswibon/tick.git
cd tick
cargo build --release
cp target/release/tick /usr/local/bin/
```

Requires Rust 2021 edition.

### Releases

Download prebuilt binaries from [GitHub Releases](https://github.com/aeswibon/tick/releases).

### Homebrew

Homebrew looks for a tap at **`github.com/aeswibon/homebrew-tick`** (not this repo):

```bash
brew tap aeswibon/tick
brew install tick
```

**Without tapping:** install the formula attached to a [GitHub release](https://github.com/aeswibon/tick/releases):

```bash
brew install https://github.com/aeswibon/tick/releases/download/v0.1.0/tick.rb
```

This repo’s `Formula/tick.rb` is a template; release CI publishes `tick.rb` with real checksums on each tag.

## Configuration

### Quick setup

```bash
tick --init
```

Edit the generated file at `~/.config/tick/config.toml`:

```toml
email = "you@example.com"
# token in config.toml, or ~/.config/tick/token, or TICK_TOKEN env
max_results = 50
page_size = 10
theme = "default"

# Optional custom JQL per view
# [views]
# assigned = "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"

# Optional: customize table columns (default: site, key, type, status, priority, age, due, assignee, reporter)
# columns = ["site", "key", "parent", "summary", "status", "assignee"]

[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
```

| Option | Default | Description |
|--------|---------|-------------|
| `email` | — | Your Atlassian account email |
| `token` | file / env | [Atlassian API token](https://id.atlassian.com/manage-profile/security/api-tokens); also `~/.config/tick/token` or `TICK_TOKEN` |
| `[views]` | built-in JQL | Override JQL for Assigned, Updated, Mentions, Watched, Sprint (see examples) |
| `sprint_field` | — | Per-site Jira field id for sprint column (see `tick --doctor`) |
| `board_id` | — | Default agile board for sprint moves (`M`; see `tick --doctor`) |
| `boards` | — | Per-project board overrides, e.g. `{ DEMO = 7 }` |
| `max_results` | `50` | Max tickets to fetch per site |
| `page_size` | `10` | Rows to scroll with `[` / `]` (table fills terminal height) |
| `notify_on_refresh` | `false` | Desktop alert when a refresh finds new issues in the active view |
| `theme` | `"default"` | Theme name (built-in or custom) |
| `columns` | built-in default | Table column ids (see config comment) |
| `sites` | — | List of Jira sites with `name` and `base_url` |

Column ids: `site`, `key`, `type`, `status`, `priority`, `age`, `due`, `assignee`, `reporter`, `parent`, `labels`, `sprint`, `summary`.

### Credentials

Provide a token via **one** of: `TICK_TOKEN` env, `~/.config/tick/token`, or `token` in `config.toml`. Use a dedicated [Atlassian API token](https://id.atlassian.com/manage-profile/security/api-tokens) with minimal scope and restrict file permissions (`chmod 600` on the token file).

### Themes

Built-in: `default`, `light`, `tokyo-night`, `dracula`. Example TOML files are in [`themes/`](themes/).

```bash
tick --list-themes        # built-in + ~/.config/tick/themes/*.toml
tick --theme dracula
```

Set `theme = "tokyo-night"` in `config.toml`, or copy a file from `themes/` to `~/.config/tick/themes/` and customize.

## Usage

```bash
tick                     # Launch the TUI
tick --theme light       # Light theme
tick --theme dracula     # Dracula theme
tick --theme mycustom    # Load ~/.config/tick/themes/mycustom.toml
tick --max-results 100   # Override max results
tick --page-size 20      # Override scroll step for [ / ]
tick --list-themes       # List built-in and custom themes
tick --debug             # Print API debug info to stderr
tick --doctor            # Test API connectivity and exit
tick --init              # Create default config file
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Navigate up / down |
| `g` / `G` | Go to first / last row |
| `[` / `]` | Scroll up / down by `page_size` rows |
| `Enter` | Toggle detail pane |
| `Esc` | Close pane / help / overlay |
| `?` | Toggle help |
| `/` | Filter tickets |
| `s` | Cycle sort mode (default → age → priority → status → key) |
| `r` | Refresh tickets |
| `y` | Copy ticket key to clipboard |
| `o` | Open ticket in browser |
| `e` | Open config in editor |
| `t` | Transition ticket status (j/k in picker) |
| `c` | Add comment (detail pane open) |
| `w` | Log work time (detail pane open) |
| `a` / `u` | Assign to me / unassign (detail pane open) |
| `S` / `P` / `L` / `M` | Edit summary / priority / labels / move sprint (detail open) |
| `!` | Toggle site error overlay |
| `h` / `l` | Previous / next detail tab (Details → Description → Comments) |
| `←` / `→` or `Tab` / `Shift+Tab` | Cycle view tab |
| `1`–`5` | Jump to Assigned / Updated / Mentions / Watched / Sprint |
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

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned releases and priorities.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports and feature requests: [GitHub Issues](https://github.com/aeswibon/tick/issues).

## License

MIT — see [LICENSE](LICENSE).

## Project structure

```
src/
  main.rs           CLI entry, doctor, TUI bootstrap
  input.rs          Keyboard handling
  app.rs            State, cache, views, filter cache
  api/              Jira REST client + ADF helpers
  ui/               draw, table, detail, help, transitions, ADF
  config.rs         Config load/validate
  columns.rs        Configurable table columns
  platform.rs       Cross-platform open / clipboard
  fetch_status.rs   Site warnings vs action errors
  ticket_lock.rs    RwLock helpers (poison-safe)
  theme.rs          Themes
```

## Tech stack

- Rust, ratatui, crossterm — TUI
- reqwest, serde — HTTP + JSON
- tokio — async runtime
- clap — CLI
- toml — config
