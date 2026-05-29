# Changelog

All notable changes to this project are documented in this file.

## [0.6.3] - 2026-05-29

### Added

- Sort direction: `S` toggles ascending ↑ / descending ↓ (with `s` for sort field)

## [0.6.2] - 2026-05-29

### Changed

- `tick auth status` reports API token login (source, per-site verification) as well as OAuth; works without OAuth configured

## [0.6.1] - 2026-05-29

### Added

- ADF → markdown when opening description edit (`D`) — headings, lists, tables, mentions preserved
- Richer ADF display: tables, media attachments, expand sections, unknown block labels

### Changed

- Upgraded `ratatui` to 0.30 (transitive `lru` 0.16 — addresses Dependabot advisory)

## [0.6.0] - 2026-05-29

### Added

- HTTP retry with exponential backoff on 429/5xx and transient network errors
- Markdown descriptions: headings, bullet lists, **bold**, *italic*, `code`, links, `@mentions`
- `@` user picker when editing descriptions (`D`), same as comments

### Changed

- `refresh_all` after writes preserves selection/scroll when the ticket list is unchanged
- Background refresh keeps the selected issue key when possible
- Comments submitted as markdown ADF

## [0.5.1] - 2026-05-29

### Added

- Six built-in themes: `gruvbox-dark`, `nord`, `one-dark`, `solarized-dark`, `rose-pine`, `catppuccin-mocha`
- `@mention` rendering in detail pane; comment user picker (`@` while composing)
- Expanded tests: themes, ADF mentions, user search, comment ADF, auth default

### Changed

- API token remains the default auth method (documented in `--init` template)

## [0.5.0] - 2026-05-29

### Added

- Atlassian OAuth 2.0: `tick auth login`, `auth status`, `auth logout` and `auth = "oauth"` in config
- Edit issue description from detail pane (`D`) — plain text saved as ADF
- Documentation: [docs/USER_GUIDE.md](docs/USER_GUIDE.md), [CONFIGURATION.md](docs/CONFIGURATION.md), [OAUTH.md](docs/OAUTH.md), [KEYBINDINGS.md](docs/KEYBINDINGS.md)

### Changed

- Jira HTTP client uses unified auth (API token or OAuth bearer)

## [0.4.4] - 2026-05-29

### Added

- Fifth view tab **Sprint** (`5` / `←` `→`) with default JQL `sprint in openSprints() AND assignee = currentUser()`
- Custom sprint JQL via `[views] sprint` in config

### Changed

- View refreshes run in parallel via `JoinSet` (scales with tab count)

## [0.4.3] - 2026-05-29

### Added

- Per-site `board_id` and per-project `boards` map for sprint moves (`M`)
- `tick --doctor` lists agile boards and marks configured entries

## [0.4.2] - 2026-05-29

### Added

- Move issue to sprint or backlog from detail pane (`M`) via Jira Agile API
- Footer shows cache age when viewing stale data (e.g. `Sort: default · 2h ago`)

## [0.4.1] - 2026-05-29

### Added

- Edit labels from detail pane (`L`) — comma-separated list, replaces all labels on the issue
- Header shows cache age when viewing offline data (e.g. `cached · 2h ago`)

## [0.4.0] - 2026-05-29

### Added

- Virtualized table: scroll through full filtered list using terminal height (no page cap)
- `tick --list-themes` and `themes/` gallery with example TOML for each built-in theme

### Changed

- `page_size` now controls `[` / `]` scroll distance (not rows per page)
- Footer shows `row/total` instead of page numbers

## [0.3.2] - 2026-05-29

### Added

- Optional desktop notifications when background or scheduled refresh finds new issues (`notify_on_refresh = true`)
- Background refresh re-runs after each completed fetch cycle

## [0.3.1] - 2026-05-29

### Added

- Labels column and `/` filter match on label text
- Optional sprint column via per-site `sprint_field` in config
- `tick --doctor` lists sprint field candidates for each site
- Sprint view JQL examples in config template and README

## [0.3.0] - 2026-05-29

### Added

- Edit issue summary from detail pane (`S`)
- Change priority via picker overlay (`P`)

## [0.2.0] - 2026-05-29

### Added

- Configurable `page_size` (table rows per page) and `--page-size` CLI flag

- Custom JQL per view via `[views]` in `config.toml`
- API token from `TICK_TOKEN` or `~/.config/tick/token`
- Scrollable site-error overlay (`!` key)
- Stale/cached indicator in header when showing disk cache
- Assign to me (`a`) and unassign (`u`) from detail pane
- Parent/epic column (`parent` column id)
- Vim-style j/k navigation in transition picker
- Windows release binary (`tick-x86_64-pc-windows-msvc.exe`)
- CI: `cargo fmt` and `cargo clippy`
- Mock HTTP integration test for JQL → bulk fetch pipeline
- `SECURITY.md` and this changelog

### Changed

- `ViewMode` moved to `view_mode` module; JQL resolved through `Config::jql_for`
- Footer shows site-error count instead of a single truncated line
- `lib.rs` + `cache` module; background refresh preserves page when ticket set unchanged
- Failed fetch keeps cached tickets instead of clearing the table

## [0.1.0] - 2026-05-29

Initial public release: multi-site Jira Cloud TUI, views, detail pane, transitions, comments, worklogs, themes, Homebrew formula, and release binaries for macOS and Linux.
