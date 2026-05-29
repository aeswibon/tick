# tick roadmap

Open-source Jira TUI (`aeswibon/tick`). Priorities are ordered for impact vs effort. Adjust as users report needs.

## v0.1.x — Stabilize ✅

**Goal:** Trustworthy installs and daily use for a single developer or small team.

| Item | Why |
|------|-----|
| Green release pipeline | Tagged releases ship macOS (Intel + ARM) + Linux binaries, checksums, `tick.rb` |
| Homebrew tap | Update `Formula/tick.rb` SHAs after each release; document `brew tap aeswibon/tick` |
| README & config docs | Accurate keybindings, credentials note, multi-site setup |
| Verified signed commits | GPG-signed history, no bot co-authors on release commits |

## v0.2 — Quality & portability ✅ (shipped)

**Goal:** Fewer surprises; easier contributions.

| Item | Status |
|------|--------|
| CI: `fmt` + `clippy` on PRs | Done |
| Windows release binary | Done |
| Integration tests (mock HTTP) | Done |
| Config: custom JQL per view | Done |
| Error overlay (`!`) | Done |
| Token file / `TICK_TOKEN` | Done (early) |
| Stale/cached header indicator | Done (early) |

## v0.3 — Power-user workflow (in progress)

**Goal:** Compete with “live in Jira” for common tasks.

| Item | Status |
|------|--------|
| Assign / unassign (`a` / `u`) | Done |
| Parent column / filter | Done |
| Keyboard-driven transition picker | Done |
| Edit summary / priority | Planned |
| Sprint filters | Planned |
| Notification on refresh | Planned |

## v0.4 — Scale & polish

**Goal:** Larger backlogs and nicer UX.

| Item | Why |
|------|-----|
| Virtualized table | Smooth scrolling with 500+ tickets |
| OAuth | Planned (token file + env done in v0.2) |
| Offline mode | Partial — disk cache + `[cached]` header |
| Theme gallery in repo | `themes/` examples + screenshot in README |
| Auto-bump `homebrew-tick` on release | Push formula SHA from release workflow (tap repo exists; bump is manual today) |

## Community & governance

| Item | When |
|------|------|
| Issue templates + labels | v0.2 |
| “Good first issue” triage | After first external contributors |
| SECURITY.md | Done |
| Changelog (`CHANGELOG.md`) | Done |

## Out of scope (for now)

- Jira Server / Data Center (Cloud REST v3 only)
- Plugin marketplace distribution
- Web UI or mobile client
- Real-time WebSocket sync (polling + cache is enough)

## How to suggest work

Open a [GitHub issue](https://github.com/aeswibon/tick/issues) with:

1. **Problem** — what workflow is painful today  
2. **Proposal** — behavior you want  
3. **Alternatives** — browser, other TUIs, scripts you tried  

---

*Last updated: 2026-05-29*
