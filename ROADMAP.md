# tick roadmap

## Shipped (recent)

| Version | Highlights |
|---------|------------|
| **0.7.2** | Watch/unwatch; due date; CLI `tick template export` |
| **0.7.1** | Closed tab + JQL search; template export (`X`); tab reorder; feature docs |
| **0.7.0** | Create (`n`), duplicate (`C`), config templates (`N`) |
| **0.6.9** | Required transition fields; assignable-user cache; ⌘R/Ctrl+R |
| **0.6.8** | Workflow transitions (`t`/`T`) |
| **0.6.6** | [FEATURES.md](docs/FEATURES.md); offline header; wiremock tests |

Full history: [CHANGELOG.md](CHANGELOG.md).

## Priority (product)

Order for upcoming work (user direction, 2026-05):

1. **A — Triage depth** — watch/unwatch, due date, issue links, subtasks, fewer browser detours on transitions  
2. **B — Create/templates** — CLI `tick template export`, template CRUD, saved JQL views  
3. **C — Platform** — `input.rs` split, config reload, rate-limit UX, ROADMAP/CI hygiene  

## Next — product

| Item | Priority | Notes |
|------|----------|-------|
| Watch / unwatch | A | Shipped 0.7.2 |
| Edit due date | A | Shipped 0.7.2 |
| Issue links (view + add) | A | Shipped 0.8.0 |
| Subtasks in detail | A | Shipped 0.8.0 |
| Richer transition/create fields | A | Multi-select components/versions (0.8.0) |
| `tick template export` CLI | B | Shipped 0.7.2 |
| Template edit/delete in TUI | B | |
| Saved JQL / extra views | B | Config-driven tabs |
| Closed tab: persist query, local filter | B | |
| Custom field columns | A/B | `columns` + field ids |

## Next — quality

| Item | Notes |
|------|-------|
| Input/key tests | Expand for Closed search, template export |
| `cargo fmt` in release checklist | CI failed v0.7.1 on fmt — fixed post-tag |

## Out of scope (unless demand)

- Jira Server / Data Center  
- Web UI  

---

*Feature guides: [docs/features/](docs/features/README.md)*
