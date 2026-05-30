# Automation (CLI)

Headless commands use the same `config.toml` and auth as the TUI.

## `tick issue show`

```bash
tick issue show HIN-123 --site zeta
```

Prints JSON: key, summary, status, assignee, labels, url.

## `tick issue transition`

```bash
tick issue transition HIN-123 --to "In Progress" --site zeta
```

Applies a workflow transition **by name**. Fails if the transition requires extra fields (use the TUI for those).

## `tick search`

```bash
tick search --jql 'project = HIN AND assignee = currentUser() ORDER BY updated DESC' --site zeta
```

Prints JSON: `{ "issues": [...], "warnings": [...] }`. Respects `max_results` in config.

## `tick bulk`

All bulk commands require `--site` and `--keys` (comma-separated or repeated).

```bash
tick bulk transition --site zeta --keys HIN-1,HIN-2 --to Done
tick bulk assign --site zeta --keys HIN-1,HIN-2 --me
tick bulk labels --site zeta --keys HIN-1 --set "bug,triage"
```

Output is JSON with `ok` and `failed` arrays. Exit code `1` if any issue fails. Use `--quiet` for compact JSON on stdout only.

## Config validation

```bash
tick --check    # offline structural validation
tick --doctor   # live Jira API probes
```

## Refresh hooks

After a **successful** refresh of the active view (`r` or background update), tick can run shell commands:

```toml
[[hooks.on_refresh]]
command = "~/.local/bin/on-tick-refresh.sh"
views = ["assigned", "mentions"]   # optional; default = all views
timeout_secs = 30                  # optional; default 30
```

| Variable | Content |
|----------|---------|
| `TICK_VIEW` | View id: `assigned`, `mentions`, `watched`, `updated`, `sprint`, `closed`, or custom view **name** |
| `TICK_JSON_PATH` | Path to a temp JSON file (array of issues) |
| `TICK_ISSUE_COUNT` | Number of issues |

Hooks run in the background. Failures print to stderr as `[tick hook] …`. Not run when the fetch returns site errors.

Details: [CONFIGURATION.md](../CONFIGURATION.md#refresh-hooks).
