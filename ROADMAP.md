# tick roadmap

## Shipped (recent)

| Version | Highlights |
|---------|------------|
| **0.11.0** | Config reload (`R`); 429 footer UX |
| **0.10.0** | Saved JQL views; template manager; Closed persist; custom columns |
| **0.9.1** | Remove link; site `link_types`; create subtask |
| **0.9.0** | Links tab navigation; relations fetch only on Links tab |
| **0.8.0** | Issue links tab; add link (`I`); subtasks; multi-select components/versions |
| **0.7.2** | Watch/unwatch; due date; CLI `tick template export` |
| **0.7.1** | Closed tab + JQL search; template export (`X`); tab reorder; feature docs |
| **0.7.0** | Create (`n`), duplicate (`C`), config templates (`N`) |
| **0.6.9** | Required transition fields; assignable-user cache; ⌘R/Ctrl+R |

Full history: [CHANGELOG.md](CHANGELOG.md).

Design spec (approved): [docs/specs/2026-05-30-future-roadmap-design.md](docs/specs/2026-05-30-future-roadmap-design.md)

## Priority (product)

Order for upcoming work (user direction, 2026-05):

1. **A — Triage polish** — links navigation, remove link, site link types *(core triage shipped in 0.7.2–0.8.0)*  
2. **B — Templates & views** — template edit/delete, saved JQL views, Closed persist, custom columns  
3. **C — Platform** — `input.rs` split, config reload, rate-limit UX, tests, docs hygiene  

## Next releases

| Version | Theme | Highlights |
|---------|-------|------------|
| **0.9.0** | A | Shipped |
| **0.9.1** | A | Shipped |
| **0.10.0** | B | Shipped |
| **0.11.0+** | C | Split `input.rs`; config reload; 429 UX; expanded tests; docs cleanup |

## Backlog (by priority)

### A — remaining triage

| Item | Target |
|------|--------|
| Links tab navigation | 0.9.0 ✅ |
| Relations fetch efficiency | 0.9.0 ✅ |
| Remove issue link | 0.9.1 ✅ |
| Configurable link types | 0.9.1 ✅ |
| Create subtask from parent | 0.9.1 ✅ |

### B — templates & views

| Item | Target |
|------|--------|
| Template edit/delete in TUI | 0.10.0 ✅ |
| Saved JQL / extra views | 0.10.0 ✅ |
| Closed tab: persist query, local filter | 0.10.0 ✅ |
| Custom field columns | 0.10.0 ✅ |

### C — platform & quality

| Item | Target |
|------|--------|
| Split `input.rs` | 0.11.0+ |
| Config reload (`R`) | 0.11.0 ✅ |
| Rate-limit UX (429) | 0.11.0 ✅ |
| Split `input.rs` | 0.11.1+ |
| Input/key test expansion | 0.11.0+ |
| CONTRIBUTING release checklist | 0.11.0+ |
| Consolidate duplicate spec paths | 0.11.0+ |

## Out of scope (unless demand)

- Jira Server / Data Center  
- Web UI  

---

*Feature guides: [docs/features/](docs/features/README.md)*
