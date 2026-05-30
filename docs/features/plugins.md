# Plugins (track C)

Optional **Lua** extensions in `~/.config/tick/plugins/`.

| Capability | Version | Description |
|--------------|---------|-------------|
| `filter_tickets` | v0.21 | Filter rows after each fetch |
| `on_key` | v0.22 | Custom chords while the table is idle |
| `run_transition` | v0.23 | `tick.run_transition` / `tick.list_transitions` (no required-field modals) |

## Install

```text
~/.config/tick/plugins/
  hide-epics/
    tick.plugin.toml
    main.lua
```

```bash
mkdir -p ~/.config/tick/plugins
cp -R examples/plugins/hide-epics ~/.config/tick/plugins/
cp -R examples/plugins/count-visible ~/.config/tick/plugins/
```

## Manifest (`tick.plugin.toml`)

```toml
name = "hide-epics"
version = "0.1.0"
api = "1"          # must match tick's plugin API (see tick --doctor)
runtime = "lua"
entry = "main.lua"

[capabilities]
filter_tickets = true
on_key = ["ctrl+shift+c"]
run_transition = false
```

Enable at least one capability. Chords use `ctrl`, `shift`, `alt`, `super`, and a key (`h`, `space`, `f1`, …).

## Reload

Plugins load at **startup**. Press **`R`** in the TUI to reload `config.toml` and **re-scan** `~/.config/tick/plugins/` (new scripts, manifest edits, and removals take effect without restarting tick).

## Multiple plugins

| Behavior | Rule |
|----------|------|
| **`filter_tickets`** | **Pipeline** — plugins run in **subdirectory name order** (lexical). Each plugin receives the previous plugin's output. |
| **`on_key`** | **First handler wins** — plugins run in load order; the first that returns `"handled"` stops the chain. |
| **Return shape** | Filters must return rows whose `key` + `site` still exist in the input list (unknown rows are dropped). |

Example: `alpha/` then `beta/` → `alpha` filters first, then `beta`.

## Lua API (`tick` table)

Available during `on_key` (and transition helpers when enabled):

| Field / function | Description |
|------------------|-------------|
| `tick.version` | Plugin API version string (`"1"`) |
| `tick.view` | `{ name, mode }` — active view |
| `tick.tickets` | Filtered table rows (same fields as `filter_tickets`) |
| `tick.selected` | `{ key, site }` or `nil` — highlighted row |
| `tick._notice` | Set a footer message from `on_key` (string) |

With `run_transition = true` in the manifest:

| Function | Description |
|----------|-------------|
| `tick.list_transitions(key)` | `{ { id, name, to_status }, ... }` |
| `tick.run_transition(key, transition_id)` | `{ ok = true }` or `{ ok = false, error = "..." }` |

## `filter_tickets`

```lua
function filter_tickets(tickets)
  -- array of { key, site, summary, status, priority, assignee, issue_type, labels, url }
  return tickets
end
```

Called **after each Jira fetch** and when loading a cached view. Runs **after** tick's built-in filters and **before** render.

## `on_key`

```lua
function on_key(chord)
  -- chord matches manifest, e.g. "ctrl+shift+c"
  local sel = tick.selected
  if sel then
    tick._notice = sel.key .. " on " .. sel.site
  end
  return "handled"   -- or "passthrough"
end
```

Called only for chords listed in the manifest, when the table is idle (no modal, footer input, or local `/` filter).

## `run_transition` (capability)

Requires `run_transition = true` in the manifest. Uses the same transition rules as the CLI: transitions that need extra fields fail with an error (use **`t`** in the TUI for those). On success, tick refreshes the active view.

Example: `examples/plugins/list-transitions/` (**Ctrl+Shift+T** shows transition ids in the footer).

## Limits

- **Timeout:** 50 ms per plugin call
- **Sandbox:** no `io` / `os` / network
- **Trust:** local code you install — not a marketplace
- **Detail pane:** read-only ADF/description is **not** exposed in v1 (table + selection only)

## Doctor

```bash
tick --doctor
```

Example output:

```text
--- Plugins ---
  Plugins dir: /home/you/.config/tick/plugins
  Plugin API supported: 1
  Reload: press R in the TUI (re-scans plugins dir)
  Filter pipeline (directory order): hide-epics → my-filter
  Loaded: hide-epics v0.1.0 (filter_tickets)
  Loaded: count-visible v0.1.0 (on_key [ctrl+shift+c])
  Skipped: draft/: no tick.plugin.toml
```

Fix `Error:` lines before relying on a plugin. `Skipped:` is informational (folders without a manifest).

## Related

- [plugin-rfc.md](../architecture/plugin-rfc.md) — design decisions and security model
- [automation.md](automation.md) — shell hooks vs plugins
