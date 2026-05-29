# tick roadmap

## Shipped (v0.6)

| Version | Highlights |
|---------|------------|
| **0.6.0** | HTTP retry/backoff, refresh UX, markdown descriptions + `@` mentions |
| **0.6.1** | ADF → markdown on edit, tables/media in detail pane, ratatui 0.30 |
| **0.6.2** | `tick auth status` for API token + OAuth |
| **0.6.3** | Sort ascending ↑ / descending ↓ (`S`) |
| **0.6.4** | `O` — open issue from clipboard/key; multi-site API probe |
| **0.6.5** | `cargo-deny` in CI; parallel multi-site probe |

## Next — infrastructure

| Item | Status |
|------|--------|
| `cargo-deny` in CI | Done (v0.6.5) |
| Input / assign / transition wiremock tests | Planned |
| Release checklist: `cargo fmt` before tag | Documented |

## Next — product

| Item | Notes |
|------|-------|
| Open-ticket UX | Probe progress in footer (`Checking site 2/3…`) |
| Offline hint | Clearer header when all sites fail but cache is shown |
| ADF round-trip polish | Exotic blocks on description edit |
| Jira Server / Data Center | Out of scope unless demand |

---

*See [CHANGELOG.md](CHANGELOG.md) for version history.*
