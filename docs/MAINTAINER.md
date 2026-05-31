# Maintainer guide (OSS)

Community roadmap: [OSS_ROADMAP.md](OSS_ROADMAP.md). Product releases: [ROADMAP.md](../ROADMAP.md) (local, not in git).

## One-time repo setup

| Task | Where | Notes |
|------|--------|------|
| **GitHub Discussions** | Repo → Settings → General → Features | Enable Discussions; categories: General, Ideas, Q&A |
| **Default labels** | Issues → Labels | Create if missing (see below) |
| **Branch protection** | Settings → Branches | Require CI on `master` |
| **Homebrew tap token** | Repo secrets → `HOMEBREW_TAP_TOKEN` | See [CONTRIBUTING.md § Releases](../CONTRIBUTING.md#releases) |

### Recommended labels

| Label | Color (suggestion) | Use |
|-------|-------------------|-----|
| `good first issue` | `#7057ff` | Small, documented tasks |
| `help wanted` | `#008672` | Any contributor welcome |
| `bug` | `#d73a4a` | Defect |
| `enhancement` | `#a2eeef` | Feature |
| `documentation` | `#0075ca` | Docs-only |
| `question` | `#d876e3` | Needs clarification |
| `idea` | `#fef2c0` | From Discussions |

PR area labels (`area: api`, `area: ui`, …) are applied automatically by [.github/workflows/labeler.yml](../.github/workflows/labeler.yml).

## Good-first-issue pipeline

1. Pick a scoped task (one module, one doc page, one wiremock test).
2. File with [**Good first issue** template](../.github/ISSUE_TEMPLATE/good_first_issue.yml) or add labels to an existing issue.
3. Include: summary, files, `cargo test` / manual steps.
4. Mention in release notes when a first-time contributor lands a PR (optional).

### Starter backlog ideas

| Task | Files | Test |
|------|-------|------|
| Snapshot: transition picker header | `src/ui/transitions.rs`, `src/ui/snapshots.rs` | `INSTA_UPDATE=1 cargo test ui::snapshots` |
| Wiremock: Jira 403 on `search_jql` | `src/api/mod.rs` | `cargo test` |
| Document a keybinding in USER_GUIDE | `docs/USER_GUIDE.md` | — |
| Example plugin: highlight blockers | `examples/plugins/` | `tick --doctor` |

## Automation (in repo)

| Workflow | Purpose |
|----------|---------|
| [ci.yml](../.github/workflows/ci.yml) | fmt, clippy, test, deny, release build |
| [labeler.yml](../.github/workflows/labeler.yml) | PR path labels |
| [stale.yml](../.github/workflows/stale.yml) | Stale issues/PRs (60d / 14d close) |
| [release.yml](../.github/workflows/release.yml) | Tag → binaries + Homebrew |

## Cutting a release

Use [.github/ISSUE_TEMPLATE/release_checklist.yml](../.github/ISSUE_TEMPLATE/release_checklist.yml) and [CONTRIBUTING.md § Releases](../CONTRIBUTING.md#releases).

## Security

[SECURITY.md](../SECURITY.md) — private reports only; bump supported version table when shipping minors.

## Updating the OSS roadmap

After shipping infra or tests, update checkboxes in [OSS_ROADMAP.md](OSS_ROADMAP.md) and the “Last updated” date.
