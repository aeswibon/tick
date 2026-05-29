# tick — Create & duplicate issue design

**Date:** 2026-05-29  
**Target version:** 0.7.0  
**Status:** Draft — awaiting review  
**User choices:** Mixed quick/varied/strict create flows; **duplicate copies all fields tick can represent** (option 3).

---

## 1. Goals

### Primary

1. **Create** new Jira Cloud issues from the TUI without opening the browser.
2. **Duplicate** the selected issue with maximal field copy; user edits mainly **summary** and **description** before submit.
3. Support **quick create** (config defaults), **varied** project/type selection, and **strict** projects (required custom fields) in one flow.

### Success criteria

- `n` starts a blank create (respecting per-site defaults when configured).
- `C` duplicates the selected row; pre-filled fields match the source issue where Jira allows on create.
- After success: table refreshes, new issue is selected and detail pane opens.
- Validation errors from Jira re-prompt for missing fields (same pattern as workflow transitions).
- Wiremock tests cover create POST and at least one validation-error path.

### Non-goals (v0.7.0)

- Jira Server / Data Center.
- Bulk duplicate.
- Sub-task-only wizards without testing real project configs.
- Copying status, comments, worklogs, or attachments.

---

## 2. User-facing behavior

### Keybindings

| Key | When | Action |
|-----|------|--------|
| `n` | Normal mode | New issue wizard |
| `C` | Row selected | Duplicate wizard (maximal copy) |
| `Esc` | During wizard | Cancel; clear create session |

Help (`?`) and [docs/FEATURES.md](../../FEATURES.md) document both keys.

### Wizard stages

Shared **create session** state machine (similar to `TransitionCollect`):

| Step | Blank (`n`) | Duplicate (`C`) |
|------|-------------|-----------------|
| Site | Picker if `sites.len() > 1`; else implicit | Same as source ticket `site` |
| Project | Picker or footer; default from config | Source `project_key` |
| Issue type | Picker from create meta; default from config | Source `issue_type` |
| Summary | Empty or config template | Prefill: `Copy of: {summary}` (editable) |
| Description | Optional; markdown + ADF | Copy source description (markdown round-trip) |
| Field review | Optional confirm screen listing copied/set fields | Same; highlight summary/description focus |
| Extra fields | Priority, labels, assignee, due date, parent — editable | **Pre-filled from source** (maximal) |
| Required custom | From create metadata; transition-field UI | Pre-fill when create meta allows; else prompt |
| Submit | `POST /rest/api/3/issue` | Same |
| Post-create | Refresh; select new key; open detail | Optional **clone link** to source (see §4) |

**Quick path (A):** If `create_project` + `create_issue_type` are set on the active site, `n` skips to **summary** after site resolution. User can press `p` / `t` during the wizard to change project or type.

**Varied path (B):** Project and issue type pickers always available via API (`/project/search`, create metadata).

**Strict path (C):** After summary/description, load required fields for `(project, issueType)` and collect values before POST; on failure parse `errors` and re-prompt.

### Duplicate — maximal copy (option 3)

Copy into the create payload everything tick can map to Jira create fields:

| Source (`Ticket` + issue GET) | Create field | Notes |
|------------------------------|--------------|--------|
| `site` | (routing) | Base URL for API |
| `project_key` | `project` | `{ "key": "…" }` |
| `issue_type` | `issuetype` | `{ "name": "…" }` |
| `summary` | `summary` | Prefixed for edit; user changes before submit |
| `description_adf` / markdown | `description` | Prefer ADF from source |
| `labels` | `labels` | Full list |
| `priority` | `priority` | Resolve name → id via cached `/priority` |
| `assignee` | `assignee` | Resolve display name → `accountId` via assignable search or issue GET |
| `due_date` | `duedate` | ISO date if set |
| `parent_key` | `parent` | `{ "key": "…" }` when set (sub-task / parent) |
| Sprint (custom) | site `sprint_field` | Best-effort: copy if field is on create screen and value is readable |
| Reporter | — | **Not copied** (Jira sets reporter to current user) |
| Status | — | New issue gets project default status |
| Comments / worklogs | — | Not copied |

**Implementation note:** `Ticket` today stores assignee as display name only. On `C`, tick **GET** `/rest/api/3/issue/{key}?fields=assignee,priority,parent,labels,description,issuetype,project,duedate,{sprint_field}` (and other create-eligible custom fields discovered from create meta) to build an accurate payload. List view data seeds the wizard UI immediately; GET completes before POST.

### Clone link (default on duplicate)

After successful create from `C`:

