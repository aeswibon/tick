# Changelog

All notable changes to this project are documented in this file.

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
