# tick architecture

High-level map for contributors. For user-facing behavior see [feature guides](../features/README.md).

## Runtime overview

tick is a **single-binary async TUI**:

1. **CLI** (`src/lib.rs`, `src/main.rs`) — parse flags, load `config.toml`, auth, optional `doctor` / template export / OAuth subcommands.
2. **Terminal setup** — raw mode, alternate screen, ratatui + crossterm backend.
3. **Main loop** (≈250 ms poll):
   - Apply background fetch results (`App::apply_pending_updates`)
   - **Render** — `ui::draw::render`
   - **Input** — `input::handle_key` (async; may call Jira)
   - Periodic **refresh** (default 3 h) via `refresh_all_notify` + background `JoinSet`

```text
┌─────────────┐     poll KeyEvent      ┌──────────────┐
│  crossterm  │ ────────────────────► │ input/       │
└─────────────┘                       │ handle_key   │
       ▲                              └──────┬───────┘
       │ draw                               │ mutates
┌──────┴──────┐                       ┌─────▼───────┐
│ ui/draw     │ ◄── read state ───────│ App         │
└─────────────┘                       │ tickets RwLock│
                                      │ view caches │
                                      └─────┬───────┘
                                            │ HTTP
                                      ┌─────▼───────┐
                                      │ api/        │
                                      │ JiraClient  │
                                      └─────────────┘
```

## Module map

| Module | Role |
|--------|------|
| `app` | Central state: selection, filters, modals, `InputMode`, view caches, refresh orchestration |
| `input/` | Keyboard routing: `normal`, `transitions`, `mentions`, `detail_actions`, `key_tests` |
| `ui/` | ratatui widgets: table, detail, pickers, help, footer (`draw.rs` orchestrates) |
| `api/` | Jira REST client, types, ADF/markdown, retries (429), wiremock-tested endpoints |
| `config` | `config.toml` / templates / views deserialization and validation |
| `view_mode` | Tab JQL, Closed search JQL builder, tab order |
| `columns` | Table column definitions and custom field ids for bulk fetch |
| `cache` | Disk cache under `~/.config/tick/cache/` |
| `bulk` | Multi-select table actions (mark, bulk transition, bulk assign) |
| `create_flow` | Create / duplicate wizard and required-field modals |
| `template_*` | Export (`X`), manage (`Shift+E`), persist TOML |
| `issue_relations_flow` | Links tab fetch, add/remove link, subtasks |
| `auth` / `oauth` / `auth_status` | API token + OAuth 2.0 |
| `theme` | Built-in and file-based themes |
| `ticket_lock` | `Arc<RwLock<Vec<Ticket>>>` read/write helpers |
| `fetch_status` | Footer errors, site warnings, action notices |
| `platform` | Open browser, desktop notifications |

Entry points:

- **TUI:** `tick::run()` in `lib.rs`
- **Library:** same crate (`tick`); integration tests use `wiremock` against `api`

## State model

- **Tickets** live in `App.tickets` (`RwLock`). Views cache copies in `view_cache` / `custom_view_cache`.
- **Table selection** is an index into **filtered** rows (`filtered_indices` + `FilterCache`), not raw vec index.
- **Modals** use boolean flags (`showing_transitions`, etc.) plus `input_mode` for footer text entry.
- **Write-back** flows call `JiraClient`, then usually `refresh_all()` or targeted cache invalidation.

## Input routing pattern

`input::handle_key` checks overlays in order:

1. Mention picker → add-link → transition field (multi/user/text) → create picker → template export/manage
2. Local filter mode (`/`)
3. Footer `InputMode` (comment, edit fields, create, template edit, …)
4. Site errors overlay → transition/priority/sprint pickers
5. Shift-modified keys (`W`, `I`, `N`, `E`, `v`, bulk `Space`)
6. `normal::handle_normal_key` (navigation, triage keys)

Async handlers await Jira without blocking the executor (tokio multi-thread runtime).

## API layer

- `JiraClient` wraps `reqwest` with `api::retry` for 429/5xx backoff.
- `api::fetch_all` — parallel per-site JQL + `bulkfetch` for fields.
- `api::pagination` — JQL page size (100) and bulk-fetch chunk size; `max_results` is a total cap per site.
- `api::types::Ticket` — denormalized row for the table; ADF kept for description/comments.
- Transition required fields: `transition_fields` + modal reuse from create flow.

See [testing.md](testing.md) for how to test without a live Jira site.

## Key conventions

- **Vim-style** keys; detail vs table contexts documented in [KEYBINDINGS.md](../KEYBINDINGS.md).
- **Errors** surface in footer via `FetchStatus` (not panics in the TUI loop).
- **Config reload** (`R`) re-reads `config.toml` in-process; `r` refreshes Jira data.
- Prefer **small modules** over growing `app.rs`; new flows get a `*_flow.rs` when they need a state machine.

## Related docs

- [plugin-rfc.md](plugin-rfc.md) — draft plugin runtime (track C; not implemented)
- [testing.md](testing.md) — unit, wiremock, insta, proptest, benches
- [CONTRIBUTING.md](../../CONTRIBUTING.md) — first PR and CI checks
- [CHANGELOG.md](../../CHANGELOG.md) — shipped versions
