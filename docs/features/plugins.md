# Plugins (track C.1)

Optional **Lua** extensions in `~/.config/tick/plugins/`. v0.21 ships **`filter_tickets` only** — no custom keys or Jira writes from plugins yet.

## Install

```text
~/.config/tick/plugins/
  hide-epics/
    tick.plugin.toml
    main.lua
```

Copy the example:

```bash
mkdir -p ~/.config/tick/plugins
cp -R examples/plugins/hide-epics ~/.config/tick/plugins/
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
```

## `filter_tickets`

Define a global function in `main.lua`:

```lua
function filter_tickets(tickets)
  -- tickets: array of { key, site, summary, status, priority, assignee, issue_type, labels, url }
  return tickets  -- return filtered array (same shape)
end
```

tick calls this **after each Jira fetch** (and when loading a cached view), before the table is shown. Plugins run in **subdirectory name order**; each receives the previous plugin's output.

- **Timeout:** 50 ms per plugin call; failure shows a footer notice and keeps the last good list.
- **Sandbox:** Lua without `io` / `os` / network; only the `filter_tickets` entrypoint is invoked.
- **Trust:** plugins are local code you install — not a marketplace.

## Doctor

```bash
tick --doctor
```

Lists the plugins directory, loaded filters, and load errors.

## Related

- [plugin-rfc.md](../architecture/plugin-rfc.md) — full track C plan (`on_key`, transitions)
- [automation.md](automation.md) — shell hooks vs plugins
