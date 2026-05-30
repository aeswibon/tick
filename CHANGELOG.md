# Changelog

All notable changes to this project are documented in this file.

## [0.24.0] - 2026-05-30

### Added

- **`[[hooks.on_config_reload]]`** — After `R` reloads config, runs shell commands with `TICK_CONFIG_PATH`, `TICK_JSON_PATH` (`tick --check` findings), `TICK_CHECK_ERRORS`, `TICK_CHECK_WARNS`
- **`[[hooks.on_mark]]`** — When **Space** adds a bulk mark (not unmark or Shift+Space mark-all); env `TICK_KEY`, `TICK_SITE`, `TICK_JSON_PATH` (single issue)

### Documentation

- [automation.md](docs/features/automation.md#config-reload-hooks), [CONFIGURATION.md](docs/CONFIGURATION.md)
- Examples: `examples/automation/on-config-reload.sh`, `on-mark.sh`

## [0.23.0] - 2026-05-30

### Added

- **Plugin `run_transition`** — `tick.list_transitions(key)` and `tick.run_transition(key, transition_id)` for plugins with `run_transition = true` (simple transitions only; refreshes view on success)
- Example: `examples/plugins/list-transitions/` (**Ctrl+Shift+T**)

### Documentation

- [plugins.md](docs/features/plugins.md#run_transition-capability)
- **Automation cookbook (D)** — [automation.md](docs/features/automation.md): layer guide (CLI vs hooks vs plugins), jq recipes, cron/CI; expanded [examples/automation/](examples/automation/)

## [0.22.0] - 2026-05-30

### Added

- **Plugin `on_key`** — Register chords in `tick.plugin.toml`; Lua `on_key(chord)` returns `handled` / `passthrough` with `tick.view` and `tick.tickets` context
- Example: `examples/plugins/count-visible/` (**Ctrl+Shift+C** row count in footer)

### Documentation

- [plugins.md](docs/features/plugins.md)

## [0.21.0] - 2026-05-30

### Added

- **Lua plugins (C.1)** — `~/.config/tick/plugins/<name>/` with `tick.plugin.toml` and `filter_tickets()` in `main.lua`; runs after each fetch and on cached view load
- Example: `examples/plugins/hide-epics/`
- `tick --doctor` lists plugin directory and load status

### Documentation

- [plugins.md](docs/features/plugins.md), [plugin-rfc.md](docs/architecture/plugin-rfc.md)

## [0.20.0] - 2026-05-30

### Fixed

- **JQL pagination** — `max_results` is enforced as a total per-site cap; search pages use up to 100 ids per request and bulk-fetch chunks of 100. Footer shows progress during multi-page fetches.

### Documentation

- [platform.md](docs/features/platform.md#jql-pagination-max_results)
- [plugin-rfc.md](docs/architecture/plugin-rfc.md) — draft RFC for track C (no runtime yet)

## [0.19.0] - 2026-05-30

### Changed

- **Lazy detail load** — View refreshes no longer fetch description/comments for every issue; they load when you open the detail pane (or change selection with detail open). `tick issue show` still returns full JSON.

### Documentation

- [platform.md](docs/features/platform.md#lazy-detail-load)

## [0.18.0] - 2026-05-30

### Added

- **Create description preview** — `Ctrl+P` while editing the create/duplicate wizard description shows a live markdown preview (ADF rendering)

### Documentation

- [create-duplicate-templates.md](docs/features/create-duplicate-templates.md)

## [0.17.0] - 2026-05-30

### Added

- **Editable custom fields** — `[[detail.editable_fields]]` for text, select, and user fields; edit from detail with **`F`**
- Values for configured fields are fetched on refresh even when omitted from `columns`

### Documentation

- [docs/features/custom-fields.md](docs/features/custom-fields.md), [CONFIGURATION.md](docs/CONFIGURATION.md#editable-custom-fields)

## [0.16.0] - 2026-05-30

### Added

- **Bulk-complete hooks** — `[[hooks.on_bulk_complete]]` after TUI bulk actions and `tick bulk`; env `TICK_BULK_LABEL`, `TICK_JSON_PATH`, `TICK_OK_COUNT`, `TICK_FAIL_COUNT`
- **Automation examples** — `examples/automation/` sample scripts for hooks

### Documentation

- [CONFIGURATION.md](docs/CONFIGURATION.md#bulk-complete-hooks), [automation.md](docs/features/automation.md#bulk-complete-hooks)

## [0.15.1] - 2026-05-30

### Changed

- **CI** — `cargo fmt` applied (no user-facing code changes)

## [0.15.0] - 2026-05-30

### Added

- **Refresh hooks** — `[[hooks.on_refresh]]` runs a shell command after successful active-view refresh; env `TICK_VIEW`, `TICK_JSON_PATH`, `TICK_ISSUE_COUNT`

### Documentation

- [CONFIGURATION.md](docs/CONFIGURATION.md#refresh-hooks), [automation.md](docs/features/automation.md#refresh-hooks)

## [0.14.0] - 2026-05-30

### Added

- **Quick search** — `Ctrl+g` searches key/summary/labels across all cached view tickets; `Enter` jumps to issue
- **CLI search** — `tick search --jql '...' [--site NAME]` (JSON issues + warnings)
- **CLI bulk** — `tick bulk transition|assign|labels` with `--keys` and `--site`; JSON result; exit 1 on partial failure

### Documentation

- [docs/features/quick-search.md](docs/features/quick-search.md), expanded [automation.md](docs/features/automation.md)

## [0.13.0] - 2026-05-30

### Added

- **Bulk labels** — with table marks, `L` sets comma-separated labels on all marked issues (same-site, max 50)
- **Headless CLI** — `tick issue show` (JSON), `tick issue transition --to <name>`
- **Config check** — `tick --check` for offline structural validation (sites, views, templates, columns)

### Changed

- **Bulk internals** — shared batch runner and transition-by-name module for TUI and CLI

### Documentation

- [docs/features/automation.md](docs/features/automation.md), bulk labels in [bulk-actions.md](docs/features/bulk-actions.md)

## [0.12.2] - 2026-05-30

### Added

- **Template labels** — `Shift+E` → `l` edits comma-separated default labels (saved to `labels = [...]` in template TOML)

### Changed

- **Single-issue writes** — assign, watch/unwatch, transition, comment, links, subtasks, and create refresh the **active view** only (not all background tabs)

## [0.12.1] - 2026-05-30

### Added

- **Contributor docs** — `docs/architecture/` (module map, testing guide), expanded [CONTRIBUTING.md](CONTRIBUTING.md)
- **Dependabot** — weekly Cargo and GitHub Actions updates (`.github/dependabot.yml`)
- **Release checklist** — maintainer issue template (`.github/ISSUE_TEMPLATE/release_checklist.yml`)
- **Tests** — wiremock retry paths (429/403/503), proptest (issue keys, JQL), insta UI snapshots
- **Benchmarks** — `cargo bench` pipeline (JQL, filter/sort, theme, issue keys)
- **Bulk watch** — `W` / `Shift+W` on marked table rows (same-site)

### Changed

- **Bulk writes** — transition and assign refresh the **active view** only (not all background tabs)

## [0.12.0] - 2026-05-30

### Added

- **Bulk table selection** — `Space` toggle mark, `Shift+Space` mark filtered rows (max 50), `Esc` clear; `✓` in Key column
- **Bulk transition** — with marks, `t` applies chosen transition by name to each issue (same site); per-issue failures reported in footer
- **Bulk assign** — with marks, `a` assigns current user to all marked issues (same site)
- **Template description** — `Shift+E` → `b` edits template `description` (markdown footer, empty allowed)

### Documentation

- [docs/features/bulk-actions.md](docs/features/bulk-actions.md), KEYBINDINGS, FEATURES, help overlay

## [0.11.2] - 2026-05-30

### Added

- **Tests** — Multi-site `fetch_all` partial failure (wiremock); assignable-user filter/merge; retry `Retry-After` parsing; fetch status formatting; issue-key parsing; view/Closed JQL; input transition/mention routing; issue-relations cache helpers

## [0.11.1] - 2026-05-30

### Changed

- **Internal** — Split `src/input.rs` into `src/input/` (`mod`, `mentions`, `transitions`, `normal`, `detail_actions`, `key_tests`); no user-facing behavior change.

## [0.11.0] - 2026-05-30

### Added

- **Config reload** — `R` reloads `config.toml` from disk (sites, views, templates, columns, theme, auth) without restarting tick
- **Rate-limit UX** — footer shows backoff countdown when Jira returns HTTP 429 (after automatic retries)

### Documentation

- [KEYBINDINGS.md](docs/KEYBINDINGS.md), [CONFIGURATION.md](docs/CONFIGURATION.md) updated for v0.11

## [0.10.0] - 2026-05-30

### Added

- **Saved JQL views** — `[[views.custom]]` with name, jql, optional `site` and tab key `7`–`9`; `v` / `Shift+V` cycle custom views
- **Template manager** — `Shift+E` to list, edit summary/project/type, or delete templates; rewrites `config.toml` or `create.templates_file`
- **Closed tab persist** — last search query and ever-assigned toggle saved to `~/.config/tick/cache/closed_prefs.json`
- **Closed local filter** — `f` filters fetched Closed results without a new JQL call
- **Custom field columns** — `columns` may include `customfield_*` ids (read-only in table; included in bulk fetch and local filter)

### Documentation

- New guide: [docs/features/saved-views-templates-columns.md](docs/features/saved-views-templates-columns.md)
- Updated [KEYBINDINGS.md](docs/KEYBINDINGS.md), [CONFIGURATION.md](docs/CONFIGURATION.md), [FEATURES.md](docs/FEATURES.md)

## [0.9.1] - 2026-05-30

### Added

- **Remove issue link** — `Shift+I` on Links tab (link rows only)
- **Per-site link types** — `link_types` on `[[sites]]` for Jira name overrides
- **Create subtask** — `Shift+N` on Links tab (summary → POST with parent)

## [0.9.0] - 2026-05-30

### Added

- **Links tab navigation** — `j`/`k` select link or subtask row; `Enter` jump (select in table or open in browser); `o` open in browser
- Relations fetch only when **Links** tab is active (not on every table row change)

## [0.8.0] - 2026-05-30

### Added

- **Issue links** — Links detail tab lists related issues; **`I`** adds Relates / Blocks / Epic links
- **Subtasks** — Shown on Links tab (lazy-loaded with issue links)
- **Multi-select fields** — Components and fix versions on transitions and create (checklist modal)

## [0.7.2] - 2026-05-30

### Added

- **Watch / unwatch** — `W` and `Shift+W` from table or detail (Jira watchers API)
- **Edit due date** — `d` in detail pane (`YYYY-MM-DD`, empty clears)
- **CLI** — `tick template export SITE KEY... [-o file] [--append]` for bootstrap templates

## [0.7.1] - 2026-05-30

### Added

- **Closed** tab (`6`) — JQL search in done issues (`/` + `Enter`); `h` toggles assignee-when-done vs **ever assigned** (`assignee was currentUser()`)
- **Export template** (`X`) — interactive field picker, save to `[[create.templates]]` or `create.templates_file`
- `create.templates_file` — merge external template TOML at config load
- **Documentation** — [docs/features/](docs/features/README.md) per-feature guides with examples; expanded [KEYBINDINGS.md](docs/KEYBINDINGS.md)

### Changed

- Tab order: **Assigned · Mentions · Watched · Updated · Sprint · Closed** (keys `1`–`6`)
- [docs/FEATURES.md](docs/FEATURES.md) slimmed to overview; depth moved to feature guides
- Footer success messages use `action_notice` (not red error styling)

## [0.7.0] - 2026-05-29

### Added

- Create new issues from the TUI (`n`) — site/project/type pickers, summary, description, required custom fields
- Duplicate selected issue (`C`) with maximal field copy; optional **Cloners** issue link after create
- Per-site `create_project`, `create_issue_type`, `create_clone_link`, `clone_link_type`; global `[create].clone_summary_prefix`
- Issue **templates** (`N`) via `[[create.templates]]` — pre-filled create with minimal summary/description edits

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