- `POST /rest/api/3/issueLink` with type **Cloners** (or site-configurable type name) from **new → source**, if the link type exists.
- On link failure: show footer warning; **do not** roll back the created issue.
- Config: `create_clone_link = true` (default), `clone_link_type = "Cloners"`.

---

## 3. Configuration

Per-site optional defaults in `config.toml`:

```toml
[[sites]]
name = "my-team"
base_url = "https://my-team.atlassian.net"
create_project = "ENG"          # optional — skip project step for `n`
create_issue_type = "Task"      # optional
create_clone_link = true        # optional, default true
clone_link_type = "Cloners"   # optional
```

Global fallback (optional):

```toml
[create]
clone_summary_prefix = "Copy of: "   # optional
```

`tick --init` template documents these keys.

---

## 4. API & modules

### New / extended API (`src/api/`)

| Function | Endpoint |
|----------|----------|
| `create_issue` | `POST /rest/api/3/issue` → returns `key` |
| `search_projects` | `GET /rest/api/3/project/search` |
| `create_meta` | `GET /rest/api/3/issue/createmeta` (or field-centric metadata if deprecated paths differ) |
| `link_issues` | `POST /rest/api/3/issueLink` |
| `issue_fields_for_clone` | `GET /rest/api/3/issue/{key}?fields=…` |

New types:

- `CreateError { message, field_errors }` — mirror `TransitionError`.
- `CreateCollect` in `app.rs` — project, type, summary, description, extras, pending required fields, `source_key: Option<String>` for duplicate.

Reuse:

- `transition_fields` parsing for required create fields where schemas overlap.
- `markdown::to_adf` for description on create.
- `enrich_transition_fields` pattern for priority/resolution/user resolution.

### UI

- `src/ui/create.rs` — wizard overlay / footer hints per step.
- Extend `draw.rs` footer for `InputMode::Create*` variants (or single `CreateField` mode like transitions).

### Input

- `start_create_blank`, `start_create_duplicate`, `handle_create_key`, `submit_create`.
- Register `n` / `C` in `handle_normal_key`.

---

## 5. Approaches considered

| Approach | Verdict |
|----------|---------|
| Single `n` with “duplicate?” prompt | Rejected — `C` matches `o`/`O` and is faster for your clone-heavy workflow. |
| Clone only summary + description | Rejected — you chose maximal (option 3). |
| Separate REST “clone” API | N/A on Cloud — field copy + optional issue link. |
| Phase 1 without required custom fields | Rejected for your mix of C — ship required-field collection with create in 0.7.0. |

---

## 6. Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Create meta varies by project | Cache per `(site, project)`; clear errors; re-prompt |
| Assignee not assignable on new issue | POST without assignee + footer warning, or picker to fix |
| Parent/epic field confusion | Use Jira `parent` when `parent_key` set; document epic-link custom fields as v0.7.1 if needed |
| Sprint field not on create screen | Skip sprint on create; footer note |
| `createmeta` API changes | Prefer field metadata from edit/create screens; follow Jira Cloud v3 docs |

---

## 7. Testing

- Wiremock: create success returns `PROJ-99`.
- Wiremock: create 400 with `errors` → field re-prompt.
- Unit: `CreateCollect` payload builder from mock `Ticket` + GET JSON.
- Unit: clone field mapping (labels, parent, priority id resolution).
- Input: `transition_user_field_key_action`-style tests for create step routing where pure.

---

## 8. Documentation & release

- [docs/FEATURES.md](../FEATURES.md) — § Create / duplicate.
- [README.md](../../README.md) keybinding table — `n`, `C`.
- [CHANGELOG.md](../../CHANGELOG.md) — v0.7.0 section.
- [ROADMAP.md](../../ROADMAP.md) — mark create/duplicate planned → shipped on release.

---

## 9. Implementation phases (for planning)

1. **API + types** — `create_issue`, project search, create meta, clone GET.
2. **Session state + `n` wizard** — summary → POST → refresh.
3. **`C` duplicate** — maximal copy + GET enrichment.
4. **Required custom fields** — reuse transition field UI.
5. **Clone link + config** — issue link POST.
6. **Docs, tests, help**.

---

## Spec self-review

- [x] No placeholder sections.
- [x] Scope bounded (no Server/DC, no bulk).
- [x] Aligns with user: mix A/B/C + maximal duplicate.
- [x] Assignee/parent/sprint gaps in `Ticket` addressed via GET on duplicate.
- [x] Clone link specified with non-blocking failure.

---

*Review this file; once approved, next step is an implementation plan (`writing-plans`) for v0.7.0.*
