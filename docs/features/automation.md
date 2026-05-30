# Automation (CLI)

Headless commands use the same `config.toml` and auth as the TUI.

## `tick issue show`

Print core issue fields as JSON:

```bash
tick issue show HIN-123 --site zeta
```

When only one `[[sites]]` entry exists, `--site` is optional.

Output fields: `key`, `site`, `summary`, `status`, `priority`, `assignee`, `labels`, `url`.

## `tick issue transition`

Apply a workflow transition **by name** (same matching as bulk transition in the TUI):

```bash
tick issue transition HIN-123 --to "In Progress" --site zeta
```

Exits with code `1` on failure (e.g. transition requires extra fields — use the TUI for those).

## Config validation

```bash
tick --check    # offline structural validation
tick --doctor   # live Jira API probes per site
```

## Planned (v0.14+)

- `tick search --jql '...'`
- `tick bulk` (transition, assign, labels from the shell)

See [bulk-actions.md](bulk-actions.md) for interactive bulk operations today.
