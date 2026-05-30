# tick roadmap

## Shipped (recent)

| Version | Highlights |
|---------|------------|
| **0.12.0** | Bulk table marks/transition/assign; template description in manager |
| **0.11.2** | Expanded unit/integration tests; roadmap sync |
| **0.11.1** | Split `src/input/` module (no behavior change) |
| **0.11.0** | Config reload (`R`); 429 footer UX |
| **0.10.0** | Saved JQL views; template manager; Closed persist; custom columns |
| **0.9.1** | Remove link; site `link_types`; create subtask |
| **0.9.0** | Links tab navigation; relations fetch only on Links tab |

Full history: [CHANGELOG.md](CHANGELOG.md).

Design notes (local): `docs/specs/` is gitignored; keep copies there for planning. Tracked user docs: [docs/features/](docs/features/README.md).

## Priority stack (2026-05) — complete

| Track | Status |
|-------|--------|
| **A — Triage polish** | Shipped (0.7.2–0.9.1) |
| **B — Templates & views** | Shipped (0.10.0) |
| **C — Platform & quality** | Shipped (0.11.0–0.11.2) |

## C — platform checklist

| Item | Status |
|------|--------|
| Config reload (`R`) | 0.11.0 ✅ |
| Rate-limit UX (429) | 0.11.0 ✅ |
| Split `input.rs` → `src/input/` | 0.11.1 ✅ |
| Test expansion (fetch, retry, keys, views, users) | 0.11.2 ✅ |
| CONTRIBUTING release checklist | 0.11.0 ✅ |
| Consolidate spec paths (gitignore vs tracked) | Deferred — local `docs/specs/` only |

## What's next (v0.12.1+)

| Direction | Examples |
|-----------|----------|
| **Workflow / site tuning** | Zeta `link_types`, custom views, templates, columns from real keys |
| **Triage power** | Bulk labels/watch; editable custom fields; template labels |
| **Quality** | Links-tab wiremock flow; template-export key tests; OAuth polish |
| **Performance** | Large `max_results`; fewer redundant refreshes after writes |

## Out of scope (unless demand)

- Jira Server / Data Center
- Web UI
- Full issue-link type discovery API (use `[[sites]].link_types` config)

---

*Feature guides: [docs/features/README.md](docs/features/README.md)*
