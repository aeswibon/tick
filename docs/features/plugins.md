# Plugins (track C)

Optional **Lua** extensions in `~/.config/tick/plugins/`.

| Capability | Version | Description |
|--------------|---------|-------------|
| `filter_tickets` | v0.21 | Filter rows after each fetch |
| `on_key` | v0.22 | Custom chords while the table is idle |

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
api = "1"          # must match tick's plugin API
runtime = "lua"
entry = "main.lua"

[capabilities]
filter_tickets = true
on_key = ["ctrl+shift+c"]
```

Enable at least one capability. Chords use `ctrl`, `shift`, `alt`, `super`, and a key (`h`, `space`, `f1`, …).

## `filter_tickets`

```lua
function filter_tickets(tickets)
  -- array of { key, site, summary, status, priority, assignee, issue_type, labels, url }
  return tickets
end
```

Called **after each Jira fetch** and when loading a cached view. Plugins run in **directory name order**.

## `on_key`

```lua
function on_key(chord)
  -- chord matches manifest, e.g. "ctrl+shift+c"
  -- tick.version, tick.view { name, mode }, tick.tickets (filtered rows)
  tick._notice = "optional footer message"
  return "handled"   -- or "passthrough"
end
```

Called only for chords listed in the manifest, when the table is idle (no modal, footer input, or local `/` filter). Plugins run in directory order until one returns `"handled"`.

## Limits

- **Timeout:** 50 ms per plugin call
- **Sandbox:** no `io` / `os` / network
- **Trust:** local code you install — not a marketplace

## Doctor

```bash
tick --doctor
```

## Related

- [plugin-rfc.md](../architecture/plugin-rfc.md) — `run_transition` (planned)
- [automation.md](automation.md) — shell hooks vs plugins
