# Changelog

All notable changes to this project are documented in this file.

## [0.2.0] - 2026-05-29

### Added

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

## [0.1.0] - 2026-05-29

Initial public release: multi-site Jira Cloud TUI, views, detail pane, transitions, comments, worklogs, themes, Homebrew formula, and release binaries for macOS and Linux.
