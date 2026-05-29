# tick roadmap

Open-source Jira TUI (`aeswibon/tick`). Priorities are ordered for impact vs effort. Adjust as users report needs.

## v0.1.x — Stabilize (current)

**Goal:** Trustworthy installs and daily use for a single developer or small team.

| Item | Why |
|------|-----|
| Green release pipeline | Tagged releases ship macOS (Intel + ARM) + Linux binaries, checksums, `tick.rb` |
| Homebrew tap | Update `Formula/tick.rb` SHAs after each release; document `brew tap aeswibon/tick` |
| README & config docs | Accurate keybindings, credentials note, multi-site setup |
| Verified signed commits | GPG-signed history, no bot co-authors on release commits |

## v0.2 — Quality & portability

**Goal:** Fewer surprises; easier contributions.

| Item | Why |
|------|-----|
| CI: `fmt` + `clippy` on PRs | Catch style and lint before merge (optional gate on snapshot CI) |
| Windows release binary | `x86_64-pc-windows-msvc` in release matrix |
| Integration tests (mock HTTP) | Stable tests for JQL → bulk fetch → ticket mapping without live Jira |
| Config: custom JQL per view | Power users want Assigned/Updated/Mentions/Watched queries in TOML |
| Error overlay | Scrollable site-error list when multiple sites fail (not one truncated footer line) |

## v0.3 — Power-user workflow

**Goal:** Compete with “live in Jira” for common tasks.

| Item | Why |
|------|-----|
| Assign / unassign | Quick ownership changes from detail pane |
| Edit summary / priority | Light field updates without opening browser |
| Sprint / epic filters | Optional columns and filters for `parent`, `sprint` |
| Keyboard-driven transition picker | Vim-style j/k in overlay (today: 1–9 only) |
| Notification on refresh | Optional desktop notify when background fetch finds new tickets |

## v0.4 — Scale & polish

**Goal:** Larger backlogs and nicer UX.

| Item | Why |
|------|-----|
| Virtualized table | Smooth scrolling with 500+ tickets |
| OAuth / API token file | Support `~/.config/tick/token` + env override (`TICK_TOKEN`) |
| Offline mode | Read-only from cache when API unreachable; stale indicator in header |
| Theme gallery in repo | `themes/` examples + screenshot in README |
| Optional `homebrew-tap` repo | Auto-bump formula SHA via release workflow |

## Community & governance

| Item | When |
|------|------|
| Issue templates + labels | v0.2 |
| “Good first issue” triage | After first external contributors |
| SECURITY.md | Before wide adoption |
| Changelog (`CHANGELOG.md`) | Start at v0.2.0 |

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
