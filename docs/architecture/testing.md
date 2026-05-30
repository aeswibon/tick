# Testing guide

How to verify changes without relying on a live Jira site for every run.

## Quick commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check    # install: cargo install cargo-deny
cargo bench         # optional performance baselines
```

CI runs the same checks on every pull request (see `.github/workflows/ci.yml`).

## Unit tests

Most modules have `#[cfg(test)]` blocks beside the code:

| Area | Location | Examples |
|------|----------|----------|
| Issue keys / URLs | `src/issue_key.rs` | browse URL, mixed paste |
| JQL / views | `src/view_mode.rs` | Closed search, escaping |
| Config | `src/config.rs` | TOML parse, validation |
| Columns | `src/columns.rs` | column ids, custom fields |
| Input routing | `src/input/key_tests.rs` | transition picker, mentions |
| API parsing | `src/api/types.rs`, `markdown.rs`, `adf.rs` | ADF text, tickets |
| Retry policy | `src/api/retry.rs` | 429 backoff, status sets |
| App logic | `src/app.rs` | filter, sort, selection |

Run a subset:

```bash
cargo test issue_key::
cargo test api::retry::
```

## Wiremock integration tests

HTTP tests spin up a local **`wiremock::MockServer`** and point `JiraClient` at its URI. No network or credentials required.

Examples:

- `src/api/mod.rs` — `fetch_all`, transitions, field updates
- `src/api/agile.rs` — boards and sprints
- `src/api/retry.rs` — 429 / 5xx retry behavior against mock responses

When adding an endpoint:

1. Record the method, path, and JSON body shape from Jira REST docs.
2. Mock success and at least one error status (4xx/5xx).
3. Assert parsed `Ticket` fields or error messages, not full HTTP dumps.

## Property-based tests (`proptest`)

Used for parsers where many random inputs should never panic and should satisfy invariants:

- `src/issue_key.rs` — normalized keys round-trip
- `src/view_mode.rs` — `escape_jql_text` never leaves unescaped quotes

Run proptest cases:

```bash
cargo test proptest
```

## UI snapshot tests (`insta`)

Ratatui screens are snapshotted as text via `TestBackend` (see `src/ui/snapshots.rs`):

- Help overlay
- Table header row (column labels)

Update snapshots after intentional UI copy changes:

```bash
INSTA_UPDATE=1 cargo test ui::snapshots
```

Review the diff in `src/ui/snapshots/*.snap` before committing.

## Testing without Jira

| Goal | Approach |
|------|----------|
| API client / parsing | wiremock tests (preferred) |
| Config / JQL / keys | unit + proptest |
| TUI layout copy | insta snapshots |
| End-to-end against Cloud | optional: Atlassian free site + `tick --init` |

For manual QA, use a **developer Cloud site** (free tier) and a dedicated API token with minimal project access. Do not commit tokens or `config.toml` with secrets.

## Planned improvements

- More wiremock coverage for timeout and 403 paths
- Snapshot coverage for transition picker and detail pane sections
- Optional `cargo-fuzz` on JSON response parsing (future work)
