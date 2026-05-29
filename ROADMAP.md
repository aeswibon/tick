# tick roadmap

Open-source Jira TUI (`aeswibon/tick`). Priorities are ordered for impact vs effort.

## v0.1.x — Stabilize ✅

## v0.2 — Quality & portability ✅

CI, Windows binary, custom JQL, error overlay, token file/env, page_size, lib.rs, cache module, Homebrew tap + auto-bump (`HOMEBREW_TAP_TOKEN`).

## v0.3 — Power-user workflow

**Goal:** Daily Jira from the terminal without opening the browser.

| Item | Status |
|------|--------|
| Assign / unassign (`a` / `u`) | Done |
| Parent column / filter | Done |
| Keyboard-driven transition picker | Done |
| Edit summary / priority (`S` / `P`) | Done (v0.3.0) |
| Labels column + filter | Planned (v0.3.1) |
| Sprint column + view JQL examples | Planned (v0.3.1) |
| Desktop notification on refresh | Planned (v0.3.2) |

## v0.4 — Scale & polish

| Item | Status |
|------|--------|
| Virtualized table | Planned |
| Stronger offline UX | Partial (cache + keep on fetch fail) |
| Theme gallery in repo | Planned |
| OAuth | Planned (on demand) |

## Community & governance

Issue templates, SECURITY.md, CHANGELOG — done.

## Out of scope (for now)

- Jira Server / Data Center
- Web UI or mobile client
- Real-time WebSocket sync

---

*Last updated: 2026-05-29*
