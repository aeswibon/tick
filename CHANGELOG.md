# Changelog

All notable changes to this project are documented in this file.

## [0.7.0] - 2026-05-29

### Added

- Create new issues from the TUI (`n`) — site/project/type pickers, summary, description, required custom fields
- Duplicate selected issue (`C`) with maximal field copy; optional **Cloners** issue link after create
- Per-site `create_project`, `create_issue_type`, `create_clone_link`, `clone_link_type`; global `[create].clone_summary_prefix`

## [0.6.9] - 2026-05-29

### Added

- Status transitions prompt for **required workflow fields** before POST (resolution picker, text for others)
- Typed transition fields: user search, boolean Yes/No, date/datetime, number
- On validation failure, parse Jira `errors` and re-prompt for missing fields

### Changed

- Assignable users cached per issue; filter locally in footer (no per-keystroke API)
- Load more users: **⌘R** (macOS) or **Ctrl+R**; plain `r`/`R` type into the filter
- Refresh **merges** users into the cache (deduped by account id, up to 500 per issue)

### Fixed

- Required-field dialog now appears: re-fetch transition with `transitionId`, load resolutions from `/resolution`, infer Done/Close needs resolution, parse `errorMessages`, show modal for text fields
- Fewer transition API calls: cache resolutions/priorities per site, parallel catalog preload, skip redundant `transitionId` fetch
- User picker: `j`/`k`/arrows navigate the list instead of appending to footer search text
- Keyboard enhancement flags enabled so Command (⌘) modifiers work in supported terminals

## [0.6.8] - 2026-05-29

### Added

- Status picker (`t` / `T`) shows workflow transitions as **action → target status**
- Jira error parsing for failed transitions (`errorMessages`, field `errors`)

### Changed

- Transitions API uses `get_workflow_transitions` with target status from `to.name`
- Clear errors when no ticket, unknown site, empty transitions, or workflow validation fails

## [0.6.7] - 2026-05-29

### Added

- ADF round-trip: ` ```adf-json` fences preserve exotic Jira blocks through edit/save
- Markdown import: strike, rules, blockquotes, code fences, ordered lists, task lists, h4–h6
- Description edit (`D`) restores existing `@` mentions (account IDs) for save

### Changed

- ADF export: task lists, decision lists, status nodes, unknown blocks → fenced JSON

## [0.6.6] - 2026-05-29

### Added

- [docs/FEATURES.md](docs/FEATURES.md) — comprehensive feature reference
- Wiremock tests for assign, unassign, transitions, worklog, issue existence, `/myself`
- Multi-site `O` lookup shows progress (`Checking site (2/3)…`) in header/footer

### Changed

- Header shows **offline** when all sites failed but cached tickets remain
- Multi-site issue probe runs in config order (enables per-site progress)

## [0.6.5] - 2026-05-29

### Added

- `cargo-deny` in CI (license and advisory checks)

### Changed

- Multi-site `O` open: probe each Jira instance via API, first match in config order

## [0.6.4] - 2026-05-29

### Added

- `O` opens a ticket in the browser from the clipboard or by pasting an issue key / Jira browse URL; with multiple sites, probes each Jira instance and opens the first match

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
