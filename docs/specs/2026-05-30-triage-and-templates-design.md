# tick — Triage depth & template tooling (v0.7.2+)

**Date:** 2026-05-30  
**Status:** Draft — pending user review  
**Priority:** A (triage) > B (templates) > C (platform)  
**Baseline:** v0.7.1

---

## 1. Goal

Reduce browser detours for daily Jira work (priority **A**), with a small **B** slice for template/bootstrap tooling. Defer large **C** refactors unless they unblock A/B.

---

## 2. Scope — v0.7.2 (first slice)

### 2.1 Watch / unwatch (A)

| Item | Detail |
|------|--------|
| API | `POST /rest/api/3/issue/{key}/watchers` and `DELETE .../watchers` (current user) |
| Keys | **`W`** watch, **`Shift+W`** unwatch (table + detail); **`w`** stays worklog |
| UX | Footer notice; refresh active view after success |
| Docs | [features/editing-fields.md](../features/editing-fields.md) or watchers section |

### 2.2 Edit due date (A)

| Item | Detail |
|------|--------|
| Key | **`d`** (detail open) — due date prompt `YYYY-MM-DD`, empty clears |
| API | `PUT` issue fields `duedate` |
| Validation | Parse date; show Jira field errors in footer |

### 2.3 CLI template export (B)

| Item | Detail |
|------|--------|
| Command | `tick template export SITE KEY [KEY...] [-o path] [--history]` |
| Impl | Reuse `template_export::export_issues_to_toml` + `JiraClient::from_config` |
| Output | Append to `create.templates_file` or stdout |

**Out of scope for v0.7.2:** issue links UI, subtasks panel, multi-select transition fields (v0.8.0).

---

## 3. Scope — v0.8.0 (second slice)

- Issue links: list in detail + add link (Relates / Blocks / Epic)
- Subtasks: fetch children via JQL `parent = KEY` or subtasks API
- Transition/create: multi-select components (checklist pattern like template export)

---

## 4. Platform (C) — as needed

- Split `input.rs` when adding `W` / `d` keys
- CONTRIBUTING: run `cargo fmt --all` before tag
- No `app.rs` rewrite unless blocked

---

## 5. Success criteria

- [ ] `W` watch/unwatch works on Cloud from table and detail
- [ ] `d` sets/clears due date with clear errors
- [ ] `tick template export` produces valid `[[create.templates]]` TOML
- [ ] CI green; docs/features updated
- [ ] 130+ tests still pass; wiremock for watch + duedate PUT optional

---

## 6. Open questions

1. **Due date key `d`** — acceptable vs another key? (Recommend `d` detail-only.)
2. **Watch key `W`** — any conflict on non-QWERTY layouts?

---

*Next step after approval: implementation plan for v0.7.2.*
