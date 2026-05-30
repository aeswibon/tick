# tick plugins (examples)

Plugins are Lua scripts under `~/.config/tick/plugins/<name>/` with a `tick.plugin.toml` manifest.

## hide-epics

Removes issues whose `issue_type` is `Epic` from the table after each view refresh.

## count-visible

**Ctrl+Shift+C** — footer notice with the active view name and filtered row count (demonstrates `on_key` + `tick.tickets`).

## list-transitions

**Ctrl+Shift+T** — footer lists Jira transition ids/names for the selected issue (`tick.list_transitions`). Pair with `tick.run_transition(key, id)` in your own plugin when you know the id.

```bash
mkdir -p ~/.config/tick/plugins
cp -R examples/plugins/hide-epics ~/.config/tick/plugins/
tick --doctor   # should list "Loaded: hide-epics"
```

Plugins load at startup; **`R`** re-scans `~/.config/tick/plugins/` and reloads Lua scripts.

Multiple filters run as a **pipeline** (subdirectory name order). `tick --doctor` shows load order, capabilities, and errors.

See [docs/features/plugins.md](../../docs/features/plugins.md) and [docs/architecture/plugin-rfc.md](../../docs/architecture/plugin-rfc.md).
