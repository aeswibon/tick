# Saved views, template manager, Closed persist, custom columns

v0.10.0 adds **config-driven JQL tabs**, **template edit/delete in the TUI**, **Closed tab state on disk**, and **read-only custom field columns**.

## Quick reference

| Feature | Keys / config |
|---------|----------------|
| Custom JQL view | `7`–`9` (or `key` in config) · `v` / `Shift+V` cycle |
| Template manager | `Shift+E` |
| Closed search persist | automatic (`~/.config/tick/cache/closed_prefs.json`) |
| Filter Closed results | `f` (local, after fetch) |
| Custom columns | `columns = [..., "customfield_10042"]` |

---

## Saved JQL views (`[[views.custom]]`)

Define extra tabs beyond the built-in six. Each view runs its own JQL (and optional single-site filter).

```toml
[[views.custom]]
name = "My bugs"
jql = "project = HIN AND assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC"
key = 7

[[views.custom]]
name = "Team backlog"
jql = "project = HIN AND status = Backlog ORDER BY rank"
site = "zeta"          # only query this [[sites]].name
key = 8
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Tab label in the header |
| `jql` | yes | Full JQL (include `ORDER BY` if you care about order) |
| `site` | no | Limit fetch to one configured site |
| `key` | no | Tab key `7`, `8`, or `9` (defaults to 7, 8, 9 for first three views) |

### Switching views

- Press the configured digit (**`7`**, **`8`**, **`9`**) from the table.
- **`v`** — next custom view (wraps).
- **`Shift+V`** — previous custom view.
- Built-in tabs **`1`–`6`** clear the custom view and work as before.

### Cache

Custom views cache to:

`~/.config/tick/cache/custom-<slug>.json`

where `<slug>` is derived from the view `name`. **`r`** refreshes the active custom view.

### Example workflow

```text
7                             # open "My bugs" custom tab (first time: fetches Jira)
/ regression                  # local filter within results (same as tab 1–5)
v                             # cycle to next custom view
r                             # refresh current custom view
1                             # back to Assigned (built-in)
```

---

## Template manager (`Shift+E`)

Edit or delete templates already in config (including those merged from `create.templates_file`).

### Flow

1. **`Shift+E`** — list all `[[create.templates]]` names.
2. **`j` / `k`** — select template.
3. **`Enter`** — action menu:
   - **`e`** — edit summary (footer prompt)
   - **`p`** — edit project key
   - **`i`** — edit issue type name
   - **`b`** — edit description (markdown, footer; empty allowed)
   - **`l`** — edit labels (comma-separated, e.g. `bug, triage`; empty clears)
   - **`d`** — delete (confirm with **`Enter`**)
4. Changes are written to **`create.templates_file`** if set, otherwise inline blocks in **`config.toml`**.

**Note:** Export new templates with **`X`** or `tick template export`; the manager is for maintaining existing entries.

### Example

```text
Shift+E
j                             # highlight "hin-bug"
Enter
e                             # edit summary
Bug: SSO timeout on login
Enter                           # saved to templates/local.toml
```

---

## Closed tab — persist and local filter

### Persisted state

Last Closed search is restored on startup:

| File | Content |
|------|---------|
| `~/.config/tick/cache/closed_prefs.json` | `query` text and `ever_assigned` (`h` toggle) |

Saved when you run a search (`/` → **`Enter`**) or toggle **`h`**.

### Keys on Closed tab

| Key | Action |
|-----|--------|
| `/` | Edit JQL search words → **`Enter`** fetches from Jira |
| `f` | **Local filter** on fetched rows (key, summary, labels, custom columns, …) |
| `h` | Toggle assignee-when-done vs ever-assigned; re-fetches if query set |
| `r` | Re-run last JQL search |

**`/``** on Closed always edits the **server** search string. **`f`** filters the **current table** without calling Jira (useful for narrowing a large result set).

### Example

```text
6                             # Closed tab
/ payment refund
Enter                         # JQL fetch; prefs saved
f gateway                     # narrow to rows matching "gateway" locally
h                             # switch to assignee was currentUser(); re-fetch
```

---

## Custom field columns (read-only)

Add Jira field ids to `columns`. Values are fetched on bulk load and shown read-only in the table.

```toml
columns = [
  "site",
  "key",
  "summary",
  "status",
  "customfield_10042",   # e.g. Team or Story Points — discover via tick --doctor
]
```

- Header shows a short label (`CF10042` for `customfield_10042`).
- Objects/arrays are flattened to display names where possible.
- Local filter (`/` or Closed **`f`**) includes custom field text.

Find field ids with **`tick --doctor`** (sprint field listing) or Jira **Settings → Issues → Custom fields**.

---

## Related

- [create-duplicate-templates.md](create-duplicate-templates.md) — `N`, `X`, CLI export
- [views-and-tabs.md](views-and-tabs.md) — built-in tabs `1`–`6`
- [table-and-navigation.md](table-and-navigation.md) — filter and sort
- [CONFIGURATION.md](../CONFIGURATION.md)
