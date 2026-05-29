# tick roadmap

## Shipped (v0.6)

| Version | Highlights |
|---------|------------|
| **0.6.0** | HTTP retry/backoff, refresh UX, markdown descriptions + `@` mentions |
| **0.6.1** | ADF → markdown on edit, tables/media in detail pane, ratatui 0.30 |
| **0.6.2** | `tick auth status` for API token + OAuth |
| **0.6.3** | Sort ascending ↑ / descending ↓ (`S`) |
| **0.6.4** | `O` — open issue from clipboard/key; multi-site API probe |
| **0.6.5** | `cargo-deny` in CI; clippy fix for async ticket resolve |
| **0.6.6** | [FEATURES.md](docs/FEATURES.md); offline header; open-ticket progress; API wiremock tests |
| **0.6.7** | ADF round-trip (`adf-json` fences, lists, strike); mention restore on `D` |
| **0.6.8** | Status via workflow transitions; Jira validation errors; `T` alias |
| **0.6.9** | Required transition fields (typed inputs); assignable-user cache; ⌘R/Ctrl+R load more |
| **0.7.0** | Create (`n`) and duplicate (`C`) issues; required create fields; clone link |

## Next — product

| Item | Notes |
|------|------|
| Richer transition fields | Multi-select, components, rich text (web UI today) |
| User picker `j`/`k` vs typing | Names like “Jack” when list is open |
| Jira Server / Data Center | Out of scope unless demand |

## Next — quality

| Item | Notes |
|------|------|
| Input/key handler tests | Expand coverage as new pickers/modals are added |

## Done — infrastructure

| Item | Status |
|------|--------|
| `cargo-deny` in CI | Done (v0.6.5) |
| Assign / transition wiremock tests | Done (v0.6.6) |
| Release checklist: `cargo fmt` before tag | Documented |

---

*See [CHANGELOG.md](CHANGELOG.md) for version history.*
